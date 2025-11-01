use std::{fmt::Display, str::FromStr};

use crate::state::ParseError;

#[derive(Clone, Debug)]
pub struct Workspace {
    pub workspace_id: i32,
    pub timestamp: i64,
}

impl Display for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{}", self.workspace_id, self.timestamp)
    }
}

impl FromStr for Workspace {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::InvalidFormat);
        }
        let parts: Vec<&str> = s.split(";").collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidFormat);
        }

        let workspace_id: i32 = parts[0].parse()?;
        let timestamp: i64 = parts[1].parse()?;

        Ok(Workspace {
            workspace_id,
            timestamp,
        })
    }
}
