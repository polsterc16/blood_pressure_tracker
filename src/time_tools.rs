use chrono::{DateTime, Datelike, Utc};
use pretty_simple_display::DebugPretty;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, DebugPretty)]
#[allow(non_snake_case)]
pub struct DateSimpleJson {
    pub timestamp: i64,
    pub Y: i32,
    pub m: u32,
    pub d: u32,
}
impl DateSimpleJson {
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            Y: 0,
            m: 0,
            d: 0,
        }
    }
    pub fn from_utc(date_time_utc: DateTime<Utc>) -> Self {
        Self {
            timestamp: date_time_utc.timestamp(),
            Y: date_time_utc.year(),
            m: date_time_utc.month(),
            d: date_time_utc.day(),
        }
    }
    pub fn set_utc(&mut self, date_time_utc: &DateTime<Utc>) {
        self.timestamp = date_time_utc.timestamp();
        self.Y = date_time_utc.year();
        self.m = date_time_utc.month();
        self.d = date_time_utc.day();
    }
}
