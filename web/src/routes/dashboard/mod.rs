use std::collections::HashMap;

use chrono::{naive::NaiveDateTime, Utc};
use rand::{rngs::OsRng, seq::IteratorRandom};
use rocket::{
    http::CookieJar,
    response::Redirect,
    serde::json::{json, Value as JsonValue},
};
use rocket_dyn_templates::Template;
use serde::{Deserialize, Deserializer, Serialize};
use serenity::{
    client::Context,
    http::Http,
    model::id::{ChannelId, GuildId, UserId},
};
use sqlx::{types::Json, Executor, MySql, Pool};

use crate::{
    check_guild_subscription, check_subscription,
    consts::{
        CHARACTERS, DAY, DEFAULT_AVATAR, MAX_CONTENT_LENGTH, MAX_EMBED_AUTHOR_LENGTH,
        MAX_EMBED_DESCRIPTION_LENGTH, MAX_EMBED_FIELDS, MAX_EMBED_FIELD_TITLE_LENGTH,
        MAX_EMBED_FIELD_VALUE_LENGTH, MAX_EMBED_FOOTER_LENGTH, MAX_EMBED_TITLE_LENGTH,
        MAX_URL_LENGTH, MAX_USERNAME_LENGTH, MIN_INTERVAL,
    },
    Database, Error,
};

pub mod export;
pub mod guild;
pub mod user;

pub type JsonResult = Result<JsonValue, JsonValue>;
type Unset<T> = Option<T>;

fn name_default() -> String {
    "Reminder".to_string()
}

fn template_name_default() -> String {
    "Template".to_string()
}

fn channel_default() -> u64 {
    0
}

fn id_default() -> u32 {
    0
}

fn interval_default() -> Unset<Option<u32>> {
    None
}

fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[derive(Serialize, Deserialize)]
pub struct ReminderTemplate {
    #[serde(default = "id_default")]
    id: u32,
    #[serde(default = "id_default")]
    guild_id: u32,
    #[serde(default = "template_name_default")]
    name: String,
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
    avatar: Option<String>,
    content: String,
    embed_author: String,
    embed_author_url: Option<String>,
    embed_color: u32,
    embed_description: String,
    embed_footer: String,
    embed_footer_url: Option<String>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_title: String,
    embed_fields: Option<Json<Vec<EmbedField>>>,
    tts: bool,
    username: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ReminderTemplateCsv {
    #[serde(default = "template_name_default")]
    name: String,
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
    avatar: Option<String>,
    content: String,
    embed_author: String,
    embed_author_url: Option<String>,
    embed_color: u32,
    embed_description: String,
    embed_footer: String,
    embed_footer_url: Option<String>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_title: String,
    embed_fields: Option<String>,
    tts: bool,
    username: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteReminderTemplate {
    id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct EmbedField {
    title: String,
    value: String,
    inline: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    #[serde(with = "base64s")]
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
    avatar: Option<String>,
    #[serde(with = "string")]
    channel: u64,
    content: String,
    embed_author: String,
    embed_author_url: Option<String>,
    embed_color: u32,
    embed_description: String,
    embed_footer: String,
    embed_footer_url: Option<String>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_title: String,
    embed_fields: Option<Json<Vec<EmbedField>>>,
    enabled: bool,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_days: Option<u32>,
    interval_months: Option<u32>,
    #[serde(default = "name_default")]
    name: String,
    restartable: bool,
    tts: bool,
    #[serde(default)]
    uid: String,
    username: Option<String>,
    utc_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct ReminderCsv {
    #[serde(with = "base64s")]
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
    avatar: Option<String>,
    channel: String,
    content: String,
    embed_author: String,
    embed_author_url: Option<String>,
    embed_color: u32,
    embed_description: String,
    embed_footer: String,
    embed_footer_url: Option<String>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_title: String,
    embed_fields: Option<String>,
    enabled: bool,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_days: Option<u32>,
    interval_months: Option<u32>,
    #[serde(default = "name_default")]
    name: String,
    restartable: bool,
    tts: bool,
    username: Option<String>,
    utc_time: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct PatchReminder {
    uid: String,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    attachment: Unset<Option<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    attachment_name: Unset<Option<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    avatar: Unset<Option<String>>,
    #[serde(default = "channel_default")]
    #[serde(with = "string")]
    channel: u64,
    #[serde(default)]
    content: Unset<String>,
    #[serde(default)]
    embed_author: Unset<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    embed_author_url: Unset<Option<String>>,
    #[serde(default)]
    embed_color: Unset<u32>,
    #[serde(default)]
    embed_description: Unset<String>,
    #[serde(default)]
    embed_footer: Unset<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    embed_footer_url: Unset<Option<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    embed_image_url: Unset<Option<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    embed_thumbnail_url: Unset<Option<String>>,
    #[serde(default)]
    embed_title: Unset<String>,
    #[serde(default)]
    embed_fields: Unset<Json<Vec<EmbedField>>>,
    #[serde(default)]
    enabled: Unset<bool>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    expires: Unset<Option<NaiveDateTime>>,
    #[serde(default = "interval_default")]
    #[serde(deserialize_with = "deserialize_optional_field")]
    interval_seconds: Unset<Option<u32>>,
    #[serde(default = "interval_default")]
    #[serde(deserialize_with = "deserialize_optional_field")]
    interval_days: Unset<Option<u32>>,
    #[serde(default = "interval_default")]
    #[serde(deserialize_with = "deserialize_optional_field")]
    interval_months: Unset<Option<u32>>,
    #[serde(default)]
    name: Unset<String>,
    #[serde(default)]
    restartable: Unset<bool>,
    #[serde(default)]
    tts: Unset<bool>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_field")]
    username: Unset<Option<String>>,
    #[serde(default)]
    utc_time: Unset<NaiveDateTime>,
}

impl PatchReminder {
    fn message_ok(&self) -> bool {
        self.content.as_ref().map_or(true, |c| c.len() <= MAX_CONTENT_LENGTH)
            && self.embed_author.as_ref().map_or(true, |c| c.len() <= MAX_EMBED_AUTHOR_LENGTH)
            && self
                .embed_description
                .as_ref()
                .map_or(true, |c| c.len() <= MAX_EMBED_DESCRIPTION_LENGTH)
            && self.embed_footer.as_ref().map_or(true, |c| c.len() <= MAX_EMBED_FOOTER_LENGTH)
            && self.embed_title.as_ref().map_or(true, |c| c.len() <= MAX_EMBED_TITLE_LENGTH)
            && self.embed_fields.as_ref().map_or(true, |c| {
                c.0.len() <= MAX_EMBED_FIELDS
                    && c.0.iter().all(|f| {
                        f.title.len() <= MAX_EMBED_FIELD_TITLE_LENGTH
                            && f.value.len() <= MAX_EMBED_FIELD_VALUE_LENGTH
                    })
            })
            && self
                .username
                .as_ref()
                .map_or(true, |c| c.as_ref().map_or(true, |v| v.len() <= MAX_USERNAME_LENGTH))
    }
}

pub fn generate_uid() -> String {
    let mut generator: OsRng = Default::default();

    (0..64)
        .map(|_| CHARACTERS.chars().choose(&mut generator).unwrap().to_owned().to_string())
        .collect::<Vec<String>>()
        .join("")
}

// https://github.com/serde-rs/json/issues/329#issuecomment-305608405
mod string {
    use std::{fmt::Display, str::FromStr};

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

mod base64s {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(opt) = value {
            serializer.collect_str(&base64::encode(opt))
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = Option::<String>::deserialize(deserializer)?;
        Some(string.map(|b| base64::decode(b).map_err(de::Error::custom))).flatten().transpose()
    }
}

#[derive(Deserialize)]
pub struct DeleteReminder {
    uid: String,
}

#[derive(Deserialize)]
pub struct ImportBody {
    body: String,
}

#[derive(Serialize, Deserialize)]
pub struct TodoCsv {
    value: String,
    channel_id: Option<String>,
}

pub async fn create_reminder(
    ctx: &Context,
    pool: &Pool<MySql>,
    guild_id: GuildId,
    user_id: UserId,
    reminder: Reminder,
) -> JsonResult {
    // validate channel
    let channel = ChannelId(reminder.channel).to_channel_cached(&ctx);
    let channel_exists = channel.is_some();

    let channel_matches_guild =
        channel.map_or(false, |c| c.guild().map_or(false, |c| c.guild_id == guild_id));

    if !channel_matches_guild || !channel_exists {
        warn!(
            "Error in `create_reminder`: channel {} not found for guild {} (channel exists: {})",
            reminder.channel, guild_id, channel_exists
        );

        return Err(json!({"error": "Channel not found"}));
    }

    let channel = create_database_channel(&ctx, ChannelId(reminder.channel), pool).await;

    if let Err(e) = channel {
        warn!("`create_database_channel` returned an error code: {:?}", e);

        return Err(
            json!({"error": "Failed to configure channel for reminders. Please check the bot permissions"}),
        );
    }

    let channel = channel.unwrap();

    // validate lengths
    check_length!(MAX_CONTENT_LENGTH, reminder.content);
    check_length!(MAX_EMBED_DESCRIPTION_LENGTH, reminder.embed_description);
    check_length!(MAX_EMBED_TITLE_LENGTH, reminder.embed_title);
    check_length!(MAX_EMBED_AUTHOR_LENGTH, reminder.embed_author);
    check_length!(MAX_EMBED_FOOTER_LENGTH, reminder.embed_footer);
    check_length_opt!(MAX_EMBED_FIELDS, reminder.embed_fields);
    if let Some(fields) = &reminder.embed_fields {
        for field in &fields.0 {
            check_length!(MAX_EMBED_FIELD_VALUE_LENGTH, field.value);
            check_length!(MAX_EMBED_FIELD_TITLE_LENGTH, field.title);
        }
    }
    check_length_opt!(MAX_USERNAME_LENGTH, reminder.username);
    check_length_opt!(
        MAX_URL_LENGTH,
        reminder.embed_footer_url,
        reminder.embed_thumbnail_url,
        reminder.embed_author_url,
        reminder.embed_image_url,
        reminder.avatar
    );

    // validate urls
    check_url_opt!(
        reminder.embed_footer_url,
        reminder.embed_thumbnail_url,
        reminder.embed_author_url,
        reminder.embed_image_url,
        reminder.avatar
    );

    // validate time and interval
    if reminder.utc_time < Utc::now().naive_utc() {
        return Err(json!({"error": "Time must be in the future"}));
    }
    if reminder.interval_seconds.is_some()
        || reminder.interval_days.is_some()
        || reminder.interval_months.is_some()
    {
        if reminder.interval_months.unwrap_or(0) * 30 * DAY as u32
            + reminder.interval_days.unwrap_or(0) * DAY as u32
            + reminder.interval_seconds.unwrap_or(0)
            < *MIN_INTERVAL
        {
            return Err(json!({"error": "Interval too short"}));
        }
    }

    // check patreon if necessary
    if reminder.interval_seconds.is_some()
        || reminder.interval_days.is_some()
        || reminder.interval_months.is_some()
    {
        if !check_guild_subscription(&ctx, guild_id).await
            && !check_subscription(&ctx, user_id).await
        {
            return Err(json!({"error": "Patreon is required to set intervals"}));
        }
    }

    // base64 decode error dropped here
    let attachment_data = reminder.attachment.as_ref().map(|s| base64::decode(s).ok()).flatten();
    let name = if reminder.name.is_empty() { name_default() } else { reminder.name.clone() };

    let new_uid = generate_uid();

    // write to db
    match sqlx::query!(
        "INSERT INTO reminders (
         uid,
         attachment,
         attachment_name,
         channel_id,
         avatar,
         content,
         embed_author,
         embed_author_url,
         embed_color,
         embed_description,
         embed_footer,
         embed_footer_url,
         embed_image_url,
         embed_thumbnail_url,
         embed_title,
         embed_fields,
         enabled,
         expires,
         interval_seconds,
         interval_days,
         interval_months,
         name,
         restartable,
         tts,
         username,
         `utc_time`
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        new_uid,
        attachment_data,
        reminder.attachment_name,
        channel,
        reminder.avatar,
        reminder.content,
        reminder.embed_author,
        reminder.embed_author_url,
        reminder.embed_color,
        reminder.embed_description,
        reminder.embed_footer,
        reminder.embed_footer_url,
        reminder.embed_image_url,
        reminder.embed_thumbnail_url,
        reminder.embed_title,
        reminder.embed_fields,
        reminder.enabled,
        reminder.expires,
        reminder.interval_seconds,
        reminder.interval_days,
        reminder.interval_months,
        name,
        reminder.restartable,
        reminder.tts,
        reminder.username,
        reminder.utc_time,
    )
    .execute(pool)
    .await
    {
        Ok(_) => sqlx::query_as_unchecked!(
            Reminder,
            "SELECT
             reminders.attachment,
             reminders.attachment_name,
             reminders.avatar,
             channels.channel,
             reminders.content,
             reminders.embed_author,
             reminders.embed_author_url,
             reminders.embed_color,
             reminders.embed_description,
             reminders.embed_footer,
             reminders.embed_footer_url,
             reminders.embed_image_url,
             reminders.embed_thumbnail_url,
             reminders.embed_title,
             reminders.embed_fields,
             reminders.enabled,
             reminders.expires,
             reminders.interval_seconds,
             reminders.interval_days,
             reminders.interval_months,
             reminders.name,
             reminders.restartable,
             reminders.tts,
             reminders.uid,
             reminders.username,
             reminders.utc_time
            FROM reminders
            LEFT JOIN channels ON channels.id = reminders.channel_id
            WHERE uid = ?",
            new_uid
        )
        .fetch_one(pool)
        .await
        .map(|r| Ok(json!(r)))
        .unwrap_or_else(|e| {
            warn!("Failed to complete SQL query: {:?}", e);

            Err(json!({"error": "Could not load reminder"}))
        }),

        Err(e) => {
            warn!("Error in `create_reminder`: Could not execute query: {:?}", e);

            Err(json!({"error": "Unknown error"}))
        }
    }
}

async fn create_database_channel(
    ctx: impl AsRef<Http>,
    channel: ChannelId,
    pool: impl Executor<'_, Database = Database> + Copy,
) -> Result<u32, crate::Error> {
    let row =
        sqlx::query!("SELECT webhook_token, webhook_id FROM channels WHERE channel = ?", channel.0)
            .fetch_one(pool)
            .await;

    match row {
        Ok(row) => {
            if row.webhook_token.is_none() || row.webhook_id.is_none() {
                let webhook = channel
                    .create_webhook_with_avatar(&ctx, "Reminder", DEFAULT_AVATAR.clone())
                    .await
                    .map_err(|e| Error::Serenity(e))?;

                sqlx::query!(
                    "UPDATE channels SET webhook_id = ?, webhook_token = ? WHERE channel = ?",
                    webhook.id.0,
                    webhook.token,
                    channel.0
                )
                .execute(pool)
                .await
                .map_err(|e| Error::SQLx(e))?;
            }

            Ok(())
        }

        Err(sqlx::Error::RowNotFound) => {
            // create webhook
            let webhook = channel
                .create_webhook_with_avatar(&ctx, "Reminder", DEFAULT_AVATAR.clone())
                .await
                .map_err(|e| Error::Serenity(e))?;

            // create database entry
            sqlx::query!(
                "INSERT INTO channels (
                 webhook_id,
                 webhook_token,
                 channel
                ) VALUES (?, ?, ?)",
                webhook.id.0,
                webhook.token,
                channel.0
            )
            .execute(pool)
            .await
            .map_err(|e| Error::SQLx(e))?;

            Ok(())
        }

        Err(e) => Err(Error::SQLx(e)),
    }?;

    let row = sqlx::query!("SELECT id FROM channels WHERE channel = ?", channel.0)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::SQLx(e))?;

    Ok(row.id)
}

#[get("/")]
pub async fn dashboard_home(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    if cookies.get_private("userid").is_some() {
        let map: HashMap<&str, String> = HashMap::new();
        Ok(Template::render("dashboard", &map))
    } else {
        Err(Redirect::to("/login/discord"))
    }
}

#[get("/<_>")]
pub async fn dashboard(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    if cookies.get_private("userid").is_some() {
        let map: HashMap<&str, String> = HashMap::new();
        Ok(Template::render("dashboard", &map))
    } else {
        Err(Redirect::to("/login/discord"))
    }
}
