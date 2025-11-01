use std::{fmt::Display, str::FromStr};

use crate::state::ParseError;

#[derive(Clone, Debug)]
pub struct FloatingWindow {
    pub at: (i16, i16),
    pub size: (i16, i16),
}

impl Display for FloatingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{};{};{};{}",
            self.at.0, self.at.1, self.size.0, self.size.1
        )
    }
}

impl FromStr for FloatingWindow {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::InvalidFormat);
        }
        let parts: Vec<&str> = s.split(";").collect();
        if parts.len() != 4 {
            return Err(ParseError::InvalidFormat);
        }

        let at_x: i16 = parts[0].parse()?;
        let at_y: i16 = parts[1].parse()?;
        let size_x: i16 = parts[2].parse()?;
        let size_y: i16 = parts[3].parse()?;

        Ok(FloatingWindow {
            at: (at_x, at_y),
            size: (size_x, size_y),
        })
    }
}
