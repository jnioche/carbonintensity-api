use std::str::FromStr;

use crate::Region;

pub enum Target {
    // NATIONAL,
    Postcode(String),
    Region(Region),
}

impl FromStr for Target {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //"" => Ok(Target::NATIONAL)

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
            Target::Postcode(postcode) => format!("postcode {postcode}"),
            Target::Region(region) => region.to_string(),
        };

        write!(f, "{target}")
    }
}
