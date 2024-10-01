use std::process;

use carbonintensity::{
    get_intensities_postcode, get_intensities_region, get_intensity_postcode, get_intensity_region,
    ApiError, Region,
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // let target: Target = args.value.parse().unwrap_or(Target::NATIONAL);
    let target: Target = args.value.parse().unwrap();

    // look for a range if a date was specified
    if let Some(start_date) = &args.start_date {
        let end_date: Option<&str> = args.end_date.as_deref();

        match target {
            Target::Postcode(postcode) => {
                let result = get_intensities_postcode(&postcode, start_date, &end_date).await;
                handle_results(result);
            }
            Target::Region(region) => {
                let result = get_intensities_region(region, start_date, &end_date).await;
                handle_results(result);
            }
        }
    } else {
        match target {
            Target::Postcode(postcode) => {
                let result = get_intensity_postcode(&postcode).await;
                handle_result(result, &"postcode", &postcode);
            }
            Target::Region(region) => {
                let result = get_intensity_region(region).await;
                handle_result(result, &"region", &region);
            }
        }
    }
}

fn handle_results(result: Result<Vec<(NaiveDateTime, i32)>, ApiError>) {
    if let Ok(results) = result {
        for (time, value) in results {
            println!("{}, {}", time, value);
        }
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}

fn handle_result(result: Result<i32, ApiError>, target: &Target) {
    if result.is_ok() {
        println!("Carbon intensity for {}: {:?}", target, result.unwrap());
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}
