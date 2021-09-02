use serenity::model::id::ChannelId;

use crate::consts::REGEX_CHANNEL;

pub enum TimeDisplayType {
    Absolute,
    Relative,
}

pub struct LookFlags {
    pub limit: u16,
    pub show_disabled: bool,
    pub channel_id: Option<ChannelId>,
    pub time_display: TimeDisplayType,
}

impl Default for LookFlags {
    fn default() -> Self {
        Self {
            limit: u16::MAX,
            show_disabled: true,
            channel_id: None,
            time_display: TimeDisplayType::Relative,
        }
    }
}

impl LookFlags {
    pub fn from_string(args: &str) -> Self {
        let mut new_flags: Self = Default::default();

        for arg in args.split(' ') {
            match arg {
                "enabled" => {
                    new_flags.show_disabled = false;
                }

                "time" => {
                    new_flags.time_display = TimeDisplayType::Absolute;
                }

                param => {
                    if let Ok(val) = param.parse::<u16>() {
                        new_flags.limit = val;
                    } else if let Some(channel) = REGEX_CHANNEL
                        .captures(arg)
                        .map(|cap| cap.get(1))
                        .flatten()
                        .map(|c| c.as_str().parse::<u64>().unwrap())
                    {
                        new_flags.channel_id = Some(ChannelId(channel));
                    }
                }
            }
        }

        new_flags
    }
}
