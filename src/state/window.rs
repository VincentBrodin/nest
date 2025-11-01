use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Window {
    pub class: String,
    pub timestamp: DateTime<Utc>,
    pub origin: i32,
}
