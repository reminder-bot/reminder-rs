use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use std::fmt::{
    Formatter,
    Display,
    Result as FmtResult,
};

use chrono_tz::Tz;
use chrono::TimeZone;

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

enum ParseType {
    Explicit,
    Displacement,
}

pub struct TimeParser {
    timezone: Tz,
    inverted: bool,
    time_string: String,
    parse_type: ParseType,
}

impl TimeParser {
    pub fn new(input: String, timezone: Tz) -> Self {
        let inverted = if input.starts_with("-") {
            true
        }
        else {
            false
        };

        let parse_type = if input.contains("/") || input.contains(":") {
            ParseType::Explicit
        }
        else {
            ParseType::Displacement
        };

        Self {
            timezone,
            inverted,
            time_string: input.trim_start_matches("-").to_string(),
            parse_type,
        }
    }

    pub fn timestamp(&self) -> Result<i64, InvalidTime> {
        match self.parse_type {
            ParseType::Explicit => {
                Ok(self.process_explicit()?)
            },

            ParseType::Displacement => {
                let now = SystemTime::now();
                let since_epoch = now
                    .duration_since(UNIX_EPOCH)
                    .expect("Time calculated as going backwards. Very bad");

                Ok(since_epoch.as_secs() as i64 + self.process_displacement()?)
            },
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
            },

            ParseType::Displacement => {
                Ok(self.process_displacement()?)
            },
        }
    }


    fn process_explicit(&self) -> Result<i64, InvalidTime> {
        let segments = self.time_string.matches("-").count();

        let parse_string = if segments == 1 {
            let slashes = self.time_string.matches("/").count();

            match slashes {
                0 => Ok("%d-".to_string()),
                1 => Ok("%d/%m-".to_string()),
                2 => Ok("%d/%m/%Y-".to_string()),
                _ => Err(InvalidTime::ParseErrorDMY)
            }
        } else {
            Ok("".to_string())
        }? + if segments == 1 {
            let colons = self.time_string.matches(":").count();

            match colons {
                1 => Ok("%H:%M"),
                2 => Ok("%H:%M:%S"),
                _ => Err(InvalidTime::ParseErrorHMS)
            }
        } else {
            Ok("")
        }?;

        let dt = self.timezone.datetime_from_str(self.time_string.as_str(), &parse_string).map_err(|_| InvalidTime::ParseErrorChrono)?;

        Ok(dt.timestamp() as i64)
    }

    fn process_displacement(&self) -> Result<i64, InvalidTime> {
        let mut current_buffer = "0".to_string();

        let mut seconds = 0 as i64;
        let mut minutes = 0 as i64;
        let mut hours = 0 as i64;
        let mut days = 0 as i64;

        for character in self.time_string.chars() {
            match character {

                's' => {
                    seconds = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                },

                'm' => {
                    minutes = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                },

                'h' => {
                    hours = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                },

                'd' => {
                    days = current_buffer.parse::<i64>().unwrap();
                    current_buffer = String::from("0");
                },

                c => {
                    if c.is_digit(10) {
                        current_buffer += &c.to_string();
                    }
                    else {
                        return Err(InvalidTime::ParseErrorDisplacement)
                    }
                },
            }
        }

        let full = (seconds + (minutes * 60) + (hours * 3600) + (days * 86400) + current_buffer.parse::<i64>().unwrap()) *
            if self.inverted { -1 } else { 1 };

        Ok(full)
    }
}
