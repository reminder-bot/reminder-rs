use serde::{Deserialize, Serialize};
use serde_repr::*;
use serenity::model::id::ChannelId;

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Debug)]
#[repr(u8)]
pub enum TimeDisplayType {
    Absolute = 0,
    Relative = 1,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
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
