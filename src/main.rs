use std::process;

use carbonintensity_api::{get_intensity_postcode, get_intensity_region, ApiError};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// CLI for the CarbonIntensity API.
///
struct Args {
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

    match &args.command {
        Commands::Postcode { postcode } => {
            let result = get_intensity_postcode(postcode).await;
            handle_result(result, "postcode", postcode);
        }
        Commands::Region { id } => {
            let result = get_intensity_region(id).await;
            handle_result(result, "region", id.to_string().as_str());
        }
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
