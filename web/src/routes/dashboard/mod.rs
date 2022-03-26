use std::collections::HashMap;

use chrono::naive::NaiveDateTime;
use rand::{rngs::OsRng, seq::IteratorRandom};
use rocket::{http::CookieJar, response::Redirect};
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use serenity::{http::Http, model::id::ChannelId};
use sqlx::Executor;

use crate::{
    consts::{CHARACTERS, DEFAULT_AVATAR},
    Database, Error,
};

pub mod guild;
pub mod user;

type Unset<T> = Option<T>;

fn name_default() -> String {
    "Reminder".to_string()
}

fn channel_default() -> u64 {
    0
}

#[derive(Serialize, Deserialize)]
pub struct EmbedField {
    title: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonReminder {
    attachment: Option<String>,
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
    embed_fields: Vec<EmbedField>,
    enabled: bool,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_months: Option<u32>,
    #[serde(default = "name_default")]
    name: String,
    pin: bool,
    restartable: bool,
    tts: bool,
    #[serde(default)]
    uid: String,
    username: Option<String>,
    utc_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
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
    enabled: bool,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_months: Option<u32>,
    #[serde(default = "name_default")]
    name: String,
    pin: bool,
    restartable: bool,
    tts: bool,
    #[serde(default)]
    uid: String,
    username: Option<String>,
    utc_time: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct PatchReminder {
    uid: String,
    #[serde(default)]
    attachment: Unset<Option<String>>,
    #[serde(default)]
    attachment_name: Unset<Option<String>>,
    #[serde(default)]
    avatar: Unset<Option<String>>,
    #[serde(default = "channel_default")]
    #[serde(with = "string")]
    channel: u64,
    #[serde(default)]
    content: Unset<String>,
    #[serde(default)]
    embed_author: Unset<String>,
    #[serde(default)]
    embed_author_url: Unset<Option<String>>,
    #[serde(default)]
    embed_color: Unset<u32>,
    #[serde(default)]
    embed_description: Unset<String>,
    #[serde(default)]
    embed_footer: Unset<String>,
    #[serde(default)]
    embed_footer_url: Unset<Option<String>>,
    #[serde(default)]
    embed_image_url: Unset<Option<String>>,
    #[serde(default)]
    embed_thumbnail_url: Unset<Option<String>>,
    #[serde(default)]
    embed_title: Unset<String>,
    #[serde(default)]
    embed_fields: Unset<EmbedField>,
    #[serde(default)]
    enabled: Unset<bool>,
    #[serde(default)]
    expires: Unset<Option<NaiveDateTime>>,
    #[serde(default)]
    interval_seconds: Unset<Option<u32>>,
    #[serde(default)]
    interval_months: Unset<Option<u32>>,
    #[serde(default)]
    name: Unset<String>,
    #[serde(default)]
    pin: Unset<bool>,
    #[serde(default)]
    restartable: Unset<bool>,
    #[serde(default)]
    tts: Unset<bool>,
    #[serde(default)]
    username: Unset<Option<String>>,
    #[serde(default)]
    utc_time: Unset<NaiveDateTime>,
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

#[derive(Deserialize)]
pub struct DeleteReminder {
    uid: String,
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
                ) VALUES (
                 webhook_id = ?,
                 webhook_token = ?,
                 channel = ?
                )",
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
