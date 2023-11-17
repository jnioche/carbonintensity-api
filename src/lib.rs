//! API for retrieving data from the Carbon Intensity API
//! <https://api.carbonintensity.org.uk/>

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
struct GenerationMix {
    fuel: String,
    perc: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Intensity {
    forecast: i32,
    index: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataEntry {
    from: String,
    to: String,
    intensity: Intensity,
    generationmix: Vec<GenerationMix>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
struct RegionData {
    regionid: i32,
    dnoregion: String,
    shortname: String,
    postcode: Option<String>,
    data: Vec<DataEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Root {
    data: Vec<RegionData>,
}

static BASE_URL: &str = "https://api.carbonintensity.org.uk/";

/// Current carbon intensity for a postcode
///
/// <https://api.carbonintensity.org.uk/regional/postcode/>
///
pub async fn get_intensity_postcode(postcode: &str) -> Result<i32, ApiError> {
    if postcode.len() < 2 || postcode.len() > 3 {
        return Err(ApiError::Error("Invalid postcode".to_string()));
    }

    let path = "regional/postcode/";
    let url = format!("{BASE_URL}{path}{postcode}");
    get_intensity(&url).await
}

/// Current carbon intensity for a region
/// The region id is a number between 1 and 17
///
/// <https://api.carbonintensity.org.uk/regional/regionid/>
///
pub async fn get_intensity_region(regionid: &u8) -> Result<i32, ApiError> {
    if regionid < &1 || regionid > &17 {
        return Err(ApiError::Error(
            "Invalid regiondid - should be between 1-17".to_string(),
        ));
    }

    let path = "regional/regionid/";
    let url = format!("{BASE_URL}{path}{regionid}");
    get_intensity(&url).await
}

/// Internal method to handle the querying and parsing
///
async fn get_intensity(url: &str) -> Result<i32, ApiError> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();

    if status.is_success() {
        let structure: Root = response.json::<Root>().await?;
        if let Some(data) = structure.data.first() {
            let entry = data.data.first().expect("Need to panic");
            // return intensity
            Ok(entry.intensity.forecast)
        } else {
            return Err(ApiError::Error("Invalid JSON returned".to_string()));
        }
    } else {
        let body = response.text().await?;
        Err(ApiError::RestError { status, body })
    }
}
