# carbonintensity-api
[![crates.io](https://img.shields.io/crates/d/carbonintensity-api)](https://crates.io/crates/carbonintensity-api)
[![crates.io](https://img.shields.io/crates/v/carbonintensity-api)](https://crates.io/crates/carbonintensity-api)
[![API](https://docs.rs/carbonintensity-api/badge.svg)](https://docs.rs/carbonintensity-api)

A simple Rust library to help retrieve data from the [Carbon Intensity API](https://api.carbonintensity.org.uk/), not all functionalities of the CarbonIntensity API might be exposed.

Please read the API's [terms of use](https://github.com/carbon-intensity/terms).

## CLI

An executable is provided to try the library. With Rust and Cargo installed

```
cargo install --path .
```

then

`carbonintensity-api -h`

should display the list of available commands and options.

```
Provides a client for the UK National Grid Carbon Intensity API

Usage: carbonintensity-api [OPTIONS] <COMMAND>

Commands:
  postcode  Outward section of a UK postcode e.g. E1, BS7, WC2N
  region    Region ID, a number between 1 and 17
  help      Print this message or the help of the given subcommand(s)

Options:
  -s, --start-date <START_DATE>  
  -e, --end-date <END_DATE>      
  -h, --help                     Print help
  -V, --version                  Print version

```

To display the current carbon intensity for a given postcode

`carbonintensity-api postcode bs7`

To do the same for a region 

`carbonintensity-api region 11`

The region id is a number between 1 and 17

 1. North Scotland
 2. South Scotland
 3. North West England
 4. North East England
 5. South Yorkshire
 6. North Wales, Merseyside and Cheshire
 7. South Wales
 8. West Midlands
 9. East Midlands
 10. East England
 11. South West England
 12. South England
 13. London
 14. South East England
 15. England
 16. Scotland
 17. Wales


Specifying dates will return a list of intensities for a region or postcode. If no end date is provided, one will be generated with a value of 13 days from the start date.

The dates are expected to be at the `%Y-%m-%dT%H:%MZ` format or simply `%Y-%m-%d`, for instance 

`carbonintensity-api -s 2023-11-11 -e 2023-11-11T12:00Z postcode bs7`

Intensities are returned by 30 mins windows.

## Library

You can use the library in your Rust project by adding it to cargo with 

`cargo add carbonintensity-api`

then declaring it in your code 

```Rust
use carbonintensity::{
    get_intensities_postcode, get_intensities_region, get_intensity_postcode, get_intensity_region,
    ApiError,
};

...

  let result = get_intensity_postcode(postcode).await;

```

## License

This project is provided under [Apache License](http://www.apache.org/licenses/LICENSE-2.0).

