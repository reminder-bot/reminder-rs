use chrono::Duration;
use chrono_tz::Tz;
use lazy_static::lazy_static;
use log::{error, info, warn};
use num_integer::Integer;
use regex::{Captures, Regex};
use serenity::{
    builder::CreateEmbed,
    http::{CacheHttp, Http, StatusCode},
    model::{
        channel::{Channel, Embed as SerenityEmbed},
        id::ChannelId,
        webhook::Webhook,
    },
    Error, Result,
};
use sqlx::{
    types::chrono::{NaiveDateTime, Utc},
    Executor,
};

use crate::Database;

lazy_static! {
    pub static ref TIMEFROM_REGEX: Regex =
        Regex::new(r#"<<timefrom:(?P<time>\d+):(?P<format>.+)?>>"#).unwrap();
    pub static ref TIMENOW_REGEX: Regex =
        Regex::new(r#"<<timenow:(?P<timezone>(?:\w|/|_)+):(?P<format>.+)?>>"#).unwrap();
}

fn fmt_displacement(format: &str, seconds: u64) -> String {
    let mut seconds = seconds;
    let mut days: u64 = 0;
    let mut hours: u64 = 0;
    let mut minutes: u64 = 0;

    for (rep, time_type, div) in
        [("%d", &mut days, 86400), ("%h", &mut hours, 3600), ("%m", &mut minutes, 60)].iter_mut()
    {
        if format.contains(*rep) {
            let (divided, new_seconds) = seconds.div_rem(&div);

            **time_type = divided;
            seconds = new_seconds;
        }
    }

    format
        .replace("%s", &seconds.to_string())
        .replace("%m", &minutes.to_string())
        .replace("%h", &hours.to_string())
        .replace("%d", &days.to_string())
}

pub fn substitute(string: &str) -> String {
    let new = TIMEFROM_REGEX.replace(string, |caps: &Captures| {
        let final_time = caps.name("time").unwrap().as_str();
        let format = caps.name("format").unwrap().as_str();

        if let Ok(final_time) = final_time.parse::<i64>() {
            let dt = NaiveDateTime::from_timestamp(final_time, 0);
            let now = Utc::now().naive_utc();

            let difference = {
                if now < dt {
                    dt - Utc::now().naive_utc()
                } else {
                    Utc::now().naive_utc() - dt
                }
            };

            fmt_displacement(format, difference.num_seconds() as u64)
        } else {
            String::new()
        }
    });

    TIMENOW_REGEX
        .replace(&new, |caps: &Captures| {
            let timezone = caps.name("timezone").unwrap().as_str();

            println!("{}", timezone);

            if let Ok(tz) = timezone.parse::<Tz>() {
                let format = caps.name("format").unwrap().as_str();
                let now = Utc::now().with_timezone(&tz);

                now.format(format).to_string()
            } else {
                String::new()
            }
        })
        .to_string()
}

struct Embed {
    inner: EmbedInner,
    fields: Vec<EmbedField>,
}

struct EmbedInner {
    title: String,
    description: String,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
    footer: String,
    footer_url: Option<String>,
    author: String,
    author_url: Option<String>,
    color: u32,
}

struct EmbedField {
    title: String,
    value: String,
    inline: bool,
}

impl Embed {
    pub async fn from_id(
        pool: impl Executor<'_, Database = Database> + Copy,
        id: u32,
    ) -> Option<Self> {
        let mut inner = sqlx::query_as_unchecked!(
            EmbedInner,
            "
SELECT
    `embed_title` AS title,
    `embed_description` AS description,
    `embed_image_url` AS image_url,
    `embed_thumbnail_url` AS thumbnail_url,
    `embed_footer` AS footer,
    `embed_footer_url` AS footer_url,
    `embed_author` AS author,
    `embed_author_url` AS author_url,
    `embed_color` AS color
FROM
    reminders
WHERE
    `id` = ?
            ",
            id
        )
        .fetch_one(pool)
        .await
        .unwrap();

        inner.title = substitute(&inner.title);
        inner.description = substitute(&inner.description);
        inner.footer = substitute(&inner.footer);

        let mut fields = sqlx::query_as_unchecked!(
            EmbedField,
            "
SELECT
    title,
    value,
    inline
FROM
    embed_fields
WHERE
    reminder_id = ?
            ",
            id
        )
        .fetch_all(pool)
        .await
        .unwrap();

        fields.iter_mut().for_each(|mut field| {
            field.title = substitute(&field.title);
            field.value = substitute(&field.value);
        });

        let e = Embed { inner, fields };

        if e.has_content() {
            Some(e)
        } else {
            None
        }
    }

    pub fn has_content(&self) -> bool {
        if self.inner.title.is_empty()
            && self.inner.description.is_empty()
            && self.inner.image_url.is_none()
            && self.inner.thumbnail_url.is_none()
            && self.inner.footer.is_empty()
            && self.inner.footer_url.is_none()
            && self.inner.author.is_empty()
            && self.inner.author_url.is_none()
            && self.fields.is_empty()
        {
            false
        } else {
            true
        }
    }
}

impl Into<CreateEmbed> for Embed {
    fn into(self) -> CreateEmbed {
        let mut c = CreateEmbed::default();

        c.title(&self.inner.title)
            .description(&self.inner.description)
            .color(self.inner.color)
            .author(|a| {
                a.name(&self.inner.author);

                if let Some(author_icon) = &self.inner.author_url {
                    a.icon_url(author_icon);
                }

                a
            })
            .footer(|f| {
                f.text(&self.inner.footer);

                if let Some(footer_icon) = &self.inner.footer_url {
                    f.icon_url(footer_icon);
                }

                f
            });

        for field in &self.fields {
            c.field(&field.title, &field.value, field.inline);
        }

        if let Some(image_url) = &self.inner.image_url {
            c.image(image_url);
        }

        if let Some(thumbnail_url) = &self.inner.thumbnail_url {
            c.thumbnail(thumbnail_url);
        }

        c
    }
}

#[derive(Debug)]
pub struct Reminder {
    id: u32,

    channel_id: u64,
    webhook_id: Option<u64>,
    webhook_token: Option<String>,

    channel_paused: bool,
    channel_paused_until: Option<NaiveDateTime>,
    enabled: bool,

    tts: bool,
    pin: bool,
    content: String,
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,

    utc_time: NaiveDateTime,
    timezone: String,
    restartable: bool,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_months: Option<u32>,

    avatar: Option<String>,
    username: Option<String>,
}

impl Reminder {
    pub async fn fetch_reminders(pool: impl Executor<'_, Database = Database> + Copy) -> Vec<Self> {
        sqlx::query_as_unchecked!(
            Reminder,
            "
SELECT
    reminders.`id` AS id,

    channels.`channel` AS channel_id,
    channels.`webhook_id` AS webhook_id,
    channels.`webhook_token` AS webhook_token,

    channels.`paused` AS channel_paused,
    channels.`paused_until` AS channel_paused_until,
    reminders.`enabled` AS enabled,

    reminders.`tts` AS tts,
    reminders.`pin` AS pin,
    reminders.`content` AS content,
    reminders.`attachment` AS attachment,
    reminders.`attachment_name` AS attachment_name,

    reminders.`utc_time` AS 'utc_time',
    reminders.`timezone` AS timezone,
    reminders.`restartable` AS restartable,
    reminders.`expires` AS expires,
    reminders.`interval_seconds` AS 'interval_seconds',
    reminders.`interval_months` AS 'interval_months',

    reminders.`avatar` AS avatar,
    reminders.`username` AS username
FROM
    reminders
INNER JOIN
    channels
ON
    reminders.channel_id = channels.id
WHERE
    reminders.`utc_time` < NOW()
            ",
        )
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|mut rem| {
            rem.content = substitute(&rem.content);

            rem
        })
        .collect::<Vec<Self>>()
    }

    async fn reset_webhook(&self, pool: impl Executor<'_, Database = Database> + Copy) {
        let _ = sqlx::query!(
            "
UPDATE channels SET webhook_id = NULL, webhook_token = NULL WHERE channel = ?
            ",
            self.channel_id
        )
        .execute(pool)
        .await;
    }

    async fn refresh(&self, pool: impl Executor<'_, Database = Database> + Copy) {
        if self.interval_seconds.is_some() || self.interval_months.is_some() {
            let now = Utc::now().naive_local();
            let mut updated_reminder_time = self.utc_time;

            if let Some(interval) = self.interval_months {
                let row = sqlx::query!(
                    // use the second date_add to force return value to datetime
                    "SELECT DATE_ADD(DATE_ADD(?, INTERVAL ? MONTH), INTERVAL 0 SECOND) AS new_time",
                    updated_reminder_time,
                    interval
                )
                .fetch_one(pool)
                .await
                .unwrap();

                updated_reminder_time = row.new_time.unwrap();
            }

            if let Some(interval) = self.interval_seconds {
                while updated_reminder_time < now {
                    updated_reminder_time += Duration::seconds(interval as i64);
                }
            }

            if self.expires.map_or(false, |expires| {
                NaiveDateTime::from_timestamp(updated_reminder_time.timestamp(), 0) > expires
            }) {
                self.force_delete(pool).await;
            } else {
                sqlx::query!(
                    "
UPDATE reminders SET `utc_time` = ? WHERE `id` = ?
                    ",
                    updated_reminder_time,
                    self.id
                )
                .execute(pool)
                .await
                .expect(&format!("Could not update time on Reminder {}", self.id));
            }
        } else {
            self.force_delete(pool).await;
        }
    }

    async fn force_delete(&self, pool: impl Executor<'_, Database = Database> + Copy) {
        sqlx::query!(
            "
DELETE FROM reminders WHERE `id` = ?
            ",
            self.id
        )
        .execute(pool)
        .await
        .expect(&format!("Could not delete Reminder {}", self.id));
    }

    async fn pin_message<M: Into<u64>>(&self, message_id: M, http: impl AsRef<Http>) {
        let _ = http.as_ref().pin_message(self.channel_id, message_id.into(), None).await;
    }

    pub async fn send(
        &self,
        pool: impl Executor<'_, Database = Database> + Copy,
        cache_http: impl CacheHttp,
    ) {
        async fn send_to_channel(
            cache_http: impl CacheHttp,
            reminder: &Reminder,
            embed: Option<CreateEmbed>,
        ) -> Result<()> {
            let channel = ChannelId(reminder.channel_id).to_channel(&cache_http).await;

            match channel {
                Ok(Channel::Guild(channel)) => {
                    match channel
                        .send_message(&cache_http, |m| {
                            m.content(&reminder.content).tts(reminder.tts);

                            if let (Some(attachment), Some(name)) =
                                (&reminder.attachment, &reminder.attachment_name)
                            {
                                m.add_file((attachment as &[u8], name.as_str()));
                            }

                            if let Some(embed) = embed {
                                m.set_embed(embed);
                            }

                            m
                        })
                        .await
                    {
                        Ok(m) => {
                            if reminder.pin {
                                reminder.pin_message(m.id, cache_http.http()).await;
                            }

                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                Ok(Channel::Private(channel)) => {
                    match channel
                        .send_message(&cache_http.http(), |m| {
                            m.content(&reminder.content).tts(reminder.tts);

                            if let (Some(attachment), Some(name)) =
                                (&reminder.attachment, &reminder.attachment_name)
                            {
                                m.add_file((attachment as &[u8], name.as_str()));
                            }

                            if let Some(embed) = embed {
                                m.set_embed(embed);
                            }

                            m
                        })
                        .await
                    {
                        Ok(m) => {
                            if reminder.pin {
                                reminder.pin_message(m.id, cache_http.http()).await;
                            }

                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
                _ => Err(Error::Other("Channel not of valid type")),
            }
        }

        async fn send_to_webhook(
            cache_http: impl CacheHttp,
            reminder: &Reminder,
            webhook: Webhook,
            embed: Option<CreateEmbed>,
        ) -> Result<()> {
            match webhook
                .execute(&cache_http.http(), reminder.pin || reminder.restartable, |w| {
                    w.content(&reminder.content).tts(reminder.tts);

                    if let Some(username) = &reminder.username {
                        w.username(username);
                    }

                    if let Some(avatar) = &reminder.avatar {
                        w.avatar_url(avatar);
                    }

                    if let (Some(attachment), Some(name)) =
                        (&reminder.attachment, &reminder.attachment_name)
                    {
                        w.add_file((attachment as &[u8], name.as_str()));
                    }

                    if let Some(embed) = embed {
                        w.embeds(vec![SerenityEmbed::fake(|c| {
                            *c = embed;
                            c
                        })]);
                    }

                    w
                })
                .await
            {
                Ok(m) => {
                    if reminder.pin {
                        if let Some(message) = m {
                            reminder.pin_message(message.id, cache_http.http()).await;
                        }
                    }

                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        if self.enabled
            && !(self.channel_paused
                && self
                    .channel_paused_until
                    .map_or(true, |inner| inner >= Utc::now().naive_local()))
        {
            let _ = sqlx::query!(
                "
UPDATE `channels` SET paused = 0, paused_until = NULL WHERE `channel` = ?
                ",
                self.channel_id
            )
            .execute(pool)
            .await;

            let embed = Embed::from_id(pool, self.id).await.map(|e| e.into());

            let result = if let (Some(webhook_id), Some(webhook_token)) =
                (self.webhook_id, &self.webhook_token)
            {
                let webhook_res =
                    cache_http.http().get_webhook_with_token(webhook_id, webhook_token).await;

                if let Ok(webhook) = webhook_res {
                    send_to_webhook(cache_http, &self, webhook, embed).await
                } else {
                    warn!("Webhook vanished: {:?}", webhook_res);

                    self.reset_webhook(pool).await;
                    send_to_channel(cache_http, &self, embed).await
                }
            } else {
                send_to_channel(cache_http, &self, embed).await
            };

            if let Err(e) = result {
                error!("Error sending {:?}: {:?}", self, e);

                if let Error::Http(error) = e {
                    if error.status_code() == Some(StatusCode::from_u16(404).unwrap()) {
                        error!("Seeing channel is deleted. Removing reminder");
                        self.force_delete(pool).await;
                    } else {
                        self.refresh(pool).await;
                    }
                } else {
                    self.refresh(pool).await;
                }
            } else {
                self.refresh(pool).await;
            }
        } else {
            info!("Reminder {} is paused", self.id);

            self.refresh(pool).await;
        }
    }
}
