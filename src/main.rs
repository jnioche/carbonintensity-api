use std::{process, str::FromStr};

use carbonintensity::{
    get_intensities_postcode, get_intensities_region, get_intensity_postcode, get_intensity_region,
    ApiError,
};
use chrono::NaiveDateTime;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// CLI for the CarbonIntensity API.
///
/// Dates can be specified either is ISO-8601 (`2022-08-21T09:00:00Z`) or simply
/// YYYY-MM-DD. If no end date is specified, it will be set to 14 days from the start date.
struct Args {
    #[clap(short, long)]
    pub start_date: Option<String>,
    #[clap(short, long)]
    pub end_date: Option<String>,

    #[clap()]
    /// numerical value for a region (1-17) or first part of a UK postcode
    pub value: String,
}

enum Target {
    // NATIONAL,
    POSTCODE,
    REGION,
}

impl FromStr for Target {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            //"" => Ok(Target::NATIONAL),
            _ if s.parse::<u8>().is_ok() => Ok(Target::REGION),
            _ => Ok(Target::POSTCODE),
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // let target: Target = args.value.parse().unwrap_or(Target::NATIONAL);
    let target: Target = args.value.parse().unwrap();

    // look for a range if a date was specified
    if let Some(start_date) = &args.start_date {
        let end_date: Option<&str> = args.end_date.as_ref().map(|r| &**r);

        match target {
            Target::POSTCODE => {
                let result =
                    get_intensities_postcode(args.value.as_str(), start_date, &end_date).await;
                handle_results(result);
            }
            Target::REGION => {
                let id: u8 = args.value.parse::<u8>().unwrap();
                let result = get_intensities_region(id, start_date, &end_date).await;
                handle_results(result);
            }
        }
    } else {
        match target {
            Target::POSTCODE => {
                let postcode = args.value.as_str();
                let result = get_intensity_postcode(postcode).await;
                handle_result(result, "postcode", postcode);
            }
            Target::REGION => {
                let id: u8 = args.value.parse::<u8>().unwrap();
                let result = get_intensity_region(id).await;
                handle_result(result, "region", id.to_string().as_str());
            }
        }
    }
}

fn handle_results(result: Result<Vec<(NaiveDateTime, i32)>, ApiError>) {
    if result.is_ok() {
        for t in result.unwrap() {
            println!("{}, {}", t.0, t.1);
        }
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}

fn handle_result(result: Result<i32, ApiError>, method: &str, value: &str) {
    if result.is_ok() {
        println!(
            "Carbon intensity for {} {}: {:?}",
            method,
            value,
            result.unwrap()
        );
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}
