use rocket::serde::json::{json, Json, Value as JsonValue};
use rocket::{http::CookieJar, State};

use reqwest::Client;

use serde::{Deserialize, Serialize};
use serenity::model::{
    id::{GuildId, RoleId},
    permissions::Permissions,
};
use sqlx::{MySql, Pool};
use std::env;

use super::Reminder;
use crate::consts::DISCORD_API;
use crate::routes::dashboard::DeleteReminder;
use chrono_tz::Tz;
use serenity::client::Context;
use serenity::model::id::UserId;

#[derive(Serialize)]
struct UserInfo {
    name: String,
    patreon: bool,
    timezone: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    timezone: String,
}

#[derive(Serialize)]
struct GuildInfo {
    id: String,
    name: String,
}

#[derive(Deserialize)]
pub struct PartialGuild {
    pub id: GuildId,
    pub icon: Option<String>,
    pub name: String,
    #[serde(default)]
    pub owner: bool,
    #[serde(rename = "permissions_new")]
    pub permissions: Option<String>,
}

#[get("/api/user")]
pub async fn get_user_info(
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    if let Some(user_id) =
        cookies.get_private("userid").map(|u| u.value().parse::<u64>().ok()).flatten()
    {
        let member_res = GuildId(env::var("PATREON_GUILD_ID").unwrap().parse().unwrap())
            .member(&ctx.inner(), user_id)
            .await;

        let timezone = sqlx::query!("SELECT timezone FROM users WHERE user = ?", user_id)
            .fetch_one(pool.inner())
            .await
            .map_or(None, |q| Some(q.timezone));

        let user_info = UserInfo {
            name: cookies
                .get_private("username")
                .map_or("DiscordUser#0000".to_string(), |c| c.value().to_string()),
            patreon: member_res.map_or(false, |member| {
                member
                    .roles
                    .contains(&RoleId(env::var("PATREON_ROLE_ID").unwrap().parse().unwrap()))
            }),
            timezone,
        };

        json!(user_info)
    } else {
        json!({"error": "Not authorized"})
    }
}

#[patch("/api/user", data = "<user>")]
pub async fn update_user_info(
    cookies: &CookieJar<'_>,
    user: Json<UpdateUser>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    if let Some(user_id) =
        cookies.get_private("userid").map(|u| u.value().parse::<u64>().ok()).flatten()
    {
        if user.timezone.parse::<Tz>().is_ok() {
            let _ = sqlx::query!(
                "UPDATE users SET timezone = ? WHERE user = ?",
                user.timezone,
                user_id,
            )
            .execute(pool.inner())
            .await;

            json!({})
        } else {
            json!({"error": "Timezone not recognized"})
        }
    } else {
        json!({"error": "Not authorized"})
    }
}

#[get("/api/user/guilds")]
pub async fn get_user_guilds(cookies: &CookieJar<'_>, reqwest_client: &State<Client>) -> JsonValue {
    if let Some(access_token) = cookies.get_private("access_token") {
        let request_res = reqwest_client
            .get(format!("{}/users/@me/guilds", DISCORD_API))
            .bearer_auth(access_token.value())
            .send()
            .await;

        match request_res {
            Ok(response) => {
                let guilds_res = response.json::<Vec<PartialGuild>>().await;

                match guilds_res {
                    Ok(guilds) => {
                        let reduced_guilds = guilds
                            .iter()
                            .filter(|g| {
                                g.owner
                                    || g.permissions.as_ref().map_or(false, |p| {
                                        let permissions =
                                            Permissions::from_bits_truncate(p.parse().unwrap());

                                        permissions.manage_messages()
                                            || permissions.manage_guild()
                                            || permissions.administrator()
                                    })
                            })
                            .map(|g| GuildInfo { id: g.id.to_string(), name: g.name.to_string() })
                            .collect::<Vec<GuildInfo>>();

                        json!(reduced_guilds)
                    }

                    Err(e) => {
                        warn!("Error constructing user from request: {:?}", e);

                        json!({"error": "Could not get user details"})
                    }
                }
            }

            Err(e) => {
                warn!("Error getting user guilds: {:?}", e);

                json!({"error": "Could not reach Discord"})
            }
        }
    } else {
        json!({"error": "Not authorized"})
    }
}

#[post("/api/user/reminders", data = "<reminder>")]
pub async fn create_reminder(
    reminder: Json<Reminder>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    match sqlx::query!(
        "INSERT INTO reminders (
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
         enabled,
         expires,
         interval_seconds,
         interval_months,
         name,
         pin,
         restartable,
         tts,
         username,
         `utc_time`
        ) VALUES (
         avatar = ?,
         content = ?,
         embed_author = ?,
         embed_author_url = ?,
         embed_color = ?,
         embed_description = ?,
         embed_footer = ?,
         embed_footer_url = ?,
         embed_image_url = ?,
         embed_thumbnail_url = ?,
         embed_title = ?,
         enabled = ?,
         expires = ?,
         interval_seconds = ?,
         interval_months = ?,
         name = ?,
         pin = ?,
         restartable = ?,
         tts = ?,
         username = ?,
         `utc_time` = ?
        )",
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
        reminder.enabled,
        reminder.expires,
        reminder.interval_seconds,
        reminder.interval_months,
        reminder.name,
        reminder.pin,
        reminder.restartable,
        reminder.tts,
        reminder.username,
        reminder.utc_time,
    )
    .execute(pool.inner())
    .await
    {
        Ok(_) => {
            json!({})
        }
        Err(e) => {
            warn!("Error in `create_reminder`: {:?}", e);

            json!({"error": "Could not create reminder"})
        }
    }
}

#[get("/api/user/reminders")]
pub async fn get_reminders(
    pool: &State<Pool<MySql>>,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
) -> JsonValue {
    if let Some(user_id) =
        cookies.get_private("userid").map(|c| c.value().parse::<u64>().ok()).flatten()
    {
        let query_res = sqlx::query!(
            "SELECT channel FROM channels INNER JOIN users ON users.dm_channel = channels.id WHERE users.user = ?",
            user_id
        )
        .fetch_one(pool.inner())
        .await;

        let dm_channel = if let Ok(query) = query_res {
            Some(query.channel)
        } else {
            if let Ok(dm_channel) = UserId(user_id).create_dm_channel(&ctx.inner()).await {
                Some(dm_channel.id.as_u64().to_owned())
            } else {
                None
            }
        };

        if let Some(channel_id) = dm_channel {
            let reminders = sqlx::query_as!(
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
                 reminders.enabled,
                 reminders.expires,
                 reminders.interval_seconds,
                 reminders.interval_months,
                 reminders.name,
                 reminders.pin,
                 reminders.restartable,
                 reminders.tts,
                 reminders.uid,
                 reminders.username,
                 reminders.utc_time
                 FROM reminders INNER JOIN channels ON channels.id = reminders.channel_id WHERE channels.channel = ?",
                channel_id
            )
                .fetch_all(pool.inner())
                .await
                .unwrap_or(vec![]);

            json!(reminders)
        } else {
            json!({"error": "User's DM channel could not be determined"})
        }
    } else {
        json!({"error": "Not authorized"})
    }
}

#[put("/api/user/reminders", data = "<reminder>")]
pub async fn overwrite_reminder(reminder: Json<Reminder>, pool: &State<Pool<MySql>>) -> JsonValue {
    match sqlx::query!(
        "UPDATE reminders SET
         avatar = ?,
         content = ?,
         embed_author = ?,
         embed_author_url = ?,
         embed_color = ?,
         embed_description = ?,
         embed_footer = ?,
         embed_footer_url = ?,
         embed_image_url = ?,
         embed_thumbnail_url = ?,
         embed_title = ?,
         enabled = ?,
         expires = ?,
         interval_seconds = ?,
         interval_months = ?,
         name = ?,
         pin = ?,
         restartable = ?,
         tts = ?,
         username = ?,
         `utc_time` = ?
         WHERE uid = ?",
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
        reminder.enabled,
        reminder.expires,
        reminder.interval_seconds,
        reminder.interval_months,
        reminder.name,
        reminder.pin,
        reminder.restartable,
        reminder.tts,
        reminder.username,
        reminder.utc_time,
        reminder.uid
    )
    .execute(pool.inner())
    .await
    {
        Ok(_) => {
            json!({})
        }
        Err(e) => {
            warn!("Error in `overwrite_reminder`: {:?}", e);

            json!({"error": "Could not modify reminder"})
        }
    }
}

#[delete("/api/user/reminders", data = "<reminder>")]
pub async fn delete_reminder(
    reminder: Json<DeleteReminder>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    if sqlx::query!("DELETE FROM reminders WHERE uid = ?", reminder.uid)
        .execute(pool.inner())
        .await
        .is_ok()
    {
        json!({})
    } else {
        json!({"error": "Could not delete reminder"})
    }
}
