pub mod builder;
pub mod content;
pub mod errors;
mod helper;
pub mod look_flags;

use std::hash::{Hash, Hasher};

use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use poise::{
    serenity::model::id::{ChannelId, GuildId, UserId},
    serenity_prelude::Cache,
};
use sqlx::Executor;

use crate::{
    models::reminder::look_flags::{LookFlags, TimeDisplayType},
    Database,
};

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: u32,
    pub uid: String,
    pub channel: u64,
    pub utc_time: NaiveDateTime,
    pub interval_seconds: Option<u32>,
    pub interval_months: Option<u32>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    pub content: String,
    pub embed_description: String,
    pub set_by: Option<u64>,
}

impl Hash for Reminder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uid.hash(state);
    }
}

impl PartialEq<Self> for Reminder {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

impl Eq for Reminder {}

impl Reminder {
    pub async fn from_uid(pool: impl Executor<'_, Database = Database>, uid: &str) -> Option<Self> {
        sqlx::query_as_unchecked!(
            Self,
            "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval_seconds,
    reminders.interval_months,
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
    reminders.uid = ?
            ",
            uid
        )
        .fetch_one(pool)
        .await
        .ok()
    }

    pub async fn from_id(pool: impl Executor<'_, Database = Database>, id: u32) -> Option<Self> {
        sqlx::query_as_unchecked!(
            Self,
            "
SELECT
    reminders.id,
    reminders.uid,
    channels.channel,
    reminders.utc_time,
    reminders.interval_seconds,
    reminders.interval_months,
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
    reminders.id = ?
            ",
            id
        )
        .fetch_one(pool)
        .await
        .ok()
    }

    pub async fn from_channel<C: Into<ChannelId>>(
        pool: impl Executor<'_, Database = Database>,
        channel_id: C,
        flags: &LookFlags,
    ) -> Vec<Self> {
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
    reminders.interval_seconds,
    reminders.interval_months,
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
            ",
            channel_id.as_u64(),
            enabled,
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    pub async fn from_guild(
        cache: impl AsRef<Cache>,
        pool: impl Executor<'_, Database = Database>,
        guild_id: Option<GuildId>,
        user: UserId,
    ) -> Vec<Self> {
        if let Some(guild_id) = guild_id {
            let guild_opt = guild_id.to_guild_cached(cache);

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
    reminders.interval_seconds,
    reminders.interval_months,
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
                .fetch_all(pool)
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
    reminders.interval_seconds,
    reminders.interval_months,
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
                .fetch_all(pool)
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
    reminders.interval_seconds,
    reminders.interval_months,
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
            .fetch_all(pool)
            .await
        }
        .unwrap()
    }

    pub async fn delete(
        &self,
        db: impl Executor<'_, Database = Database>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM reminders WHERE uid = ?", self.uid).execute(db).await.map(|_| ())
    }

    pub fn display_content(&self) -> &str {
        if self.content.is_empty() {
            &self.embed_description
        } else {
            &self.content
        }
    }

    pub fn display_del(&self, count: usize, timezone: &Tz) -> String {
        format!(
            "**{}**: '{}' *<#{}>* at **{}**",
            count + 1,
            self.display_content(),
            self.channel,
            timezone.timestamp(self.utc_time.timestamp(), 0).format("%Y-%m-%d %H:%M:%S")
        )
    }

    pub fn display(&self, flags: &LookFlags, timezone: &Tz) -> String {
        let time_display = match flags.time_display {
            TimeDisplayType::Absolute => timezone
                .timestamp(self.utc_time.timestamp(), 0)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),

            TimeDisplayType::Relative => format!("<t:{}:R>", self.utc_time.timestamp()),
        };

        if self.interval_seconds.is_some() || self.interval_months.is_some() {
            format!(
                "'{}' *occurs next at* **{}**, repeating (set by {})",
                self.display_content(),
                time_display,
                self.set_by.map(|i| format!("<@{}>", i)).unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!(
                "'{}' *occurs next at* **{}** (set by {})",
                self.display_content(),
                time_display,
                self.set_by.map(|i| format!("<@{}>", i)).unwrap_or_else(|| "unknown".to_string())
            )
        }
    }
}
