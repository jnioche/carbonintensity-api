//! API for retrieving data from the Carbon Intensity API
//! <https://api.carbonintensity.org.uk/>

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use futures::future;
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

mod region;
mod target;

pub use region::Region;
pub use target::Target;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationMix {
    fuel: String,
    perc: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Intensity {
    forecast: i32,
    index: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    from: String,
    to: String,
    intensity: Intensity,
    generationmix: Vec<GenerationMix>,
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

static BASE_URL: &str = "https://api.carbonintensity.org.uk";

/// Current carbon intensity for a target (e.g. a region)
///
/// Uses either
/// - <https://api.carbonintensity.org.uk/regional/postcode/>
/// - <https://api.carbonintensity.org.uk/regional/regionid/>
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
    };

    let url = format!("{BASE_URL}/{path}");
    get_intensity_for_url(&url).await
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
        Some(end_date) => {
            let end_date = parse_date(end_date)?;
            // check that the date is not in the future - otherwise set it to now
            if now.and_utc().timestamp() < end_date.and_utc().timestamp() {
                now
            } else {
                end_date
            }
        }
    };

    //  split into ranges
    let mut ranges = Vec::new();

    let duration = Duration::days(13);
    let mut current = start_date;
    loop {
        let mut next_end = current + duration;
        // break the end of year boundary
        let new_year_d = NaiveDate::from_ymd_opt(current.year() + 1, 1, 1).unwrap();
        let new_year = NaiveDateTime::new(new_year_d, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        if next_end >= new_year {
            next_end = new_year;
        }
        if next_end >= end_date {
            ranges.push((current, end_date));
            break;
        } else {
            ranges.push((current, next_end));
        }

        current = next_end;
    }
    Ok(ranges)
}

/// Get intensities for a given target (region or postcode) in 30 minutes windows
///
/// Dates are strings in ISO-8601 format YYYY-MM-DDThh:mmZ
/// but YYYY-MM-DD is tolerated
///
/// Uses either
/// - https://api.carbonintensity.org.uk/regional/intensity/2023-05-15/2023-05-20/postcode/RG10
/// - https://api.carbonintensity.org.uk/regional/intensity/2023-05-15/2023-05-20/regionid/13
pub async fn get_intensities(
    target: &Target,
    start: &str,
    end: &Option<&str>,
) -> Result<Vec<IntensityForDate>> {
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

            let url = format!("{BASE_URL}/regional/intensity/{start_date}/{end_date}/{path}");

            tokio::spawn(async move {
                let region_data = get_intensities_for_url(&url).await?;
                to_tuples(region_data.data)
            })
        })
        .collect();

    let tasks_results = future::try_join_all(tasks).await?;
    tasks_results
        .into_iter()
        .collect::<Result<Vec<_>>>() // convert to single Result
        .map(|nested_tuples| nested_tuples.into_iter().flatten().collect())
}

/// converts the values from JSON into a simpler
/// representation Vec<DateTime, float>
fn to_tuples(data: Vec<Data>) -> Result<Vec<IntensityForDate>> {
    data.into_iter()
        .map(|datum| {
            let start_date = parse_date(&datum.from)?;
            let intensity = datum.intensity.forecast;
            Ok((start_date, intensity))
        })
        .collect()
}

async fn get_intensities_for_url(url: &str) -> Result<RegionData> {
    let PowerData { data } = get_response(url).await?;
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

// Internal method to handle the querying and parsing
async fn get_instant_data(url: &str) -> Result<Root> {
    get_response::<Root>(url).await
}

/// Makes a GET request to the given URL
///
/// Deserialize the JSON response as `T` and returns Ok<T> if all is well.
/// Returns an `ApiError` when the HTTP request failed or the response body
/// couldn't be deserialized as a `T` value.
async fn get_response<T>(url: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let client = Client::new();
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

    use super::*;

    impl Data {
        fn test_data(from: &str, to: &str, intensity: i32) -> Self {
            Self {
                from: from.to_string(),
                to: to.to_string(),
                intensity: Intensity {
                    forecast: intensity,
                    index: "very high".to_string(),
                },
                generationmix: vec![
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
                ],
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
        assert!(result.is_err());
        match result.err().unwrap() {
            ApiError::DateParseError(_err) => {} // success,
            err => panic!("Expected a ApiError::DateParseError, got {:?}", err),
        };

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
    fn deserialize_power_data_test() {
        let json_str = r#"
        {"data":{"regionid":11,"shortname":"South West England","postcode":"BS7","data":[{"from":"2022-12-31T23:30Z","to":"2023-01-01T00:00Z","intensity":{"forecast":152,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.3},{"fuel":"imports","perc":14.3},{"fuel":"gas","perc":28.5},{"fuel":"nuclear","perc":7},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.5},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.1}]},{"from":"2023-01-01T00:00Z","to":"2023-01-01T00:30Z","intensity":{"forecast":181,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":9.1},{"fuel":"gas","perc":36.1},{"fuel":"nuclear","perc":6.8},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":42.8}]},{"from":"2023-01-01T00:30Z","to":"2023-01-01T01:00Z","intensity":{"forecast":189,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.3},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":12.1},{"fuel":"gas","perc":37.6},{"fuel":"nuclear","perc":6.4},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":38.8}]},{"from":"2023-01-01T01:00Z","to":"2023-01-01T01:30Z","intensity":{"forecast":183,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.7},{"fuel":"coal","perc":3.2},{"fuel":"imports","perc":6.1},{"fuel":"gas","perc":37.3},{"fuel":"nuclear","perc":7.3},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":44}]},{"from":"2023-01-01T01:30Z","to":"2023-01-01T02:00Z","intensity":{"forecast":175,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.5},{"fuel":"coal","perc":2.9},{"fuel":"imports","perc":6.6},{"fuel":"gas","perc":36},{"fuel":"nuclear","perc":7.2},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.5}]}]}}
    "#;

        let _result: std::result::Result<PowerData, serde_json::Error> =
            serde_json::from_str(json_str);
    }

    #[test]
    fn range_splitting() {
        let periods = normalise_dates("2022-12-01", &Option::Some("2023-02-01"));
        println!("{:?}", periods);
    }
}
