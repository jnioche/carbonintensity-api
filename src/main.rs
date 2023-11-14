use carbonintensity_api::get_intensity_postcode;
use std::process::exit;

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage:");
        eprintln!("  {} <postcode>", args[0]);
        exit(1);
    }
    let postcode = &args[1];
    
    let intensity = get_intensity_postcode(postcode).await.expect("Please enter a 3 character UK postcode");
    
    println!("Carbon intensity for postcode {}: {:?}", postcode, intensity);
}

