# carbonintensity-api
A simple Rust library to help retrieve data from [https://api.carbonintensity.org.uk/]

Mostly a way for the author to learn Rust, not all functionalities of the CarbonIntensity API will be exposed.

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
