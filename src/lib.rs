//! API for retrieving data from the Carbon Intensity API
//! <https://api.carbonintensity.org.uk/>

use chrono::{Days, Local, NaiveDate, NaiveDateTime};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::ParseError;

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
    UrlParseError(#[from] ParseError),
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

static BASE_URL: &str = "https://api.carbonintensity.org.uk/";

/// Current carbon intensity for a postcode
///
/// <https://api.carbonintensity.org.uk/regional/postcode/>
///
pub async fn get_intensity_postcode(postcode: &str) -> Result<i32, ApiError> {
    if postcode.len() < 2 || postcode.len() > 4 {
        return Err(ApiError::Error("Invalid postcode".to_string()));
    }

    let path = "regional/postcode/";
    let url = format!("{BASE_URL}{path}{postcode}");
    get_intensity(&url).await
}

/// Current carbon intensity for a region
/// The region id is a number between 1 and 17
///
/// 1 North Scotland
/// 2 South Scotland
/// 3 North West England
/// 4 North East England
/// 5 South Yorkshire
/// 6 North Wales, Merseyside and Cheshire
/// 7 South Wales
/// 8 West Midlands
/// 9 East Midlands
/// 10 East England
/// 11 South West England
/// 12 South England
/// 13 London
/// 14 South East England
/// 15 England
/// 16 Scotland
/// 17 Wales
///
/// <https://api.carbonintensity.org.uk/regional/regionid/>
///
pub async fn get_intensity_region(regionid: u8) -> Result<i32, ApiError> {
    if regionid < 1 || regionid > 17 {
        return Err(ApiError::Error(
            "Invalid regiondid - should be between 1-17".to_string(),
        ));
    }

    let path = "regional/regionid/";
    let url = format!("{BASE_URL}{path}{regionid}");
    get_intensity(&url).await
}

fn parse(date: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    let sd = NaiveDate::parse_from_str(date, "%Y-%m-%d");
    if sd.is_ok() {
        return Ok(sd.unwrap().and_hms_opt(0, 0, 0).unwrap());
    }
    // try the longest form or fail
    NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%MZ")
}

fn normalise_dates(start: &str, end: &Option<&str>) -> Result<String, ApiError> {
    let start_date: NaiveDateTime = match parse(start) {
        Ok(res) => res,
        Err(_err) => return Err(ApiError::Error("Invalid start date".to_string() + start)),
    };

    // if the end is not set - use 14 days from start
    let mut end_date: NaiveDateTime;
    if end.is_none() {
        end_date = start_date.checked_add_days(Days::new(13)).unwrap();
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
    let now = Local::now().naive_local();

    if now.timestamp() < end_date.timestamp() {
        end_date = now;
    }

    //  finally check that the date range doesn't exceed 14 days
    let duration = end_date - start_date;

    if duration.num_days() >= 14 {
        return Err(ApiError::Error("More than 14 days in range".to_string()));
    }

    //  normalise representation of end date
    Ok(end_date.format("%Y-%m-%dT%H:%MZ").to_string())
}

/// Return a representation of the end date
/// or an error if the dates are incorrectly formulated
pub async fn get_intensities_region(
    regionid: u8,
    start: &str,
    end: &Option<&str>,
) -> Result<RegionData, ApiError> {
    if regionid < 1 || regionid > 17 {
        return Err(ApiError::Error(
            "Invalid regiondid - should be between 1-17".to_string(),
        ));
    }

    let ed = normalise_dates(&start, &end)?;

    let path = "regional/intensity/";
    let url = format!("{BASE_URL}{path}{start}/{ed}/regionid/{regionid}");

    get_intensities(&url).await
}

///  ISO8601 format YYYY-MM-DDThh:mmZ
/// but tolerates YYYY-MM-DD
/// https://api.carbonintensity.org.uk/regional/intensity/2023-05-15/2023-05-20/postcode/RG10
///
pub async fn get_intensities_postcode(
    postcode: &str,
    start: &str,
    end: &Option<&str>,
) -> Result<RegionData, ApiError> {
    if postcode.len() < 2 || postcode.len() > 4 {
        return Err(ApiError::Error("Invalid postcode".to_string()));
    }

    let ed = normalise_dates(&start, &end)?;

    let path = "regional/intensity/";
    let url = format!("{BASE_URL}{path}{start}/{ed}/postcode/{postcode}");

    get_intensities(&url).await
}

pub async fn get_intensities(url: &str) -> Result<RegionData, ApiError> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();

    if status.is_success() {
        let json_str = response.text().await?;
        let structure: Result<PowerData, serde_json::Error> = serde_json::from_str(&json_str);
        if structure.is_ok() {
            Ok(structure.unwrap().data)
        } else {
            return Err(ApiError::Error(format!("Invalid JSON returned {json_str}")));
        }
    } else {
        let body = response.text().await?;
        Err(ApiError::RestError { status, body })
    }
}

/// Retrieves the intensity value from a structure
///
async fn get_intensity(url: &str) -> Result<i32, ApiError> {
    let result = get_instant_data(url).await.map_err(|err| err)?;

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
///
async fn get_instant_data(url: &str) -> Result<Root, ApiError> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();

    if status.is_success() {
        let structure = response.json::<Root>().await;
        if structure.is_ok() {
            return Ok(structure.unwrap());
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
}
