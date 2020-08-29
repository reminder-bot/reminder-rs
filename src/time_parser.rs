use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use chrono_tz::Tz;

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

    }

    pub fn displacement(&self) -> i32 {

    }

    fn process(&self) -> i32 {
        match self.parse_type {
            ParseType::Explicit => {
                self.process_explicit()
            },

            ParseType::Displacement => {
                let now = SystemTime::now();
                let since_epoch = now
                    .duration_since(UNIX_EPOCH)
                    .expect("Time calculated as going backwards. Very bad");

                since_epoch.as_secs() + self.process_displacement()
            },
        }
    }

    fn process_explicit(&self) -> i32 {

        0
    }

    fn process_displacement(&self) -> i32 {
        let mut current_buffer = "0".to_string();

        let mut seconds = 0;
        let mut minutes = 0;
        let mut hours = 0;
        let mut days = 0;

        for character in self.time_string.chars() {
            match character {

                's' => {
                    seconds = current_buffer.parse::<u32>().unwrap();
                    current_buffer = String::from("0");
                },

                'm' => {
                    minutes = current_buffer.parse::<u32>().unwrap();
                    current_buffer = String::from("0");
                },

                'h' => {
                    hours = current_buffer.parse::<u32>().unwrap();
                    current_buffer = String::from("0");
                },

                'd' => {
                    days = current_buffer.parse::<u32>().unwrap();
                    current_buffer = String::from("0");
                },

                c => {
                    if c.is_digit(10) {
                        current_buffer += c.as_str();
                    }
                    else {
                        // raise exception
                    }
                },
            }
        }

        let full = seconds + (minutes * 60) + (hours * 3600) + (days * 86400) + current_buffer.parse::<u32>() *
            if self.inverted { -1 } else { 1 };

        full
    }
}
