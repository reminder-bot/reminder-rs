use std::{collections::HashSet, fmt::Display};

use chrono::{Duration, NaiveDateTime, Utc};
use chrono_tz::Tz;
use poise::serenity::{
    http::CacheHttp,
    model::{
        channel::GuildChannel,
        id::{ChannelId, GuildId, UserId},
        webhook::Webhook,
    },
    Result as SerenityResult,
};
use sqlx::MySqlPool;

use crate::{
    consts::{DAY, DEFAULT_AVATAR, MAX_TIME, MIN_INTERVAL},
    interval_parser::Interval,
    models::{
        channel_data::ChannelData,
        reminder::{content::Content, errors::ReminderError, helper::generate_uid, Reminder},
        user_data::UserData,
    },
    Context,
};

async fn create_webhook(
    ctx: impl CacheHttp,
    channel: GuildChannel,
    name: impl Display,
) -> SerenityResult<Webhook> {
    channel.create_webhook_with_avatar(ctx.http(), name, DEFAULT_AVATAR.clone()).await
}

#[derive(Hash, PartialEq, Eq)]
pub enum ReminderScope {
    User(u64),
    Channel(u64),
}

impl ReminderScope {
    pub fn mention(&self) -> String {
        match self {
            Self::User(id) => format!("<@{}>", id),
            Self::Channel(id) => format!("<#{}>", id),
        }
    }
}

pub struct ReminderBuilder {
    pool: MySqlPool,
    uid: String,
    channel: u32,
    utc_time: NaiveDateTime,
    timezone: String,
    interval_secs: Option<i64>,
    interval_months: Option<i64>,
    expires: Option<NaiveDateTime>,
    content: String,
    tts: bool,
    attachment_name: Option<String>,
    attachment: Option<Vec<u8>>,
    set_by: Option<u32>,
}

impl ReminderBuilder {
    pub async fn build(self) -> Result<Reminder, ReminderError> {
        let queried_time = sqlx::query!(
            "SELECT DATE_ADD(?, INTERVAL (SELECT nudge FROM channels WHERE id = ?) SECOND) AS `utc_time`",
            self.utc_time,
            self.channel,
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();

        match queried_time.utc_time {
            Some(utc_time) => {
                if utc_time < (Utc::now() - Duration::seconds(60)).naive_local() {
                    Err(ReminderError::PastTime)
                } else {
                    sqlx::query!(
                        "
INSERT INTO reminders (
    `uid`,
    `channel_id`,
    `utc_time`,
    `timezone`,
    `interval_seconds`,
    `interval_months`,
    `expires`,
    `content`,
    `tts`,
    `attachment_name`,
    `attachment`,
    `set_by`
) VALUES (
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    ?
)
            ",
                        self.uid,
                        self.channel,
                        utc_time,
                        self.timezone,
                        self.interval_secs,
                        self.interval_months,
                        self.expires,
                        self.content,
                        self.tts,
                        self.attachment_name,
                        self.attachment,
                        self.set_by
                    )
                    .execute(&self.pool)
                    .await
                    .unwrap();

                    Ok(Reminder::from_uid(&self.pool, &self.uid).await.unwrap())
                }
            }

            None => Err(ReminderError::LongTime),
        }
    }
}

pub struct MultiReminderBuilder<'a> {
    scopes: Vec<ReminderScope>,
    utc_time: NaiveDateTime,
    timezone: Tz,
    interval: Option<Interval>,
    expires: Option<NaiveDateTime>,
    content: Content,
    set_by: Option<u32>,
    ctx: &'a Context<'a>,
    guild_id: Option<GuildId>,
}

impl<'a> MultiReminderBuilder<'a> {
    pub fn new(ctx: &'a Context, guild_id: Option<GuildId>) -> Self {
        MultiReminderBuilder {
            scopes: vec![],
            utc_time: Utc::now().naive_utc(),
            timezone: Tz::UTC,
            interval: None,
            expires: None,
            content: Content::new(),
            set_by: None,
            ctx,
            guild_id,
        }
    }

    pub fn timezone(mut self, timezone: Tz) -> Self {
        self.timezone = timezone;

        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;

        self
    }

    pub fn time<T: Into<i64>>(mut self, time: T) -> Self {
        self.utc_time = NaiveDateTime::from_timestamp(time.into(), 0);

        self
    }

    pub fn expires<T: Into<i64>>(mut self, time: Option<T>) -> Self {
        if let Some(t) = time {
            self.expires = Some(NaiveDateTime::from_timestamp(t.into(), 0));
        } else {
            self.expires = None;
        }

        self
    }

    pub fn author(mut self, user: UserData) -> Self {
        self.set_by = Some(user.id);
        self.timezone = user.timezone();

        self
    }

    pub fn interval(mut self, interval: Option<Interval>) -> Self {
        self.interval = interval;

        self
    }

    pub fn set_scopes(&mut self, scopes: Vec<ReminderScope>) {
        self.scopes = scopes;
    }

    pub async fn build(self) -> (HashSet<ReminderError>, HashSet<(Reminder, ReminderScope)>) {
        let mut errors = HashSet::new();

        let mut ok_locs = HashSet::new();

        if self.interval.map_or(false, |i| ((i.sec + i.month * 30 * DAY) as i64) < *MIN_INTERVAL) {
            errors.insert(ReminderError::ShortInterval);
        } else if self.interval.map_or(false, |i| ((i.sec + i.month * 30 * DAY) as i64) > *MAX_TIME)
        {
            errors.insert(ReminderError::LongInterval);
        } else {
            for scope in self.scopes {
                let db_channel_id = match scope {
                    ReminderScope::User(user_id) => {
                        if let Ok(user) = UserId(user_id).to_user(&self.ctx.discord()).await {
                            let user_data = UserData::from_user(
                                &user,
                                &self.ctx.discord(),
                                &self.ctx.data().database,
                            )
                            .await
                            .unwrap();

                            if let Some(guild_id) = self.guild_id {
                                if guild_id.member(&self.ctx.discord(), user).await.is_err() {
                                    Err(ReminderError::InvalidTag)
                                } else {
                                    Ok(user_data.dm_channel)
                                }
                            } else {
                                Ok(user_data.dm_channel)
                            }
                        } else {
                            Err(ReminderError::InvalidTag)
                        }
                    }
                    ReminderScope::Channel(channel_id) => {
                        let channel =
                            ChannelId(channel_id).to_channel(&self.ctx.discord()).await.unwrap();

                        if let Some(guild_channel) = channel.clone().guild() {
                            if Some(guild_channel.guild_id) != self.guild_id {
                                Err(ReminderError::InvalidTag)
                            } else {
                                let mut channel_data =
                                    ChannelData::from_channel(&channel, &self.ctx.data().database)
                                        .await
                                        .unwrap();

                                if channel_data.webhook_id.is_none()
                                    || channel_data.webhook_token.is_none()
                                {
                                    match create_webhook(
                                        &self.ctx.discord(),
                                        guild_channel,
                                        "Reminder",
                                    )
                                    .await
                                    {
                                        Ok(webhook) => {
                                            channel_data.webhook_id =
                                                Some(webhook.id.as_u64().to_owned());
                                            channel_data.webhook_token = webhook.token;

                                            channel_data
                                                .commit_changes(&self.ctx.data().database)
                                                .await;

                                            Ok(channel_data.id)
                                        }

                                        Err(e) => Err(ReminderError::DiscordError(e.to_string())),
                                    }
                                } else {
                                    Ok(channel_data.id)
                                }
                            }
                        } else {
                            Err(ReminderError::InvalidTag)
                        }
                    }
                };

                match db_channel_id {
                    Ok(c) => {
                        let builder = ReminderBuilder {
                            pool: self.ctx.data().database.clone(),
                            uid: generate_uid(),
                            channel: c,
                            utc_time: self.utc_time,
                            timezone: self.timezone.to_string(),
                            interval_secs: self.interval.map(|i| i.sec as i64),
                            interval_months: self.interval.map(|i| i.month as i64),
                            expires: self.expires,
                            content: self.content.content.clone(),
                            tts: self.content.tts,
                            attachment_name: self.content.attachment_name.clone(),
                            attachment: self.content.attachment.clone(),
                            set_by: self.set_by,
                        };

                        match builder.build().await {
                            Ok(r) => {
                                ok_locs.insert((r, scope));
                            }
                            Err(e) => {
                                errors.insert(e);
                            }
                        }
                    }
                    Err(e) => {
                        errors.insert(e);
                    }
                }
            }
        }

        (errors, ok_locs)
    }
}
