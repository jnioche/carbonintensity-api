use std::{error::Error, fmt::Display, num::ParseIntError, str::FromStr};

/// Region
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    NorthScotland = 1,
    SouthScotland = 2,
    NorthWestEngland = 3,
    NorthEastEngland = 4,
    SouthYorkshire = 5,
    NorthWalesMerseysideAndCheshire = 6,
    SouthWales = 7,
    WestMidlands = 8,
    EastMidlands = 9,
    EastEngland = 10,
    SouthWestEngland = 11,
    SouthEngland = 12,
    London = 13,
    SouthEastEngland = 14,
    England = 15,
    Scotland = 16,
    Wales = 17,
}

impl FromStr for Region {
    type Err = RegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let region_id = s.parse::<u8>()?;

        let region = match region_id {
            1 => Self::NorthScotland,
            2 => Self::SouthScotland,
            3 => Self::NorthWestEngland,
            4 => Self::NorthEastEngland,
            5 => Self::SouthYorkshire,
            6 => Self::NorthWalesMerseysideAndCheshire,
            7 => Self::SouthWales,
            8 => Self::WestMidlands,
            9 => Self::EastMidlands,
            10 => Self::EastEngland,
            11 => Self::SouthWestEngland,
            12 => Self::SouthEngland,
            13 => Self::London,
            14 => Self::SouthEastEngland,
            15 => Self::England,
            16 => Self::Scotland,
            17 => Self::Wales,
            _ => return Err(RegionError::OutsideRange),
        };

        Ok(region)
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Region::NorthScotland => "North Scotland",
            Region::SouthScotland => "South Scotland",
            Region::NorthWestEngland => "North West England",
            Region::NorthEastEngland => "North East England",
            Region::SouthYorkshire => "South Yorkshire",
            Region::NorthWalesMerseysideAndCheshire => "North Wales, Merseyside and Cheshire",
            Region::SouthWales => "South Wales",
            Region::WestMidlands => "West Midlands",
            Region::EastMidlands => "East Midlands",
            Region::EastEngland => "East England",
            Region::SouthWestEngland => "South West England",
            Region::SouthEngland => "South England",
            Region::London => "London",
            Region::SouthEastEngland => "South East England",
            Region::England => "England",
            Region::Scotland => "Scotland",
            Region::Wales => "Wales",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq)]
pub enum RegionError {
    ParseError,
    OutsideRange,
}

impl Error for RegionError {}

impl Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            RegionError::ParseError => "Failed to parse region id",
            RegionError::OutsideRange => {
                "Region id outside allowed range. Must be between 1 and 17 (inclusive)"
            }
        };
        write!(f, "{}", message)
    }
}

/// Conversion from/into a `ParseIntError`
///
/// This is useful when using the error propagation operator (`?`)
/// to avoid having to manually convert the error in the one returned.
impl From<ParseIntError> for RegionError {
    fn from(_: ParseIntError) -> Self {
        RegionError::ParseError
    }
}

#[cfg(test)]
mod tests {
    use super::{Region, RegionError};

    #[test]
    fn ids_match() {
        assert_eq!(1_u8, Region::NorthScotland as u8);
        assert_eq!(2_u8, Region::SouthScotland as u8);
        assert_eq!(3_u8, Region::NorthWestEngland as u8);
        assert_eq!(4_u8, Region::NorthEastEngland as u8);
        assert_eq!(5_u8, Region::SouthYorkshire as u8);
        assert_eq!(6_u8, Region::NorthWalesMerseysideAndCheshire as u8);
        assert_eq!(7_u8, Region::SouthWales as u8);
        assert_eq!(8_u8, Region::WestMidlands as u8);
        assert_eq!(9_u8, Region::EastMidlands as u8);
        assert_eq!(10_u8, Region::EastEngland as u8);
        assert_eq!(11_u8, Region::SouthWestEngland as u8);
        assert_eq!(12_u8, Region::SouthEngland as u8);
        assert_eq!(13_u8, Region::London as u8);
        assert_eq!(14_u8, Region::SouthEastEngland as u8);
        assert_eq!(15_u8, Region::England as u8);
        assert_eq!(16_u8, Region::Scotland as u8);
        assert_eq!(17_u8, Region::Wales as u8);
    }

    #[test]
    fn from_str() {
        assert_eq!("1".parse::<Region>(), Ok(Region::NorthScotland));
        assert_eq!("2".parse::<Region>(), Ok(Region::SouthScotland));
        assert_eq!("3".parse::<Region>(), Ok(Region::NorthWestEngland));
        assert_eq!("4".parse::<Region>(), Ok(Region::NorthEastEngland));
        assert_eq!("5".parse::<Region>(), Ok(Region::SouthYorkshire));
        assert_eq!(
            "6".parse::<Region>(),
            Ok(Region::NorthWalesMerseysideAndCheshire)
        );
        assert_eq!("7".parse::<Region>(), Ok(Region::SouthWales));
        assert_eq!("8".parse::<Region>(), Ok(Region::WestMidlands));
        assert_eq!("9".parse::<Region>(), Ok(Region::EastMidlands));
        assert_eq!("10".parse::<Region>(), Ok(Region::EastEngland));
        assert_eq!("11".parse::<Region>(), Ok(Region::SouthWestEngland));
        assert_eq!("12".parse::<Region>(), Ok(Region::SouthEngland));
        assert_eq!("13".parse::<Region>(), Ok(Region::London));
        assert_eq!("14".parse::<Region>(), Ok(Region::SouthEastEngland));
        assert_eq!("15".parse::<Region>(), Ok(Region::England));
        assert_eq!("16".parse::<Region>(), Ok(Region::Scotland));
        assert_eq!("17".parse::<Region>(), Ok(Region::Wales));
    }

    #[test]
    fn region_display() {
        assert_eq!(Region::NorthScotland.to_string(), "North Scotland");
        assert_eq!(Region::SouthScotland.to_string(), "South Scotland");
        assert_eq!(Region::NorthWestEngland.to_string(), "North West England");
        assert_eq!(Region::NorthEastEngland.to_string(), "North East England");
        assert_eq!(Region::SouthYorkshire.to_string(), "South Yorkshire");
        assert_eq!(
            Region::NorthWalesMerseysideAndCheshire.to_string(),
            "North Wales, Merseyside and Cheshire"
        );
        assert_eq!(Region::SouthWales.to_string(), "South Wales");
        assert_eq!(Region::WestMidlands.to_string(), "West Midlands");
        assert_eq!(Region::EastMidlands.to_string(), "East Midlands");
        assert_eq!(Region::EastEngland.to_string(), "East England");
        assert_eq!(Region::SouthWestEngland.to_string(), "South West England");
        assert_eq!(Region::SouthEngland.to_string(), "South England");
        assert_eq!(Region::London.to_string(), "London");
        assert_eq!(Region::SouthEastEngland.to_string(), "South East England");
        assert_eq!(Region::England.to_string(), "England");
        assert_eq!(Region::Scotland.to_string(), "Scotland");
        assert_eq!(Region::Wales.to_string(), "Wales");
    }

    #[test]
    fn error_display() {
        assert_eq!(
            RegionError::ParseError.to_string(),
            "Failed to parse region id"
        );
        assert_eq!(
            RegionError::OutsideRange.to_string(),
            "Region id outside allowed range. Must be between 1 and 17 (inclusive)"
        );
    }

    #[test]
    fn error_parse_int_conversion() {
        fn foo() -> Result<(), RegionError> {
            // Propagate ParseIntError to test conversion to RegionError
            let _: u8 = "foo".parse()?;

            Ok(())
        }

        assert_eq!(foo(), Err(RegionError::ParseError));
    }
}
