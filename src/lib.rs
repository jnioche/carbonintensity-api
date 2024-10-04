//! API for retrieving data from the Carbon Intensity API
//! <https://api.carbonintensity.org.uk/>

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
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
    #[error("Error: {0}")]
    Error(String),
}

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
pub async fn get_intensity(target: &Target) -> Result<i32, ApiError> {
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

fn parse(date: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    if let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    // try the longest form or fail
    NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%MZ")
}

/// Normalises the start and end dates
/// returns ranges that are acceptable by the API
/// both in their duration and string representation
fn normalise_dates(
    start: &str,
    end: &Option<&str>,
) -> Result<Vec<(NaiveDateTime, NaiveDateTime)>, ApiError> {
    let start_date: NaiveDateTime = match parse(start) {
        Ok(res) => res,
        Err(_err) => return Err(ApiError::Error("Invalid start date".to_string() + start)),
    };

    let now = Local::now().naive_local();

    // if the end is not set - use now
    let mut end_date: NaiveDateTime;
    if end.is_none() {
        end_date = now;
    } else {
        // a date exists
        let sd = parse(end.unwrap());
        if sd.is_err() {
            return Err(ApiError::Error(
                "Invalid end date ".to_string() + end.unwrap(),
            ));
        } else {
            end_date = sd.unwrap();
        }
    }

    // check that the date is not in the future - otherwise set it to now
    if now.and_utc().timestamp() < end_date.and_utc().timestamp() {
        end_date = now;
    }

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
        ranges.push((current, end_date));
        if next_end >= end_date {
            break;
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
) -> Result<Vec<(NaiveDateTime, i32)>, ApiError> {
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

    let mut output = Vec::new();

    // TODO query in parallel
    for (start_date, end_date) in ranges {
        // shift dates by one minute
        let start_date = start_date + Duration::minutes(1);
        let end_date = end_date + Duration::minutes(1);
        // format dates
        let start_date = start_date.format("%Y-%m-%dT%H:%MZ");
        let end_date = end_date.format("%Y-%m-%dT%H:%MZ");

        let url = format!("{BASE_URL}/regional/intensity/{start_date}/{end_date}/{path}");
        let region_data = get_intensities_for_url(&url).await?;
        let mut tuples = to_tuple(region_data)?;
        output.append(&mut tuples);
    }
    Ok(output)
}

/// converts the values from JSON into a simpler
/// representation Vec<DateTime, float>
fn to_tuple(data: RegionData) -> Result<Vec<(NaiveDateTime, i32)>, ApiError> {
    let mut values: Vec<(NaiveDateTime, i32)> = Vec::new();
    for d in data.data {
        let start_date = parse(&d.from)?;
        let intensity = d.intensity.forecast;
        values.push((start_date, intensity));
    }
    Ok(values)
}

async fn get_intensities_for_url(url: &str) -> Result<RegionData, ApiError> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();

    if status.is_success() {
        let json_str = response.text().await?;
        if let Ok(PowerData { data }) = serde_json::from_str::<PowerData>(&json_str) {
            Ok(data)
        } else {
            Err(ApiError::Error(format!("Invalid JSON returned {json_str}")))
        }
    } else {
        let body = response.text().await?;
        Err(ApiError::RestError { status, body })
    }
}

/// Retrieves the intensity value from a structure
async fn get_intensity_for_url(url: &str) -> Result<i32, ApiError> {
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
async fn get_instant_data(url: &str) -> Result<Root, ApiError> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();

    if status.is_success() {
        if let Ok(root) = response.json::<Root>().await {
            return Ok(root);
        } else {
            return Err(ApiError::Error("Invalid JSON returned".to_string()));
        }
    }
    // failure
    let body = response.text().await?;
    Err(ApiError::RestError { status, body })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {
        let json_str = r#"
        {"data":{"regionid":11,"shortname":"South West England","postcode":"BS7","data":[{"from":"2022-12-31T23:30Z","to":"2023-01-01T00:00Z","intensity":{"forecast":152,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.3},{"fuel":"imports","perc":14.3},{"fuel":"gas","perc":28.5},{"fuel":"nuclear","perc":7},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.5},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.1}]},{"from":"2023-01-01T00:00Z","to":"2023-01-01T00:30Z","intensity":{"forecast":181,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.4},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":9.1},{"fuel":"gas","perc":36.1},{"fuel":"nuclear","perc":6.8},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":42.8}]},{"from":"2023-01-01T00:30Z","to":"2023-01-01T01:00Z","intensity":{"forecast":189,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.3},{"fuel":"coal","perc":3.4},{"fuel":"imports","perc":12.1},{"fuel":"gas","perc":37.6},{"fuel":"nuclear","perc":6.4},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":38.8}]},{"from":"2023-01-01T01:00Z","to":"2023-01-01T01:30Z","intensity":{"forecast":183,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.7},{"fuel":"coal","perc":3.2},{"fuel":"imports","perc":6.1},{"fuel":"gas","perc":37.3},{"fuel":"nuclear","perc":7.3},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":44}]},{"from":"2023-01-01T01:30Z","to":"2023-01-01T02:00Z","intensity":{"forecast":175,"index":"moderate"},"generationmix":[{"fuel":"biomass","perc":1.5},{"fuel":"coal","perc":2.9},{"fuel":"imports","perc":6.6},{"fuel":"gas","perc":36},{"fuel":"nuclear","perc":7.2},{"fuel":"other","perc":0},{"fuel":"hydro","perc":0.4},{"fuel":"solar","perc":0},{"fuel":"wind","perc":45.5}]}]}}
    "#;

        let result: Result<PowerData, serde_json::Error> = serde_json::from_str(json_str);
        println!("{:?}", result);
    }

    #[test]
    fn range_splitting() {
        let periods = normalise_dates("2022-12-01", &Option::Some("2023-02-01"));
        println!("{:?}", periods);
    }
}
