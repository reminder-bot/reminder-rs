pub mod builder;
pub mod content;
pub mod errors;
mod helper;
pub mod look_flags;

use std::{
    convert::{TryFrom, TryInto},
    env,
};

use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use ring::hmac;
use serenity::{
    client::Context,
    model::id::{ChannelId, GuildId, UserId},
};
use sqlx::MySqlPool;

use crate::{
    models::reminder::{
        errors::InteractionError,
        helper::longhand_displacement,
        look_flags::{LookFlags, TimeDisplayType},
    },
    SQLPool,
};

#[derive(Clone, Copy)]
pub enum ReminderAction {
    Delete,
}

impl ToString for ReminderAction {
    fn to_string(&self) -> String {
        match self {
            Self::Delete => String::from("del"),
        }
    }
}

impl TryFrom<&str> for ReminderAction {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "del" => Ok(Self::Delete),

            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Reminder {
    pub id: u32,
    pub uid: String,
    pub channel: u64,
    pub utc_time: NaiveDateTime,
    pub interval: Option<u32>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    pub content: String,
    pub embed_description: String,
    pub set_by: Option<u64>,
}

impl Reminder {
    pub async fn from_uid(pool: &MySqlPool, uid: String) -> Option<Self> {
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
    reminders.uid = ?
            ",
            uid
        )
        .fetch_one(pool)
        .await
        .ok()
    }

    pub async fn from_id(ctx: &Context, id: u32) -> Option<Self> {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

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
    reminders.id = ?
            ",
            id
        )
        .fetch_one(&pool)
        .await
        .ok()
    }

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
            let guild_opt = guild_id.to_guild_cached(&ctx);

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

    pub fn display(&self, flags: &LookFlags, timezone: &Tz) -> String {
        let time_display = match flags.time_display {
            TimeDisplayType::Absolute => timezone
                .timestamp(self.utc_time.timestamp(), 0)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),

            TimeDisplayType::Relative => format!("<t:{}:R>", self.utc_time.timestamp()),
        };

        if let Some(interval) = self.interval {
            format!(
                "'{}' *occurs next at* **{}**, repeating every **{}** (set by {})",
                self.display_content(),
                time_display,
                longhand_displacement(interval as u64),
                self.set_by
                    .map(|i| format!("<@{}>", i))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!(
                "'{}' *occurs next at* **{}** (set by {})",
                self.display_content(),
                time_display,
                self.set_by
                    .map(|i| format!("<@{}>", i))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        }
    }

    pub async fn from_interaction<U: Into<u64>>(
        ctx: &Context,
        member_id: U,
        payload: String,
    ) -> Result<(Self, ReminderAction), InteractionError> {
        let sections = payload.split('.').collect::<Vec<&str>>();

        if sections.len() != 3 {
            Err(InteractionError::InvalidFormat)
        } else {
            let action = ReminderAction::try_from(sections[0])
                .map_err(|_| InteractionError::InvalidAction)?;

            let reminder_id = u32::from_le_bytes(
                base64::decode(sections[1])
                    .map_err(|_| InteractionError::InvalidBase64)?
                    .try_into()
                    .map_err(|_| InteractionError::InvalidSize)?,
            );

            if let Some(reminder) = Self::from_id(ctx, reminder_id).await {
                if reminder.signed_action(member_id, action) == payload {
                    Ok((reminder, action))
                } else {
                    Err(InteractionError::SignatureMismatch)
                }
            } else {
                Err(InteractionError::NoReminder)
            }
        }
    }

    pub fn signed_action<U: Into<u64>>(&self, member_id: U, action: ReminderAction) -> String {
        let s_key = hmac::Key::new(
            hmac::HMAC_SHA256,
            env::var("SECRET_KEY")
                .expect("No SECRET_KEY provided")
                .as_bytes(),
        );

        let mut context = hmac::Context::with_key(&s_key);

        context.update(&self.id.to_le_bytes());
        context.update(&member_id.into().to_le_bytes());

        let signature = context.sign();

        format!(
            "{}.{}.{}",
            action.to_string(),
            base64::encode(self.id.to_le_bytes()),
            base64::encode(&signature)
        )
    }

    pub async fn delete(&self, ctx: &Context) {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

        sqlx::query!(
            "
DELETE FROM reminders WHERE id = ?
            ",
            self.id
        )
        .execute(&pool)
        .await
        .unwrap();
    }
}
