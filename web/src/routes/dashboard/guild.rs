use rocket::State;

use crate::consts::DISCORD_CDN;
use serde::Serialize;
use sqlx::{MySql, Pool};

use super::Reminder;
use rocket::serde::json::{json, Json, Value as JsonValue};
use serenity::client::Context;
use serenity::http::CacheHttp;
use serenity::model::id::GuildId;

#[derive(Serialize)]
struct ChannelInfo {
    id: String,
    name: String,
    webhook_avatar: Option<String>,
    webhook_name: Option<String>,
}

// todo check the user can access this guild
#[get("/api/guild/<id>/channels")]
pub async fn get_guild_channels(
    id: u64,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    let channels_res = GuildId(id).channels(ctx.inner()).await;

    match channels_res {
        Ok(channels) => {
            let mut channel_info = vec![];

            for (channel_id, channel) in
                channels.iter().filter(|(_, channel)| channel.is_text_based())
            {
                let mut ch = ChannelInfo {
                    name: channel.name.to_string(),
                    id: channel_id.to_string(),
                    webhook_avatar: None,
                    webhook_name: None,
                };

                if let Ok(webhook_details) = sqlx::query!(
                    "SELECT webhook_id, webhook_token FROM channels WHERE channel = ?",
                    channel.id.as_u64()
                )
                .fetch_one(pool.inner())
                .await
                {
                    if let (Some(webhook_id), Some(webhook_token)) =
                        (webhook_details.webhook_id, webhook_details.webhook_token)
                    {
                        let webhook_res =
                            ctx.http.get_webhook_with_token(webhook_id, &webhook_token).await;

                        if let Ok(webhook) = webhook_res {
                            ch.webhook_avatar = webhook.avatar.map(|a| {
                                format!("{}/{}/{}.webp?size=128", DISCORD_CDN, webhook_id, a)
                            });

                            ch.webhook_name = webhook.name;
                        }
                    }
                }

                channel_info.push(ch);
            }

            json!(channel_info)
        }
        Err(e) => {
            warn!("Could not fetch channels from {}: {:?}", id, e);

            json!({"error": "Could not get channels"})
        }
    }
}

#[derive(Serialize)]
struct RoleInfo {
    id: String,
    name: String,
}

// todo check the user can access this guild
#[get("/api/guild/<id>/roles")]
pub async fn get_guild_roles(id: u64, ctx: &State<Context>) -> JsonValue {
    let roles_res = ctx.cache.guild_roles(id);

    match roles_res {
        Some(roles) => {
            let roles = roles
                .iter()
                .map(|(_, r)| RoleInfo { id: r.id.to_string(), name: r.name.to_string() })
                .collect::<Vec<RoleInfo>>();

            json!(roles)
        }
        None => {
            warn!("Could not fetch roles from {}", id);

            json!({"error": "Could not get roles"})
        }
    }
}

#[post("/api/guild/<id>/reminders", data = "<reminder>")]
pub async fn create_reminder(
    id: u64,
    reminder: Json<Reminder>,
    serenity_context: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    json!({"error": "Not implemented"})
}

#[get("/api/guild/<id>/reminders")]
pub async fn get_reminders(id: u64, ctx: &State<Context>, pool: &State<Pool<MySql>>) -> JsonValue {
    let channels_res = GuildId(id).channels(&ctx.inner()).await;

    match channels_res {
        Ok(channels) => {
            let channels = channels
                .keys()
                .into_iter()
                .map(|k| k.as_u64().to_string())
                .collect::<Vec<String>>()
                .join(",");

            sqlx::query_as_unchecked!(
                Reminder,
                "
SELECT
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
FROM
    reminders
LEFT JOIN
    channels
ON
    channels.id = reminders.channel_id
WHERE
    FIND_IN_SET(channels.channel, ?)
            ",
                channels
            )
            .fetch_all(pool.inner())
            .await
            .map(|r| json!(r))
            .unwrap_or_else(|e| {
                warn!("Failed to complete SQL query: {:?}", e);

                json!({"error": "Could not load reminders"})
            })
        }
        Err(e) => {
            warn!("Could not fetch channels from {}: {:?}", id, e);

            json!([])
        }
    }
}

#[patch("/api/guild/<id>/reminders", data = "<reminder>")]
pub async fn edit_reminder(
    id: u64,
    reminder: Json<Reminder>,
    serenity_context: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    json!({"error": "Not implemented"})
}

#[delete("/api/guild/<id>/reminders", data = "<reminder>")]
pub async fn delete_reminder(
    id: u64,
    reminder: Json<Reminder>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    json!({"error": "Not implemented"})
}
