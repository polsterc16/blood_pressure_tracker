use anyhow::{Context, bail};
use chrono::{DateTime, Datelike, Local, Utc};
use pretty_simple_display::DebugPretty;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

// ################################################################

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DateYearMonth {
    pub year: i32,
    pub month: i32,
}
impl fmt::Display for DateYearMonth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}
impl FromStr for DateYearMonth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let date_str: &str = match s.split_once('.') {
            Some((a, b)) => a,
            None => s,
        };
        if date_str.len() != "2000-01".len() {
            bail!("Unable to parse: `{}` (`yyyy-mm`)", s)
        }

        let year = date_str[0..4]
            .parse::<i32>()
            .context(format!("Unable to parse: `{}` (`yyyy-mm`)", s))?;
        let month = date_str[5..]
            .parse::<i32>()
            .context(format!("Unable to parse: `{}` (`yyyy-mm`)", s))?;

        if month > 12 || month < 1 {
            bail!("Unable to parse: `{}` (`yyyy-mm`)", s)
        }

        return anyhow::Ok(Self {
            year: year,
            month: month,
        });
    }
}
impl DateYearMonth {
    pub fn new(year: i32, month: i32) -> Self {
        if month > 12 || month < 1 {
            panic!("`month` not valid: {}", month);
        }
        Self {
            year: year,
            month: month,
        }
    }
    pub fn empty() -> Self {
        return Self::new(2000, 1);
    }
    pub fn from_now() -> Self {
        let now = Local::now();
        return Self::new(now.year(), now.month() as i32);
    }
    pub fn from_utc(date_time_utc: DateTime<Utc>) -> Self {
        Self::new(date_time_utc.year(), date_time_utc.month() as i32)
    }

    pub fn set_ym(&mut self, year: i32, month: i32) -> anyhow::Result<()> {
        if month > 12 || month < 1 {
            bail!("`month` not valid: {}", month);
        }
        self.year = year;
        self.month = month;

        return anyhow::Ok(());
    }
    pub fn set_utc(&mut self, date_time_utc: &DateTime<Utc>) {
        self.set_ym(date_time_utc.year(), date_time_utc.month() as i32)
            .unwrap();
    }
    pub fn get_ym(&self) -> [i32; 2] {
        [self.year, self.month]
    }

    pub fn add_months(&mut self, months: i32) -> anyhow::Result<()> {
        // self.year = date_time_utc.year();
        self.month += months;

        let mut wdt = 100;

        while self.month > 12 {
            self.month -= 12;
            self.year += 1;

            wdt -= 1;
            if wdt < 0 {
                bail!("wdt timed out!");
            }
        }
        while self.month < 1 {
            self.month += 12;
            self.year -= 1;

            wdt -= 1;
            if wdt < 0 {
                bail!("wdt timed out!");
            }
        }
        return anyhow::Ok(());
    }
}

// ################################################################

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
