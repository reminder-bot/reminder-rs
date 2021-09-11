use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;

use crate::consts::REGEX_CHANNEL;

#[derive(Serialize, Deserialize)]
pub enum TimeDisplayType {
    Absolute = 0,
    Relative = 1,
}

#[derive(Serialize, Deserialize)]
pub struct LookFlags {
    pub show_disabled: bool,
    pub channel_id: Option<ChannelId>,
    pub time_display: TimeDisplayType,
}

impl Default for LookFlags {
    fn default() -> Self {
        Self { show_disabled: true, channel_id: None, time_display: TimeDisplayType::Relative }
    }
}
