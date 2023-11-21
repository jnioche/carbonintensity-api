# carbonintensity-api
[![crates.io](https://img.shields.io/crates/d/carbonintensity-api)](https://crates.io/crates/carbonintensity-api)
[![crates.io](https://img.shields.io/crates/v/carbonintensity-api)](https://crates.io/crates/carbonintensity-api)
[![API](https://docs.rs/carbonintensity-api/badge.svg)](https://docs.rs/carbonintensity-api)

A simple Rust library to help retrieve data from the [Carbon Intensity API](https://api.carbonintensity.org.uk/), not all functionalities of the CarbonIntensity API might be exposed.

Please read the API's [terms of use](https://github.com/carbon-intensity/terms).

An executable is provided to try the library. With Rust and Cargo installed

```
cargo run help
```

should display the list of available commands and options.

```
Usage: carbonintensity-api [OPTIONS] <COMMAND>

Commands:
  postcode  UK postcode e.g. BS7, E1
  region    Region ID, a number between 1 and 17
  help      Print this message or the help of the given subcommand(s)

Options:
  -s, --start-date <START_DATE>  
  -e, --end-date <END_DATE>      
  -h, --help                     Print help
  -V, --version                  Print version
```

## License

This project is provided under [Apache License](http://www.apache.org/licenses/LICENSE-2.0).
