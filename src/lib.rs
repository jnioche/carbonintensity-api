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

#[derive(Debug, Serialize, Deserialize)]
struct RegionData {
    regionid: i32,
    dnoregion: String,
    shortname: String,
    postcode: String,
    data: Vec<DataEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Root {
    data: Vec<RegionData>,
}

/// Current carbon intensity for a postcode
/// in the UK national grid
///
/// https://api.carbonintensity.org.uk/regional/postcode/
///
pub async fn get_intensity_postcode(postcode: &str) -> Result<i32, ApiError> {
    let base_url = "https://api.carbonintensity.org.uk/regional/postcode/";
    let client = Client::new();
    let url = format!("{base_url}{postcode}");
    let response = client.get(url).send().await?;

    let status = response.status();
    if status.is_success() {
        let structure: Root = response.json::<Root>().await?;
        if let Some(data) = structure.data.first() {
            let entry = data.data.first().expect("Need to panic");
            // return intensity
            Ok(entry.intensity.forecast)
        } else {
            return Err(ApiError::Error("Ivalid JSON returned".to_string()));
        }
    } else {
        let body = response.text().await?;
        Err(ApiError::RestError { status, body })
    }
}
