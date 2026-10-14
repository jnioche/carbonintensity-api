//! API for retrieving data from the Carbon Intensity API
//! <https://api.carbonintensity.org.uk/>

use futures::future;
use std::sync::LazyLock;

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

mod region;
mod target;

pub use region::Region;
pub use target::Target;

// oldest entry available for 2018-05-10 23:30:00
static OLDEST_VALID_DATE: LazyLock<NaiveDateTime> = LazyLock::new(|| {
    NaiveDate::from_ymd_opt(2018, 5, 10)
        .unwrap()
        .and_hms_opt(23, 30, 0)
        .unwrap()
});

/// An error communicating with the Carbon Intensity API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// There was an error making the HTTP request.
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    /// A REST API method returned an error status.
    #[error("REST error {status}: {body}")]
    RestError { status: StatusCode, body: String },
    /// There was an error parsing a URL from a string.
    #[error("Error parsing URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Error parsing date: {0}")]
    DateParseError(#[from] chrono::ParseError),
    #[error("Error executing concurrent task: {0}")]
    ConcurrentTaskFailedError(#[from] tokio::task::JoinError),
    #[error("Error: {0}")]
    Error(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;

pub type IntensityForDate = (NaiveDateTime, i32);

/// Fuel types reported by the API in the generation mix
pub const FUELS: [&str; 9] = [
    "biomass", "coal", "imports", "gas", "nuclear", "other", "hydro", "solar", "wind",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationMix {
    pub fuel: String,
    pub perc: f64,
}

/// Intensity for a 30 minutes window, along with the generation mix behind it
#[derive(Debug, Clone, PartialEq)]
pub struct IntensityWithMix {
    pub from: NaiveDateTime,
    pub intensity: i32,
    pub mix: Vec<GenerationMix>,
}

impl IntensityWithMix {
    /// Percentage for a given fuel or None if the fuel is not in the mix
    pub fn perc(&self, fuel: &str) -> Option<f64> {
        self.mix.iter().find(|m| m.fuel == fuel).map(|m| m.perc)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Intensity {
    forecast: i32,
    index: String,
    actual: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    from: String,
    to: String,
    intensity: Intensity,
    generationmix: Option<Vec<GenerationMix>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct RegionData {
    regionid: i32,
    dnoregion: Option<String>,
    shortname: String,
    postcode: Option<String>,
    data: Vec<Data>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Root {
    data: Vec<RegionData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PowerData {
    data: RegionData,
}

#[derive(Debug, Serialize, Deserialize)]
struct NationalData {
    data: Vec<Data>,
}

static BASE_URL: &str = "https://api.carbonintensity.org.uk";

/// Current carbon intensity for a target (e.g. a region)
///
/// Uses one of
/// - <https://api.carbonintensity.org.uk/regional/postcode/>
/// - <https://api.carbonintensity.org.uk/regional/regionid/>
/// - <https://api.carbonintensity.org.uk/intensity>
pub async fn get_intensity(target: &Target) -> Result<i32> {
    let path = match target {
        Target::Postcode(postcode) => {
            if postcode.len() < 2 || postcode.len() > 4 {
                return Err(ApiError::Error("Invalid postcode".to_string()));
            }
            format!("regional/postcode/{postcode}")
        }
        &Target::Region(region) => {
            let region_id = region as u8;
            format!("regional/regionid/{region_id}")
        }
        &Target::National => "intensity".to_string(),
    };

    let url = format!("{BASE_URL}/{path}");
    if *target != Target::National {
        get_intensity_for_url(&url).await
    } else {
        get_intensity_for_url_national(&url).await
    }
}

fn parse_date(date: &str) -> std::result::Result<NaiveDateTime, chrono::ParseError> {
    if let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    // try the longest form or fail
    NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%MZ")
}

/// Normalises the start and end dates
/// returns ranges that are acceptable by the API
/// both in their duration and string representation
fn normalise_dates(start: &str, end: &Option<&str>) -> Result<Vec<(NaiveDateTime, NaiveDateTime)>> {
    let start_date = parse_date(start)?;

    let now = Local::now().naive_local();

    // if the end is not set - use now
    let end_date = match end {
        None => now,
        Some(end_date) => parse_date(end_date)?,
    };

    let start_date = validate_date(start_date);
    let end_date = validate_date(end_date);

    //  split into ranges
    let mut ranges = Vec::new();

    let duration = Duration::days(13);
    let mut current = start_date;
    loop {
        let next_end = current + duration;

        let new_year_d = NaiveDate::from_ymd_opt(current.year() + 1, 1, 1).unwrap();
        let new_year = NaiveDateTime::new(new_year_d, NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        if next_end >= new_year {
            // The API silently truncates cross-year requests at Dec 31 23:00 (the slot
            // from=23:30 whose 'to' is Jan 1 is never returned by any request — API
            // limitation). Avoid cross-year URLs by splitting 30 min before year end,
            // then restarting cleanly from Jan 1.
            if end_date < new_year {
                ranges.push((current, end_date));
                break;
            }
            let chunk_end = new_year - Duration::minutes(30);
            if chunk_end > current {
                ranges.push((current, chunk_end));
            }
            current = new_year;
            if current >= end_date {
                break;
            }
            continue;
        }

        if next_end >= end_date {
            ranges.push((current, end_date));
            break;
        }

        ranges.push((current, next_end));
        current = next_end;
    }
    Ok(ranges)
}

/// Get intensities for a given target (region or postcode) in 30 minutes windows
///
/// Dates are strings in ISO-8601 format YYYY-MM-DDThh:mmZ
/// but YYYY-MM-DD is tolerated
///
/// Uses one of
/// - https://api.carbonintensity.org.uk/regional/intensity/2023-05-15/2023-05-20/postcode/RG10
/// - https://api.carbonintensity.org.uk/regional/intensity/2023-05-15/2023-05-20/regionid/13
/// - https://api.carbonintensity.org.uk/intensity/2023-05-15/2023-05-20/
pub async fn get_intensities(
    target: &Target,
    start: &str,
    end: &Option<&str>,
) -> Result<Vec<IntensityForDate>> {
    get_data_for_ranges(target, start, end, to_tuples).await
}

/// Get intensities along with their generation mix for a given target (region or postcode)
/// in 30 minutes windows
///
/// Dates are strings in ISO-8601 format YYYY-MM-DDThh:mmZ
/// but YYYY-MM-DD is tolerated
///
/// The generation mix is only returned by the regional endpoints,
/// an error is returned if the target is National.
pub async fn get_intensities_with_mix(
    target: &Target,
    start: &str,
    end: &Option<&str>,
) -> Result<Vec<IntensityWithMix>> {
    if *target == Target::National {
        return Err(ApiError::Error(
            "Generation mix is not available at the national level".to_string(),
        ));
    }
    get_data_for_ranges(target, start, end, to_intensities_with_mix).await
}

/// Splits the dates into ranges, queries them concurrently
/// and converts the results with `convert`
async fn get_data_for_ranges<T>(
    target: &Target,
    start: &str,
    end: &Option<&str>,
    convert: fn(Vec<Data>) -> Result<Vec<T>>,
) -> Result<Vec<T>>
where
    T: Send + 'static,
{
    let path = match target {
        Target::Postcode(postcode) => {
            if postcode.len() < 2 || postcode.len() > 4 {
                return Err(ApiError::Error("Invalid postcode".to_string()));
            }

            format!("postcode/{postcode}")
        }
        &Target::Region(region) => {
            let region_id = region as u8;
            format!("regionid/{region_id}")
        }
        &Target::National => "intensity".to_string(),
    };

    let ranges = normalise_dates(start, end)?;

    // Spawns concurrent tasks...
    let tasks: Vec<_> = ranges
        .into_iter()
        .map(|(start_date, end_date)| {
            // shift dates by one minute
            let start_date = start_date + Duration::minutes(1);
            let end_date = end_date + Duration::minutes(1);
            // format dates
            let start_date = start_date.format("%Y-%m-%dT%H:%MZ");
            let end_date = end_date.format("%Y-%m-%dT%H:%MZ");

            if *target != Target::National {
                let url = format!("{BASE_URL}/regional/intensity/{start_date}/{end_date}/{path}");

                tokio::spawn(async move {
                    let region_data = get_intensities_for_url(&url).await?;
                    convert(region_data.data)
                })
            } else {
                let url = format!("{BASE_URL}/{path}/{start_date}/{end_date}/");

                tokio::spawn(async move {
                    let national_data = get_intensities_for_url_national(&url).await?;
                    convert(national_data.data)
                })
            }
        })
        .collect();

    let tasks_results = future::try_join_all(tasks).await?;
    tasks_results
        .into_iter()
        .collect::<Result<Vec<_>>>() // convert to single Result
        .map(|nested| nested.into_iter().flatten().collect())
}

/// converts the values from JSON into a simpler
/// representation Vec<DateTime, float>
fn to_tuples(data: Vec<Data>) -> Result<Vec<IntensityForDate>> {
    data.into_iter()
        .map(|datum| {
            let start_date = parse_date(&datum.from)?;
            let intensity = datum.intensity.actual.unwrap_or(datum.intensity.forecast);
            Ok((start_date, intensity))
        })
        .collect()
}

/// converts the values from JSON into IntensityWithMix
fn to_intensities_with_mix(data: Vec<Data>) -> Result<Vec<IntensityWithMix>> {
    data.into_iter()
        .map(|datum| {
            let from = parse_date(&datum.from)?;
            let intensity = datum.intensity.actual.unwrap_or(datum.intensity.forecast);
            let mix = datum.generationmix.unwrap_or_default();
            Ok(IntensityWithMix {
                from,
                intensity,
                mix,
            })
        })
        .collect()
}

/// Returns a date within a valid date
///
/// Datetimes older than 2018-05-10 23:30:00 are invalid.
/// Also, datetimes in the future are invalid.
///
/// - if a datetime is too old, returns the oldest valid date
/// - if a datetime is in the future, returns now
/// - otherwise returns the input datetime
fn validate_date(date: NaiveDateTime) -> NaiveDateTime {
    let now = Local::now().naive_local();

    // check if date is too old
    if date < *OLDEST_VALID_DATE {
        return *OLDEST_VALID_DATE;
    }
    // check that the date is not in the future
    if date > now {
        return now;
    }

    date
}

async fn get_intensities_for_url(url: &str) -> Result<RegionData> {
    let PowerData { data } = get_response(url).await?;
    Ok(data)
}

async fn get_intensities_for_url_national(url: &str) -> Result<NationalData> {
    let data = get_response::<NationalData>(url).await?;
    Ok(data)
}

/// Retrieves the intensity value from a structure
async fn get_intensity_for_url(url: &str) -> Result<i32> {
    let result = get_instant_data(url).await?;

    let intensity = result
        .data
        .first()
        .ok_or_else(|| ApiError::Error("No data found".to_string()))?
        .data
        .first()
        .ok_or_else(|| ApiError::Error("No intensity data found".to_string()))?
        .intensity
        .forecast;

    Ok(intensity)
}

/// Retrieves the intensity value from a structure
async fn get_intensity_for_url_national(url: &str) -> Result<i32> {
    let result = get_response::<NationalData>(url).await?;

    let intensity = result
        .data
        .first()
        .ok_or_else(|| ApiError::Error("No data found".to_string()))?
        .intensity
        .actual
        .unwrap();

    Ok(intensity)
}

// Internal method to handle the querying and parsing
async fn get_instant_data(url: &str) -> Result<Root> {
    get_response::<Root>(url).await
}

/// Makes a GET request to the given URL
///
/// Deserialise the JSON response as `T` and returns Ok<T> if all is well.
/// Returns an `ApiError` when the HTTP request failed or the response body
/// couldn't be deserialised as a `T` value.
async fn get_response<T>(url: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let client = Client::new();
    #[cfg(debug_assertions)]
    eprintln!("GET {url}");
    let response = client.get(url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        return Err(ApiError::RestError { status, body });
    }

    let target = response.json::<T>().await?;
    Ok(target)
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use chrono::{Days, Months, SubsecRound};

    use super::*;

    impl Data {
        fn test_data(from: &str, to: &str, intensity: i32) -> Self {
            Self {
                from: from.to_string(),
                to: to.to_string(),
                intensity: Intensity {
                    forecast: intensity,
                    index: "very high".to_string(),
                    actual: None,
                },
                generationmix: Option::from(vec![
                    GenerationMix {
                        fuel: "gas".to_string(),
                        perc: 80.0,
                    },
                    GenerationMix {
                        fuel: "wind".to_string(),
                        perc: 10.0,
                    },
                    GenerationMix {
                        fuel: "other".to_string(),
                        perc: 10.0,
                    },
                ]),
            }
        }
    }

    /// Returns a NaiveDateTime from a string slice. Assumes input is valid
    fn test_date_time(date: &str) -> NaiveDateTime {
        NaiveDate::from_str(date)
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    }

    #[test]
    fn to_tuples_test() {
        // One of the dates is invalid
        let data = vec![
            Data::test_data("2024-01-01", "2024-02-01", 350),
            Data::test_data("Invalid", "2024-03-01", 300),
            Data::test_data("2024-03-01", "2024-04-01", 250),
        ];
        let result = to_tuples(data);
        assert!(matches!(result, Err(ApiError::DateParseError(_))));

        // Happy path
        let data = vec![
            Data::test_data("2024-01-01", "2024-02-01", 350),
            Data::test_data("2024-02-01", "2024-03-01", 300),
        ];
        let result = to_tuples(data);

        let jan = test_date_time("2024-01-01");
        let feb = test_date_time("2024-02-01");
        let expected = vec![(jan, 350), (feb, 300)];

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn to_intensities_with_mix_test() {
        let data = vec![
            Data::test_data("2024-01-01", "2024-02-01", 350),
            Data::test_data("2024-02-01", "2024-03-01", 300),
        ];
        let result = to_intensities_with_mix(data).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].from, test_date_time("2024-01-01"));
        assert_eq!(result[0].intensity, 350);
        assert_eq!(result[1].intensity, 300);
        assert_eq!(result[0].perc("gas"), Some(80.0));
        assert_eq!(result[0].perc("wind"), Some(10.0));
        assert_eq!(result[0].perc("solar"), None);

        let data = vec![Data::test_data("Invalid", "2024-03-01", 300)];
        let result = to_intensities_with_mix(data);
        assert!(matches!(result, Err(ApiError::DateParseError(_))));
    }

    #[test]
    fn deserialise_power_data_test() {
        let json_str = r#"
        {"data":{"regionid":11,"shortname":"South West England","postcode":"BS7","data":[{"from":"2022-12-31T23:30Z","to":"2023-01-01T00:00Z","intensity":{"forecast":152,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.3},{"fuel":"imports","perc":14.3},{"fuel":"gas","perc":28.5},{"fuel":"nuclear","perc":7},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.5},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.1}]},{"from":"2023-01-01T00:00Z","to":"2023-01-01T00:30Z","intensity":{"forecast":181,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":9.1},{"fuel":"gas","perc":36.1},{"fuel":"nuclear","perc":6.8},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":42.8}]},{"from":"2023-01-01T00:30Z","to":"2023-01-01T01:00Z","intensity":{"forecast":189,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.3},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":12.1},{"fuel":"gas","perc":37.6},{"fuel":"nuclear","perc":6.4},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":38.8}]},{"from":"2023-01-01T01:00Z","to":"2023-01-01T01:30Z","intensity":{"forecast":183,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.7},{"fuel":"coal","perc":3.2},{"fuel":"imports","perc":6.1},{"fuel":"gas","perc":37.3},{"fuel":"nuclear","perc":7.3},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":44}]},{"from":"2023-01-01T01:30Z","to":"2023-01-01T02:00Z","intensity":{"forecast":175,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.5},{"fuel":"coal","perc":2.9},{"fuel":"imports","perc":6.6},{"fuel":"gas","perc":36},{"fuel":"nuclear","perc":7.2},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.5}]}]}}
    "#;

        let result: PowerData = serde_json::from_str(json_str).unwrap();
        let result = to_intensities_with_mix(result.data.data).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].intensity, 152);
        assert_eq!(result[0].perc("wind"), Some(45.1));
        // all fuels returned by the API are known
        for fuel in result[0].mix.iter().map(|m| m.fuel.as_str()) {
            assert!(FUELS.contains(&fuel), "unknown fuel {fuel}");
        }
    }

    #[test]
    fn normalise_dates_invalid() {
        // Invalid start date
        let result = normalise_dates("not a date", &None);
        assert!(matches!(result, Err(ApiError::DateParseError(_))));

        // Invalid end date
        let result = normalise_dates("2024-01-01", &Some("not a date"));
        assert!(matches!(result, Err(ApiError::DateParseError(_))));
    }

    #[test]
    fn normalise_dates_too_old() {
        let oldest_valid_date = NaiveDate::from_ymd_opt(2018, 5, 10)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap();

        // Start date too old
        let result = normalise_dates("1111-01-01", &Some("2018-05-15"));
        assert!(result.is_ok());

        let ranges = result.unwrap();
        assert_eq!(ranges.len(), 1);

        let expected = vec![(oldest_valid_date, test_date_time("2018-05-15"))];
        assert_eq!(ranges, expected);
    }

    #[test]
    fn normalise_dates_future() {
        // End date in the future
        let now = Local::now().naive_local();
        let five_days = Days::new(5);
        let five_days_ago = now.checked_sub_days(five_days).unwrap().date();
        let in_five_days = now.checked_add_days(five_days).unwrap().date();

        let result = normalise_dates(&five_days_ago.to_string(), &Some(&in_five_days.to_string()));
        assert!(result.is_ok());

        let ranges = result.unwrap();
        assert_eq!(ranges.len(), 1);

        let (start, end) = ranges[0];
        let expected_start = five_days_ago.and_hms_opt(0, 0, 0).unwrap();
        // start unchanged
        assert_eq!(start, expected_start);
        // end became now because it was in the future
        assert_eq!(end.trunc_subsecs(0), now.trunc_subsecs(0));
    }

    fn year_end_23_30(year: i32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, 12, 31)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap()
    }

    #[test]
    fn normalise_dates_splitting() {
        // Range that ends exactly at Jan 1: last chunk ends at Dec 31 23:30
        // (the Dec 31 23:30 slot whose 'to' is Jan 1 is never accessible from the API)
        let result = normalise_dates("2022-12-01", &Some("2023-01-01"));
        assert!(result.is_ok());
        let ranges = result.unwrap();
        let expected = vec![
            (test_date_time("2022-12-01"), test_date_time("2022-12-14")),
            (test_date_time("2022-12-14"), test_date_time("2022-12-27")),
            (test_date_time("2022-12-27"), year_end_23_30(2022)),
        ];
        assert_eq!(ranges, expected);
    }

    #[test]
    fn normalise_dates_skipping_year() {
        // Ranges spanning 2 years. See: https://github.com/jnioche/carbonintensity-api/issues/6
        // The API silently truncates cross-year requests at Dec 31 23:00; the Dec 31 23:30
        // slot (whose 'to' is Jan 1) is never returned by any request (API limitation).
        // The fix splits 30 min before year end so chunk URLs stay within the current year,
        // then restarts cleanly from Jan 1.
        let result = normalise_dates("2022-12-31", &Some("2023-01-02"));
        assert!(result.is_ok());
        let ranges = result.unwrap();
        let expected = vec![
            (test_date_time("2022-12-31"), year_end_23_30(2022)),
            (test_date_time("2023-01-01"), test_date_time("2023-01-02")),
        ];
        assert_eq!(ranges, expected);
    }

    #[test]
    fn normalise_dates_year_boundary_slot() {
        // Chunk ending before year end stays entirely within the current year (no cross-year
        // truncation). Chunk after boundary starts cleanly at Jan 1.
        // Verified with the live API: URL ending at Dec 31 23:31Z returns 47 Dec 31 slots
        // (00:00 through 23:00); the 23:30 slot is permanently inaccessible.
        let result = normalise_dates("2022-12-30", &Some("2023-01-02"));
        assert!(result.is_ok());
        let ranges = result.unwrap();

        // Last range of current year must end at Dec 31 23:30
        let last_current_year = ranges.iter().filter(|(_, e)| e.year() == 2022).last().unwrap();
        assert_eq!(last_current_year.1, year_end_23_30(2022));

        // First range of new year must start at Jan 1 00:00
        let first_new_year = ranges.iter().find(|(s, _)| s.year() == 2023).unwrap();
        assert_eq!(first_new_year.0, test_date_time("2023-01-01"));
    }

    #[test]
    fn normalise_dates_end_within_current_year() {
        // When end_date is within the current year, no year-boundary split occurs
        let result = normalise_dates("2022-12-28", &Some("2022-12-30"));
        assert!(result.is_ok());
        let ranges = result.unwrap();
        let expected = vec![(test_date_time("2022-12-28"), test_date_time("2022-12-30"))];
        assert_eq!(ranges, expected);
    }

    #[test]
    fn validate_date_test() {
        // valid dates just returned as-is
        let just_a_day = test_date_time("2024-07-30");
        let datetime = validate_date(just_a_day);
        assert_eq!(datetime.trunc_subsecs(0), just_a_day.trunc_subsecs(0));

        // future dates turns into now
        let future = Local::now()
            .naive_local()
            .checked_add_months(Months::new(2))
            .unwrap();
        let datetime = validate_date(future);
        let now = Local::now().naive_local();
        assert_eq!(datetime.trunc_subsecs(0), now.trunc_subsecs(0));

        // oldest is fine
        let oldest_date = NaiveDate::from_ymd_opt(2018, 5, 10)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap();
        let datetime = validate_date(oldest_date);
        assert_eq!(datetime.trunc_subsecs(0), oldest_date.trunc_subsecs(0));

        // just too old - turn into the oldest valid date
        let old = test_date_time("1980-12-31");
        let datetime = validate_date(old);
        assert_eq!(datetime, oldest_date);
    }
}
