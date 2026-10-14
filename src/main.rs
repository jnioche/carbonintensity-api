use std::process;

use carbonintensity::{
    get_intensities, get_intensities_with_mix, get_intensity, ApiError, IntensityWithMix, Target,
    FUELS,
};
use chrono::NaiveDateTime;
use clap::Parser;
use std::io::Write;

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

    /// outputs the generation mix as CSV along with the intensities
    /// (regions and postcodes only, requires a start date)
    #[clap(short, long, requires = "start_date")]
    pub mix: bool,

    /// numerical value for a region (1-17) or first part of a UK postcode
    /// returns data at the national level if not set
    #[clap()]
    #[arg(default_value_t=Target::National)]
    pub target: Target,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let target: Target = args.target;

    // look for a range if a date was specified
    if let Some(start_date) = &args.start_date {
        let end_date: Option<&str> = args.end_date.as_deref();

        if args.mix {
            let result = get_intensities_with_mix(&target, start_date, &end_date).await;
            handle_results_with_mix(result);
        } else {
            let result = get_intensities(&target, start_date, &end_date).await;
            handle_results(result);
        }
    } else {
        let result = get_intensity(&target).await;
        handle_result(result, &target);
    }
}

#[allow(clippy::explicit_write)]
fn handle_results(result: Result<Vec<(NaiveDateTime, i32)>, ApiError>) {
    if let Ok(results) = result {
        for (time, value) in results {
            writeln!(std::io::stdout(), "{}, {}", time, value).unwrap_or_default();
        }
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}

/// Writes the results as CSV with one column per fuel
#[allow(clippy::explicit_write)]
fn handle_results_with_mix(result: Result<Vec<IntensityWithMix>, ApiError>) {
    let results = result.unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "time,intensity,{}", FUELS.join(",")).unwrap_or_default();
    for result in results {
        for m in &result.mix {
            if !FUELS.contains(&m.fuel.as_str()) {
                eprintln!("Unknown fuel '{}' at {} ignored", m.fuel, result.from);
            }
        }
        let percs: Vec<String> = FUELS
            .iter()
            .map(|fuel| result.perc(fuel).map(|p| p.to_string()).unwrap_or_default())
            .collect();
        writeln!(
            stdout,
            "{},{},{}",
            result.from,
            result.intensity,
            percs.join(",")
        )
        .unwrap_or_default();
    }
}

#[allow(clippy::explicit_write)]
fn handle_result(result: Result<i32, ApiError>, target: &Target) {
    if result.is_ok() {
        writeln!(
            std::io::stdout(),
            "Carbon intensity for {}: {:?}",
            target,
            result.unwrap()
        )
        .unwrap_or_default();
    } else {
        eprintln!("{}", result.unwrap_err());
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use carbonintensity::Region;

    use crate::{Args, Target};

    fn parsed_args(args: Vec<&str>) -> Result<Args, clap::Error> {
        let args = ["carbonintensity-api"].iter().chain(args.iter());
        Args::try_parse_from(args)
    }

    #[test]
    fn cli_valid_arguments() {
        // single postcode
        let args: Args = parsed_args(vec!["bs7"]).unwrap();
        assert_eq!(args.target, Target::Postcode("bs7".to_string()));

        // single region id
        let args = parsed_args(vec!["13"]).unwrap();
        assert_eq!(args.target, Target::Region(Region::London));

        // start date  / postcode
        let args = parsed_args(vec!["--start-date", "2024-05-06", "BS7"]).unwrap();
        assert_eq!(args.start_date, Some("2024-05-06".to_string()));
        assert_eq!(args.target, Target::Postcode("BS7".to_string()));

        // start date / region id
        let args = parsed_args(vec!["--start-date", "2024-05-06", "16"]).unwrap();
        assert_eq!(args.start_date, Some("2024-05-06".to_string()));
        assert_eq!(args.target, Target::Region(Region::Scotland));

        // start date / end date
        let args = parsed_args(vec![
            "--start-date",
            "2024-05-06",
            "--end-date",
            "2024-07-08",
            "BS7",
        ])
        .unwrap();
        assert_eq!(args.start_date, Some("2024-05-06".to_string()));
        assert_eq!(args.end_date, Some("2024-07-08".to_string()));
        assert_eq!(args.target, Target::Postcode("BS7".to_string()));

        // short names
        parsed_args(vec!["-s 2024-05-06", "-e 2024-05-06", "BS7"]).unwrap();
        parsed_args(vec!["-s 2024-05-06", "BS7"]).unwrap();
        parsed_args(vec!["-e 2024-05-06", "BS7"]).unwrap();

        // mix
        let args = parsed_args(vec!["-s", "2024-05-06", "--mix", "13"]).unwrap();
        assert!(args.mix);
        let args = parsed_args(vec!["-s", "2024-05-06", "13"]).unwrap();
        assert!(!args.mix);
    }

    #[test]
    fn cli_mix_requires_start_date() {
        assert!(parsed_args(vec!["--mix", "13"]).is_err());
    }
}
