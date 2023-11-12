use error_chain::error_chain;
use std::{io::Read, process::exit};

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

fn main() -> Result<()> {

    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage:");
        eprintln!("  {} <postcode>", args[0]);
        exit(1);
    }
    let postcode = &args[1];

    let base_url = "https://api.carbonintensity.org.uk/regional/postcode/";
    let url = format!("{base_url}{postcode}");

    let mut res = reqwest::blocking::get(url)?;
    
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    println!("{}", body);

    Ok(())
}

