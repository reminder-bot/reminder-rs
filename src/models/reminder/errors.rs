use crate::consts::{MAX_TIME, MIN_INTERVAL};

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ReminderError {
    LongTime,
    LongInterval,
    PastTime,
    ShortInterval,
    InvalidTag,
    DiscordError(String),
}

impl ToString for ReminderError {
    fn to_string(&self) -> String {
        match self {
            ReminderError::LongTime => {
                "That time is too far in the future. Please specify a shorter time.".to_string()
            }
            ReminderError::LongInterval => format!(
                "Please ensure the interval specified is less than {max_time} days",
                max_time = *MAX_TIME / 86_400
            ),
            ReminderError::PastTime => {
                "Please ensure the time provided is in the future. If the time should be in the future, please be more specific with the definition.".to_string()
            }
            ReminderError::ShortInterval => format!(
                "Please ensure the interval provided is longer than {min_interval} seconds",
                min_interval = *MIN_INTERVAL
            ),
            ReminderError::InvalidTag => {
                "Couldn't find a location by your tag. Your tag must be either a channel or a user (not a role)".to_string()
            }
            ReminderError::DiscordError(s) => format!("A Discord error occurred: **{}**", s),
        }
    }
}
