use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use chrono_tz::Tz;
use chrono::offset::Utc;
use chrono::{Timelike, Datelike, TimeZone};

enum ParseType {
    Explicit,
    Displacement,
}

struct TimeParser {
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

    pub fn timestamp(&self) -> i32 {
        0
    }

    pub fn displacement(&self) -> i32 {
        0
    }

    fn process(&self) -> i32 {
        match self.parse_type {
            ParseType::Explicit => {
                // TODO remove unwrap from here
                self.process_explicit().unwrap()
            },

            ParseType::Displacement => {
                let now = SystemTime::now();
                let since_epoch = now
                    .duration_since(UNIX_EPOCH)
                    .expect("Time calculated as going backwards. Very bad");

                since_epoch.as_secs() as i32 + self.process_displacement()
            },
        }
    }

    fn process_explicit(&self) -> Result<i32, Box<dyn std::error::Error>> {
        let dt = self.timezone.datetime_from_str(self.time_string.as_str(), "%d/%m/%Y-%H:%M:%S")?;

        Ok(dt.timestamp() as i32)
    }

    fn process_displacement(&self) -> i32 {
        let mut current_buffer = "0".to_string();

        let mut seconds = 0 as i32;
        let mut minutes = 0 as i32;
        let mut hours = 0 as i32;
        let mut days = 0 as i32;

        for character in self.time_string.chars() {
            match character {

                's' => {
                    seconds = current_buffer.parse::<i32>().unwrap();
                    current_buffer = String::from("0");
                },

                'm' => {
                    minutes = current_buffer.parse::<i32>().unwrap();
                    current_buffer = String::from("0");
                },

                'h' => {
                    hours = current_buffer.parse::<i32>().unwrap();
                    current_buffer = String::from("0");
                },

                'd' => {
                    days = current_buffer.parse::<i32>().unwrap();
                    current_buffer = String::from("0");
                },

                c => {
                    if c.is_digit(10) {
                        current_buffer += &c.to_string();
                    }
                    else {
                        // raise exception
                    }
                },
            }
        }

        let full = seconds + (minutes * 60) + (hours * 3600) + (days * 86400) + current_buffer.parse::<i32>().unwrap() *
            if self.inverted { -1 } else { 1 };

        full
    }
}
