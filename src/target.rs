use crate::Region;

/// Carbon intensity target, e.g. a postcode or a region
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    National,
    Postcode(String),
    Region(Region),
}

/// Creates a `Target` from a `String`
///
/// If the string is equal to 'national' or is empty returns `Target::National.
/// If it contains a valid `Region` id this returns a `Target::Region`,
/// otherwise it returns a `Target::Postcode`.
///
/// Note how this is infallible because it balls back to `Target::Postcode`.
///
/// ```
/// # use carbonintensity::{Target, Region};
/// let target = Target::from("13".to_string());
/// assert_eq!(target, Target::Region(Region::London));
///
/// let target = Target::from("BS7".to_string());
/// let bs7 = Target::Postcode("BS7".to_string());
/// assert_eq!(target, bs7);
/// ```
impl From<String> for Target {
    fn from(s: String) -> Self {
        if s.trim().is_empty() | s.trim().eq_ignore_ascii_case("national") {
            return Self::National;
        }

        // Check if input can be parsed as a Region
        if let Ok(region) = s.parse::<Region>() {
            return Self::Region(region);
        }

        // Assumes the string was a postcode
        Self::Postcode(s)
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = match self {
            Target::National => "National".to_string(),
            Target::Postcode(postcode) => format!("postcode {postcode}"),
            Target::Region(region) => region.to_string(),
        };

        write!(f, "{target}")
    }
}
