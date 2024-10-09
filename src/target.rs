use std::str::FromStr;

use crate::Region;

/// Carbon intensity target, e.g. a postcode or a region
#[derive(PartialEq)]
pub enum Target {
    National,
    Postcode(String),
    Region(Region),
}

impl FromStr for Target {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Target::National);
        }

        // Check if input can be parsed as a Region
        if let Ok(region) = s.parse::<Region>() {
            return Ok(Target::Region(region));
        }

        // Assumes the string was a postcode
        Ok(Target::Postcode(s.to_string()))
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = match self {
            Target::National => "National ".to_string(),
            Target::Postcode(postcode) => format!("postcode {postcode}"),
            Target::Region(region) => region.to_string(),
        };

        write!(f, "{target}")
    }
}
