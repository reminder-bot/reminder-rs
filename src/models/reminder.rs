use serenity::{
    client::Context,
    model::id::{ChannelId, GuildId, UserId},
};

use chrono::NaiveDateTime;

use crate::{
    consts::{DAY, HOUR, MINUTE, REGEX_CHANNEL},
    SQLPool,
};

use num_integer::Integer;

fn longhand_displacement(seconds: u64) -> String {
    let (days, seconds) = seconds.div_rem(&DAY);
    let (hours, seconds) = seconds.div_rem(&HOUR);
    let (minutes, seconds) = seconds.div_rem(&MINUTE);

    let mut sections = vec![];

    for (var, name) in [days, hours, minutes, seconds]
        .iter()
        .zip(["days", "hours", "minutes", "seconds"].iter())
    {
        if *var > 0 {
            sections.push(format!("{} {}", var, name));
        }
    }

    sections.join(", ")
}

pub struct Reminder {
    pub id: u32,
    pub uid: String,
    pub channel: u64,
    pub utc_time: NaiveDateTime,
    pub interval: Option<u32>,
    pub expires: NaiveDateTime,
    pub enabled: bool,
    pub content: String,
    pub embed_description: String,
    pub set_by: Option<u64>,
}

impl Reminder {
    pub async fn from_channel<C: Into<ChannelId>>(
        ctx: &Context,
        channel_id: C,
        flags: &LookFlags,
    ) -> Vec<Self> {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

        let enabled = if flags.show_disabled { "0,1" } else { "1" };
        let channel_id = channel_id.into();

        sqlx::query_as_unchecked!(
            Self,
            "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval,
    reminders.expires,
    reminders.enabled,
    reminders.content,
    reminders.embed_description,
    users.user AS set_by
FROM
    reminders
INNER JOIN
    channels
ON
    reminders.channel_id = channels.id
LEFT JOIN
    users
ON
    reminders.set_by = users.id
WHERE
    channels.channel = ? AND
    FIND_IN_SET(reminders.enabled, ?)
ORDER BY
    reminders.utc_time
LIMIT
    ?
            ",
            channel_id.as_u64(),
            enabled,
            flags.limit
        )
        .fetch_all(&pool)
        .await
        .unwrap()
    }

    pub async fn from_guild(ctx: &Context, guild_id: Option<GuildId>, user: UserId) -> Vec<Self> {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

        if let Some(guild_id) = guild_id {
            let guild_opt = guild_id.to_guild_cached(&ctx).await;

            if let Some(guild) = guild_opt {
                let channels = guild
                    .channels
                    .keys()
                    .into_iter()
                    .map(|k| k.as_u64().to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                sqlx::query_as_unchecked!(
                    Self,
                    "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval,
    reminders.expires,
    reminders.enabled,
    reminders.content,
    reminders.embed_description,
    users.user AS set_by
FROM
    reminders
LEFT JOIN
    channels
ON
    channels.id = reminders.channel_id
LEFT JOIN
    users
ON
    reminders.set_by = users.id
WHERE
    FIND_IN_SET(channels.channel, ?)
                ",
                    channels
                )
                .fetch_all(&pool)
                .await
            } else {
                sqlx::query_as_unchecked!(
                    Self,
                    "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval,
    reminders.expires,
    reminders.enabled,
    reminders.content,
    reminders.embed_description,
    users.user AS set_by
FROM
    reminders
LEFT JOIN
    channels
ON
    channels.id = reminders.channel_id
LEFT JOIN
    users
ON
    reminders.set_by = users.id
WHERE
    channels.guild_id = (SELECT id FROM guilds WHERE guild = ?)
                ",
                    guild_id.as_u64()
                )
                .fetch_all(&pool)
                .await
            }
        } else {
            sqlx::query_as_unchecked!(
                Self,
                "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval,
    reminders.expires,
    reminders.enabled,
    reminders.content,
    reminders.embed_description,
    users.user AS set_by
FROM
    reminders
INNER JOIN
    channels
ON
    channels.id = reminders.channel_id
LEFT JOIN
    users
ON
    reminders.set_by = users.id
WHERE
    channels.id = (SELECT dm_channel FROM users WHERE user = ?)
            ",
                user.as_u64()
            )
            .fetch_all(&pool)
            .await
        }
        .unwrap()
    }

    pub fn display_content(&self) -> &str {
        if self.content.is_empty() {
            &self.embed_description
        } else {
            &self.content
        }
    }

    pub fn display(&self, flags: &LookFlags, inter: &str) -> String {
        let time_display = match flags.time_display {
            TimeDisplayType::Absolute => format!("<t:{}>", self.utc_time.timestamp()),

            TimeDisplayType::Relative => format!("<t:{}:R>", self.utc_time.timestamp()),
        };

        if let Some(interval) = self.interval {
            format!(
                "'{}' *{}* **{}**, repeating every **{}** (set by {})",
                self.display_content(),
                &inter,
                time_display,
                longhand_displacement(interval as u64),
                self.set_by
                    .map(|i| format!("<@{}>", i))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!(
                "'{}' *{}* **{}** (set by {})",
                self.display_content(),
                &inter,
                time_display,
                self.set_by
                    .map(|i| format!("<@{}>", i))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        }
    }
}

enum TimeDisplayType {
    Absolute,
    Relative,
}

pub struct LookFlags {
    pub limit: u16,
    pub show_disabled: bool,
    pub channel_id: Option<ChannelId>,
    time_display: TimeDisplayType,
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
                        .captures(&arg)
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
