use std::{
    convert::TryFrom,
    fmt::{Display, Formatter, Result as FmtResult},
    str::from_utf8,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;
use tokio::process::Command;

use crate::consts::{LOCAL_TIMEZONE, PYTHON_LOCATION};

#[derive(Debug)]
pub enum InvalidTime {
    ParseErrorDMY,
    ParseErrorHMS,
    ParseErrorDisplacement,
    ParseErrorChrono,
}

impl Display for InvalidTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "InvalidTime: {:?}", self)
    }
}

impl std::error::Error for InvalidTime {}

#[derive(Copy, Clone)]
enum ParseType {
    Explicit,
    Displacement,
}

#[derive(Clone)]
pub struct TimeParser {
    timezone: Tz,
    inverted: bool,
    time_string: String,
    parse_type: ParseType,
}

impl TryFrom<&TimeParser> for i64 {
    type Error = InvalidTime;

    fn try_from(value: &TimeParser) -> Result<Self, Self::Error> {
        value.timestamp()
    }
}

impl TimeParser {
    pub fn new(input: &str, timezone: Tz) -> Self {
        let inverted = input.starts_with('-');

        let parse_type = if input.contains('/') || input.contains(':') {
            ParseType::Explicit
        } else {
            ParseType::Displacement
        };

        Self {
            timezone,
            inverted,
            time_string: input.trim_start_matches('-').to_string(),
            parse_type,
        }
    }

    pub fn timestamp(&self) -> Result<i64, InvalidTime> {
        match self.parse_type {
            ParseType::Explicit => Ok(self.process_explicit()?),

            ParseType::Displacement => {
                let now = SystemTime::now();
                let since_epoch = now
                    .duration_since(UNIX_EPOCH)
                    .expect("Time calculated as going backwards. Very bad");

                Ok(since_epoch.as_secs() as i64 + self.process_displacement()?)
            }
        }
    }

    pub fn displacement(&self) -> Result<i64, InvalidTime> {
        match self.parse_type {
            ParseType::Explicit => {
                let now = SystemTime::now();
                let since_epoch = now
                    .duration_since(UNIX_EPOCH)
                    .expect("Time calculated as going backwards. Very bad");

                Ok(self.process_explicit()? - since_epoch.as_secs() as i64)
            }

            ParseType::Displacement => Ok(self.process_displacement()?),
        }
    }

    fn process_explicit(&self) -> Result<i64, InvalidTime> {
        let mut time = Utc::now()
            .with_timezone(&self.timezone)
            .with_second(0)
            .unwrap();

        let mut segments = self.time_string.rsplit('-');
        // this segment will always exist even if split fails
        let hms = segments.next().unwrap();

        let h_m_s = hms.split(':');

        for (t, setter) in h_m_s.take(3).zip(&[
            DateTime::with_hour,
            DateTime::with_minute,
            DateTime::with_second,
        ]) {
            time = setter(&time, t.parse().map_err(|_| InvalidTime::ParseErrorHMS)?)
                .map_or_else(|| Err(InvalidTime::ParseErrorHMS), Ok)?;
        }

        if let Some(dmy) = segments.next() {
            let mut d_m_y = dmy.split('/');

            let day = d_m_y.next();
            let month = d_m_y.next();
            let year = d_m_y.next();

            for (t, setter) in [day, month]
                .iter()
                .zip(&[DateTime::with_day, DateTime::with_month])
            {
                if let Some(t) = t {
                    time = setter(&time, t.parse().map_err(|_| InvalidTime::ParseErrorDMY)?)
                        .map_or_else(|| Err(InvalidTime::ParseErrorDMY), Ok)?;
                }
            }

            if let Some(year) = year {
                if year.len() == 4 {
                    time = time
                        .with_year(year.parse().map_err(|_| InvalidTime::ParseErrorDMY)?)
                        .map_or_else(|| Err(InvalidTime::ParseErrorDMY), Ok)?;
                } else if year.len() == 2 {
                    time = time
                        .with_year(
                            format!("20{}", year)
                                .parse()
                                .map_err(|_| InvalidTime::ParseErrorDMY)?,
                        )
                        .map_or_else(|| Err(InvalidTime::ParseErrorDMY), Ok)?;
                } else {
                    return Err(InvalidTime::ParseErrorDMY);
                }
            }
        }

        Ok(time.timestamp() as i64)
    }

    fn process_displacement(&self) -> Result<i64, InvalidTime> {
        let mut current_buffer = "0".to_string();

        let mut seconds = 0_i64;
        let mut minutes = 0_i64;
        let mut hours = 0_i64;
        let mut days = 0_i64;

        for character in self.time_string.chars() {
            match character {
                's' => {
                    seconds = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                }

                'm' => {
                    minutes = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                }

                'h' => {
                    hours = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                }

                'd' => {
                    days = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                }

                c => {
                    if c.is_digit(10) {
                        current_buffer += &c.to_string();
                    } else {
                        return Err(InvalidTime::ParseErrorDisplacement);
                    }
                }
            }
        }

        let full = (seconds
            + (minutes * 60)
            + (hours * 3600)
            + (days * 86400)
            + current_buffer.parse::<i64>().unwrap())
            * if self.inverted { -1 } else { 1 };

        Ok(full)
    }
}

pub async fn natural_parser(time: &str, timezone: &str) -> Option<i64> {
    Command::new(&*PYTHON_LOCATION)
        .arg("-c")
        .arg(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dp.py")))
        .arg(time)
        .arg(timezone)
        .arg(&*LOCAL_TIMEZONE)
        .output()
        .await
        .ok()
        .map(|inner| {
            if inner.status.success() {
                Some(from_utf8(&*inner.stdout).unwrap().parse::<i64>().unwrap())
            } else {
                None
            }
        })
        .flatten()
        .map(|inner| if inner < 0 { None } else { Some(inner) })
        .flatten()
}
