use std::process;

use carbonintensity::{
    get_intensities_postcode, get_intensities_region, get_intensity_postcode, get_intensity_region,
    ApiError, RegionData,
};
use clap::{Parser, Subcommand};

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

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// UK postcode e.g. BS7, E1
    Postcode { postcode: String },
    /// Region ID, a number between 1 and 17
    Region { id: u8 },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    // look for a range if a date was specified
    if let Some(start_date) = &args.start_date {
        let end_date: Option<&str> = args.end_date.as_ref().map(|r| &**r);

        match &args.command {
            Commands::Postcode { postcode } => {
                let result = get_intensities_postcode(postcode, start_date, &end_date).await;
                handle_results(result);
            }
            Commands::Region { id } => {
                let result = get_intensities_region(*id, start_date, &end_date).await;
                handle_results(result);
            }
        }
    } else {
        match &args.command {
            Commands::Postcode { postcode } => {
                let result = get_intensity_postcode(postcode).await;
                handle_result(result, "postcode", postcode);
            }
            Commands::Region { id } => {
                let result = get_intensity_region(*id).await;
                handle_result(result, "region", id.to_string().as_str());
            }
        }
    }
}

fn handle_results(result: Result<RegionData, ApiError>) {
    if result.is_ok() {
        println!("{:?}", result.unwrap());
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
