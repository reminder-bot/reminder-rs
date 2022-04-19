use std::env;

use base64;
use chrono::Utc;
use rocket::{
    http::CookieJar,
    serde::json::{json, Json, Value as JsonValue},
    State,
};
use serde::Serialize;
use serenity::{
    client::Context,
    model::{
        channel::GuildChannel,
        id::{ChannelId, GuildId, RoleId},
    },
};
use sqlx::{MySql, Pool};

use crate::{
    check_guild_subscription, check_subscription,
    consts::{
        DAY, MAX_CONTENT_LENGTH, MAX_EMBED_AUTHOR_LENGTH, MAX_EMBED_DESCRIPTION_LENGTH,
        MAX_EMBED_FIELDS, MAX_EMBED_FIELD_TITLE_LENGTH, MAX_EMBED_FIELD_VALUE_LENGTH,
        MAX_EMBED_FOOTER_LENGTH, MAX_EMBED_TITLE_LENGTH, MAX_URL_LENGTH, MAX_USERNAME_LENGTH,
        MIN_INTERVAL,
    },
    routes::dashboard::{
        create_database_channel, generate_uid, name_default, template_name_default, DeleteReminder,
        DeleteReminderTemplate, PatchReminder, Reminder, ReminderTemplate,
    },
};

#[derive(Serialize)]
struct ChannelInfo {
    id: String,
    name: String,
    webhook_avatar: Option<String>,
    webhook_name: Option<String>,
}

#[get("/api/guild/<id>/patreon")]
pub async fn get_guild_patreon(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    match GuildId(id).to_guild_cached(ctx.inner()) {
        Some(guild) => {
            let member_res = GuildId(env::var("PATREON_GUILD_ID").unwrap().parse().unwrap())
                .member(&ctx.inner(), guild.owner_id)
                .await;

            let patreon = member_res.map_or(false, |member| {
                member
                    .roles
                    .contains(&RoleId(env::var("PATREON_ROLE_ID").unwrap().parse().unwrap()))
            });

            json!({ "patreon": patreon })
        }

        None => {
            json!({"error": "Bot not in guild"})
        }
    }
}

#[get("/api/guild/<id>/channels")]
pub async fn get_guild_channels(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    match GuildId(id).to_guild_cached(ctx.inner()) {
        Some(guild) => {
            let mut channels = guild
                .channels
                .iter()
                .filter_map(|(id, channel)| channel.to_owned().guild().map(|c| (id.to_owned(), c)))
                .filter(|(_, channel)| channel.is_text_based())
                .collect::<Vec<(ChannelId, GuildChannel)>>();

            channels.sort_by(|(_, c1), (_, c2)| c1.position.cmp(&c2.position));

            let channel_info = channels
                .iter()
                .map(|(channel_id, channel)| ChannelInfo {
                    name: channel.name.to_string(),
                    id: channel_id.to_string(),
                    webhook_avatar: None,
                    webhook_name: None,
                })
                .collect::<Vec<ChannelInfo>>();

            json!(channel_info)
        }

        None => {
            json!({"error": "Bot not in guild"})
        }
    }
}

#[derive(Serialize)]
struct RoleInfo {
    id: String,
    name: String,
}

#[get("/api/guild/<id>/roles")]
pub async fn get_guild_roles(id: u64, cookies: &CookieJar<'_>, ctx: &State<Context>) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

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

#[get("/api/guild/<id>/templates")]
pub async fn get_reminder_templates(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    match sqlx::query_as_unchecked!(
        ReminderTemplate,
        "SELECT * FROM reminder_template WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
        id
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(templates) => {
            json!(templates)
        }
        Err(e) => {
            warn!("Could not fetch templates from {}: {:?}", id, e);

            json!({"error": "Could not get templates"})
        }
    }
}

#[post("/api/guild/<id>/templates", data = "<reminder_template>")]
pub async fn create_reminder_template(
    id: u64,
    reminder_template: Json<ReminderTemplate>,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    // validate lengths
    check_length!(MAX_CONTENT_LENGTH, reminder_template.content);
    check_length!(MAX_EMBED_DESCRIPTION_LENGTH, reminder_template.embed_description);
    check_length!(MAX_EMBED_TITLE_LENGTH, reminder_template.embed_title);
    check_length!(MAX_EMBED_AUTHOR_LENGTH, reminder_template.embed_author);
    check_length!(MAX_EMBED_FOOTER_LENGTH, reminder_template.embed_footer);
    check_length_opt!(MAX_EMBED_FIELDS, reminder_template.embed_fields);
    if let Some(fields) = &reminder_template.embed_fields {
        for field in &fields.0 {
            check_length!(MAX_EMBED_FIELD_VALUE_LENGTH, field.value);
            check_length!(MAX_EMBED_FIELD_TITLE_LENGTH, field.title);
        }
    }
    check_length_opt!(MAX_USERNAME_LENGTH, reminder_template.username);
    check_length_opt!(
        MAX_URL_LENGTH,
        reminder_template.embed_footer_url,
        reminder_template.embed_thumbnail_url,
        reminder_template.embed_author_url,
        reminder_template.embed_image_url,
        reminder_template.avatar
    );

    // validate urls
    check_url_opt!(
        reminder_template.embed_footer_url,
        reminder_template.embed_thumbnail_url,
        reminder_template.embed_author_url,
        reminder_template.embed_image_url,
        reminder_template.avatar
    );

    let name = if reminder_template.name.is_empty() {
        template_name_default()
    } else {
        reminder_template.name.clone()
    };

    match sqlx::query!(
        "INSERT INTO reminder_template
        (guild_id,
         name,
         attachment,
         attachment_name,
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
         tts,
         username
        ) VALUES ((SELECT id FROM guilds WHERE guild = ?), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id, name,
        reminder_template.attachment,
        reminder_template.attachment_name,
        reminder_template.avatar,
        reminder_template.content,
        reminder_template.embed_author,
        reminder_template.embed_author_url,
        reminder_template.embed_color,
        reminder_template.embed_description,
        reminder_template.embed_footer,
        reminder_template.embed_footer_url,
        reminder_template.embed_image_url,
        reminder_template.embed_thumbnail_url,
        reminder_template.embed_title,
        reminder_template.embed_fields,
        reminder_template.tts,
        reminder_template.username,
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(_) => {
            json!({})
        }
        Err(e) => {
            warn!("Could not fetch templates from {}: {:?}", id, e);

            json!({"error": "Could not get templates"})
        }
    }
}

#[delete("/api/guild/<id>/templates", data = "<delete_reminder_template>")]
pub async fn delete_reminder_template(
    id: u64,
    delete_reminder_template: Json<DeleteReminderTemplate>,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    match sqlx::query!(
        "DELETE FROM reminder_template WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND id = ?",
        id, delete_reminder_template.id
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(_) => {
            json!({})
        }
        Err(e) => {
            warn!("Could not delete template from {}: {:?}", id, e);

            json!({"error": "Could not delete template"})
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
    check_authorization!(cookies, serenity_context.inner(), id);

    let user_id =
        cookies.get_private("userid").map(|c| c.value().parse::<u64>().ok()).flatten().unwrap();

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
        return json!({"error": "Time must be in the future"});
    }
    if reminder.interval_seconds.is_some() || reminder.interval_months.is_some() {
        if reminder.interval_months.unwrap_or(0) * 30 * DAY as u32
            + reminder.interval_seconds.unwrap_or(0)
            < *MIN_INTERVAL
        {
            return json!({"error": "Interval too short"});
        }
    }

    // check patreon if necessary
    if reminder.interval_seconds.is_some() || reminder.interval_months.is_some() {
        if !check_guild_subscription(serenity_context.inner(), GuildId(id)).await
            && !check_subscription(serenity_context.inner(), user_id).await
        {
            return json!({"error": "Patreon is required to set intervals"});
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
         interval_months,
         name,
         pin,
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
        reminder.interval_months,
        name,
        reminder.pin,
        reminder.restartable,
        reminder.tts,
        reminder.username,
        reminder.utc_time,
    )
    .execute(pool.inner())
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
             reminders.interval_months,
             reminders.name,
             reminders.pin,
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
        .fetch_one(pool.inner())
        .await
        .map(|r| json!(r))
        .unwrap_or_else(|e| {
            warn!("Failed to complete SQL query: {:?}", e);

            json!({"error": "Could not load reminder"})
        }),

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
                 reminders.interval_months,
                 reminders.name,
                 reminders.pin,
                 reminders.restartable,
                 reminders.tts,
                 reminders.uid,
                 reminders.username,
                 reminders.utc_time
                FROM reminders
                LEFT JOIN channels ON channels.id = reminders.channel_id
                WHERE FIND_IN_SET(channels.channel, ?)",
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
    reminder: Json<PatchReminder>,
    serenity_context: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    let mut error = vec![];

    update_field!(pool.inner(), error, reminder.[
        attachment,
        attachment_name,
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
        interval_months,
        name,
        pin,
        restartable,
        tts,
        username,
        utc_time
    ]);

    if reminder.channel > 0 {
        let channel = ChannelId(reminder.channel).to_channel_cached(&serenity_context.inner());
        match channel {
            Some(channel) => {
                let channel_matches_guild = channel.guild().map_or(false, |c| c.guild_id.0 == id);

                if !channel_matches_guild {
                    warn!(
                        "Error in `edit_reminder`: channel {:?} not found for guild {}",
                        reminder.channel, id
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

                match sqlx::query!(
                    "UPDATE reminders SET channel_id = ? WHERE uid = ?",
                    channel,
                    reminder.uid
                )
                .execute(pool.inner())
                .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Error setting channel: {:?}", e);

                        error.push("Couldn't set channel".to_string())
                    }
                }
            }

            None => {
                warn!(
                    "Error in `edit_reminder`: channel {:?} not found for guild {}",
                    reminder.channel, id
                );

                return json!({"error": "Channel not found"});
            }
        }
    }

    match sqlx::query_as_unchecked!(
        Reminder,
        "SELECT reminders.attachment,
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
         reminders.interval_months,
         reminders.name,
         reminders.pin,
         reminders.restartable,
         reminders.tts,
         reminders.uid,
         reminders.username,
         reminders.utc_time
        FROM reminders
        LEFT JOIN channels ON channels.id = reminders.channel_id
        WHERE uid = ?",
        reminder.uid
    )
    .fetch_one(pool.inner())
    .await
    {
        Ok(reminder) => json!({"reminder": reminder, "errors": error}),

        Err(e) => {
            warn!("Error exiting `edit_reminder': {:?}", e);

            json!({"reminder": Option::<Reminder>::None, "errors": vec!["Unknown error"]})
        }
    }
}

#[delete("/api/guild/<_>/reminders", data = "<reminder>")]
pub async fn delete_reminder(
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
