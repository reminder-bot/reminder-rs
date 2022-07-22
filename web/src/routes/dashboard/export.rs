use csv::{QuoteStyle, WriterBuilder};
use rocket::{
    http::CookieJar,
    serde::json::{json, Json, Value as JsonValue},
    State,
};
use serenity::{
    client::Context,
    model::id::{ChannelId, GuildId},
};
use sqlx::{MySql, Pool};

use crate::routes::dashboard::{ImportBody, ReminderCsv, ReminderTemplateCsv, TodoCsv};

#[get("/api/guild/<id>/export/reminders")]
pub async fn export_reminders(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    let mut csv_writer = WriterBuilder::new().quote_style(QuoteStyle::Always).from_writer(vec![]);

    let channels_res = GuildId(id).channels(&ctx.inner()).await;

    match channels_res {
        Ok(channels) => {
            let channels = channels
                .keys()
                .into_iter()
                .map(|k| k.as_u64().to_string())
                .collect::<Vec<String>>()
                .join(",");

            let result = sqlx::query_as_unchecked!(
                ReminderCsv,
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
                 reminders.restartable,
                 reminders.tts,
                 reminders.username,
                 reminders.utc_time
                FROM reminders
                LEFT JOIN channels ON channels.id = reminders.channel_id
                WHERE FIND_IN_SET(channels.channel, ?)",
                channels
            )
            .fetch_all(pool.inner())
            .await;

            match result {
                Ok(reminders) => {
                    reminders.iter().for_each(|reminder| {
                        csv_writer.serialize(reminder).unwrap();
                    });

                    match csv_writer.into_inner() {
                        Ok(inner) => match String::from_utf8(inner) {
                            Ok(encoded) => {
                                json!({ "body": encoded })
                            }

                            Err(e) => {
                                warn!("Failed to write UTF-8: {:?}", e);

                                json!({"error": "Failed to write UTF-8"})
                            }
                        },

                        Err(e) => {
                            warn!("Failed to extract CSV: {:?}", e);

                            json!({"error": "Failed to extract CSV"})
                        }
                    }
                }

                Err(e) => {
                    warn!("Failed to complete SQL query: {:?}", e);

                    json!({"error": "Failed to query reminders"})
                }
            }
        }

        Err(e) => {
            warn!("Could not fetch channels from {}: {:?}", id, e);

            json!({"error": "Failed to get guild channels"})
        }
    }
}

#[put("/api/guild/<id>/export/reminders", data = "<body>")]
pub async fn import_reminders(
    id: u64,
    cookies: &CookieJar<'_>,
    body: Json<ImportBody>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    match base64::decode(&body.body) {
        Ok(body) => {
            let mut reader = csv::Reader::from_reader(body.as_slice());

            for result in reader.deserialize::<ReminderCsv>() {
                match result {
                    Ok(record) => {}

                    Err(e) => {
                        warn!("Couldn't deserialize CSV row: {:?}", e);
                    }
                }
            }

            json!({"error": "Not implemented"})
        }

        Err(_) => {
            json!({"error": "Malformed base64"})
        }
    }
}

#[get("/api/guild/<id>/export/todos")]
pub async fn export_todos(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    let mut csv_writer = WriterBuilder::new().quote_style(QuoteStyle::Always).from_writer(vec![]);

    match sqlx::query_as_unchecked!(
        TodoCsv,
        "SELECT value, CONCAT('#', channels.channel) AS channel_id FROM todos
        LEFT JOIN channels ON todos.channel_id = channels.id
        INNER JOIN guilds ON todos.guild_id = guilds.id
        WHERE guilds.guild = ?",
        id
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(todos) => {
            todos.iter().for_each(|todo| {
                csv_writer.serialize(todo).unwrap();
            });

            match csv_writer.into_inner() {
                Ok(inner) => match String::from_utf8(inner) {
                    Ok(encoded) => {
                        json!({ "body": encoded })
                    }

                    Err(e) => {
                        warn!("Failed to write UTF-8: {:?}", e);

                        json!({"error": "Failed to write UTF-8"})
                    }
                },

                Err(e) => {
                    warn!("Failed to extract CSV: {:?}", e);

                    json!({"error": "Failed to extract CSV"})
                }
            }
        }
        Err(e) => {
            warn!("Could not fetch templates from {}: {:?}", id, e);

            json!({"error": "Failed to query templates"})
        }
    }
}

#[put("/api/guild/<id>/export/todos", data = "<body>")]
pub async fn import_todos(
    id: u64,
    cookies: &CookieJar<'_>,
    body: Json<ImportBody>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    let channels_res = GuildId(id).channels(&ctx.inner()).await;

    match channels_res {
        Ok(channels) => match base64::decode(&body.body) {
            Ok(body) => {
                let mut reader = csv::Reader::from_reader(body.as_slice());

                let query_placeholder = "(?, (SELECT id FROM channels WHERE channel = ?), (SELECT id FROM guilds WHERE guild = ?))";
                let mut query_params = vec![];

                for result in reader.deserialize::<TodoCsv>() {
                    match result {
                        Ok(record) => match record.channel_id {
                            Some(channel_id) => {
                                let channel_id = channel_id.split_at(1).1;

                                match channel_id.parse::<u64>() {
                                    Ok(channel_id) => {
                                        if channels.contains_key(&ChannelId(channel_id)) {
                                            query_params.push((record.value, Some(channel_id), id));
                                        } else {
                                            return json!({
                                                "error":
                                                    format!("Invalid channel ID {}", channel_id)
                                            });
                                        }
                                    }

                                    Err(_) => {
                                        return json!({
                                            "error": format!("Invalid channel ID {}", channel_id)
                                        });
                                    }
                                }
                            }

                            None => {
                                query_params.push((record.value, None, id));
                            }
                        },

                        Err(e) => {
                            warn!("Couldn't deserialize CSV row: {:?}", e);

                            return json!({"error": "Deserialize error. Aborted"});
                        }
                    }
                }

                let _ = sqlx::query!(
                    "DELETE FROM todos WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
                    id
                )
                .execute(pool.inner())
                .await;

                let query_str = format!(
                    "INSERT INTO todos (value, channel_id, guild_id) VALUES {}",
                    vec![query_placeholder].repeat(query_params.len()).join(",")
                );
                let mut query = sqlx::query(&query_str);

                for param in query_params {
                    query = query.bind(param.0).bind(param.1).bind(param.2);
                }

                let res = query.execute(pool.inner()).await;

                match res {
                    Ok(_) => {
                        json!({})
                    }

                    Err(e) => {
                        warn!("Couldn't execute todo query: {:?}", e);

                        json!({"error": "An unexpected error occured."})
                    }
                }
            }

            Err(_) => {
                json!({"error": "Malformed base64"})
            }
        },

        Err(e) => {
            warn!("Couldn't fetch channels for guild {}: {:?}", id, e);

            json!({"error": "Couldn't fetch channels."})
        }
    }
}

#[get("/api/guild/<id>/export/reminder_templates")]
pub async fn export_reminder_templates(
    id: u64,
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    check_authorization!(cookies, ctx.inner(), id);

    let mut csv_writer = WriterBuilder::new().quote_style(QuoteStyle::Always).from_writer(vec![]);

    match sqlx::query_as_unchecked!(
        ReminderTemplateCsv,
        "SELECT
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
        FROM reminder_template WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
        id
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(templates) => {
            templates.iter().for_each(|template| {
                csv_writer.serialize(template).unwrap();
            });

            match csv_writer.into_inner() {
                Ok(inner) => match String::from_utf8(inner) {
                    Ok(encoded) => {
                        json!({ "body": encoded })
                    }

                    Err(e) => {
                        warn!("Failed to write UTF-8: {:?}", e);

                        json!({"error": "Failed to write UTF-8"})
                    }
                },

                Err(e) => {
                    warn!("Failed to extract CSV: {:?}", e);

                    json!({"error": "Failed to extract CSV"})
                }
            }
        }
        Err(e) => {
            warn!("Could not fetch templates from {}: {:?}", id, e);

            json!({"error": "Failed to query templates"})
        }
    }
}
