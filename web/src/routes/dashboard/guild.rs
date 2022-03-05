use chrono::Utc;
use rocket::{
    http::CookieJar,
    serde::json::{json, Json, Value as JsonValue},
    State,
};
use serde::Serialize;
use serenity::{
    client::Context,
    model::id::{ChannelId, GuildId},
};
use sqlx::{MySql, Pool};

use crate::{
    check_guild_subscription, check_subscription,
    consts::{
        DAY, DISCORD_CDN, MAX_CONTENT_LENGTH, MAX_EMBED_AUTHOR_LENGTH,
        MAX_EMBED_DESCRIPTION_LENGTH, MAX_EMBED_FOOTER_LENGTH, MAX_EMBED_TITLE_LENGTH,
        MAX_URL_LENGTH, MAX_USERNAME_LENGTH, MIN_INTERVAL,
    },
    routes::dashboard::{create_database_channel, DeleteReminder, Reminder},
};

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
    cookies: &CookieJar<'_>,
    serenity_context: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    // get userid from cookies
    let user_id = cookies.get_private("userid").map(|c| c.value().parse::<u64>().ok()).flatten();

    if user_id.is_none() {
        return json!({"error": "User not authorized"});
    }

    let user_id = user_id.unwrap();

    // validate channel
    let channel = ChannelId(reminder.channel).to_channel_cached(&serenity_context.inner());
    let channel_exists = channel.is_some();

    let channel_matches_guild =
        channel.map_or(false, |c| c.guild().map_or(false, |c| c.guild_id.0 == id));

    if !channel_matches_guild || !channel_exists {
        warn!(
            "Error in `create_reminder`: channel {} not found for guild {} (channel exists: {})",
            reminder.channel, id, channel_exists
        );

        return json!({"error": "Channel not found"});
    }

    let channel = create_database_channel(
        serenity_context.inner(),
        ChannelId(reminder.channel),
        pool.inner(),
    )
    .await;

    if let Err(e) = channel {
        warn!("`create_database_channel` returned an error code: {:?}", e);

        return json!({"error": "Failed to configure channel for reminders. Please check the bot permissions"});
    }

    let channel = channel.unwrap();

    // validate lengths
    check_length!(MAX_CONTENT_LENGTH, reminder.content);
    check_length!(MAX_EMBED_DESCRIPTION_LENGTH, reminder.embed_description);
    check_length!(MAX_EMBED_TITLE_LENGTH, reminder.embed_title);
    check_length!(MAX_EMBED_AUTHOR_LENGTH, reminder.embed_author);
    check_length!(MAX_EMBED_FOOTER_LENGTH, reminder.embed_footer);
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
        return json!({"error": "Time must be in the future"});
    }
    if reminder.interval_months.unwrap_or(0) * 30 * DAY as u32
        + reminder.interval_seconds.unwrap_or(0)
        < *MIN_INTERVAL
    {
        return json!({"error": "Interval too short"});
    }

    // check patreon if necessary
    if reminder.interval_seconds.is_some() || reminder.interval_months.is_some() {
        if !check_guild_subscription(serenity_context.inner(), GuildId(id)).await
            && !check_subscription(serenity_context.inner(), user_id).await
        {
            return json!({"error": "Patreon is required to set intervals"});
        }
    }

    // write to db
    match sqlx::query!(
        "INSERT INTO reminders (
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
         channel_id = ?,
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
        Ok(_) => json!({}),

        Err(e) => {
            warn!("Error in `create_reminder`: Could not execute query: {:?}", e);

            json!({"error": "Unknown error"})
        }
    }
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
    reminder: Json<DeleteReminder>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    match sqlx::query!("DELETE FROM reminders WHERE uid = ?", reminder.uid)
        .execute(pool.inner())
        .await
    {
        Ok(_) => {
            json!({})
        }

        Err(e) => {
            warn!("Error in `delete_reminder`: {:?}", e);

            json!({"error": "Could not delete reminder"})
        }
    }
}
