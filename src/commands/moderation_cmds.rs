use std::collections::hash_map::Entry;

use chrono::offset::Utc;
use chrono_tz::{Tz, TZ_VARIANTS};
use levenshtein::levenshtein;
use poise::CreateReply;

use crate::{
    component_models::pager::{MacroPager, Pager},
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    models::{
        command_macro::{guild_command_macro, CommandMacro},
        CtxData,
    },
    Context, Data, Error,
};

async fn timezone_autocomplete(ctx: Context<'_>, partial: String) -> Vec<String> {
    if partial.is_empty() {
        ctx.data().popular_timezones.iter().map(|t| t.to_string()).collect::<Vec<String>>()
    } else {
        TZ_VARIANTS
            .iter()
            .filter(|tz| tz.to_string().contains(&partial))
            .take(25)
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
    }
}

/// Select your timezone
#[poise::command(slash_command, identifying_name = "timezone")]
pub async fn timezone(
    ctx: Context<'_>,
    #[description = "Timezone to use from this list: https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee"]
    #[autocomplete = "timezone_autocomplete"]
    timezone: Option<String>,
) -> Result<(), Error> {
    let mut user_data = ctx.author_data().await.unwrap();

    let footer_text = format!("Current timezone: {}", user_data.timezone);

    if let Some(timezone) = timezone {
        match timezone.parse::<Tz>() {
            Ok(tz) => {
                user_data.timezone = timezone.clone();
                user_data.commit_changes(&ctx.data().database).await;

                let now = Utc::now().with_timezone(&tz);

                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Timezone Set")
                            .description(format!(
                                "Timezone has been set to **{}**. Your current time should be `{}`",
                                timezone,
                                now.format("%H:%M")
                            ))
                            .color(*THEME_COLOR)
                    })
                })
                .await?;
            }

            Err(_) => {
                let filtered_tz = TZ_VARIANTS
                    .iter()
                    .filter(|tz| {
                        timezone.contains(&tz.to_string())
                            || tz.to_string().contains(&timezone)
                            || levenshtein(&tz.to_string(), &timezone) < 4
                    })
                    .take(25)
                    .map(|t| t.to_owned())
                    .collect::<Vec<Tz>>();

                let fields = filtered_tz.iter().map(|tz| {
                    (
                        tz.to_string(),
                        format!("🕗 `{}`", Utc::now().with_timezone(tz).format("%H:%M")),
                        true,
                    )
                });

                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Timezone Not Recognized")
                            .description("Possibly you meant one of the following timezones, otherwise click [here](https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee):")
                            .color(*THEME_COLOR)
                            .fields(fields)
                            .footer(|f| f.text(footer_text))
                            .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
                    })
                })
                .await?;
            }
        }
    } else {
        let popular_timezones_iter = ctx.data().popular_timezones.iter().map(|t| {
            (t.to_string(), format!("🕗 `{}`", Utc::now().with_timezone(t).format("%H:%M")), true)
        });

        ctx.send(|m| {
            m.embed(|e| {
                e.title("Timezone Usage")
                    .description(
                        "**Usage:**
`/timezone Name`

**Example:**
`/timezone Europe/London`

You may want to use one of the popular timezones below, otherwise click [here](https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee):",
                    )
                    .color(*THEME_COLOR)
                    .fields(popular_timezones_iter)
                    .footer(|f| f.text(footer_text))
                    .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
            })
        })
        .await?;
    }

    Ok(())
}

/// View the webhook being used to send reminders to this channel
#[poise::command(
    slash_command,
    identifying_name = "webhook_url",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn webhook(ctx: Context<'_>) -> Result<(), Error> {
    match ctx.channel_data().await {
        Ok(data) => {
            if let (Some(id), Some(token)) = (data.webhook_id, data.webhook_token) {
                let _ = ctx
                    .send(|b| {
                        b.ephemeral(true).content(format!(
                            "**Warning!**
This link can be used by users to anonymously send messages, with or without permissions.
Do not share it!
|| https://discord.com/api/webhooks/{}/{} ||",
                            id, token,
                        ))
                    })
                    .await;
            } else {
                let _ = ctx.say("No webhook configured on this channel.").await;
            }
        }
        Err(_) => {
            let _ = ctx.say("No webhook configured on this channel.").await;
        }
    }

    Ok(())
}

async fn macro_name_autocomplete(ctx: Context<'_>, partial: String) -> Vec<String> {
    sqlx::query!(
        "
SELECT name
FROM macro
WHERE
    guild_id = (SELECT id FROM guilds WHERE guild = ?)
    AND name LIKE CONCAT(?, '%')",
        ctx.guild_id().unwrap().0,
        partial,
    )
    .fetch_all(&ctx.data().database)
    .await
    .unwrap_or_default()
    .iter()
    .map(|s| s.name.clone())
    .collect()
}

/// Record and replay command sequences
#[poise::command(
    slash_command,
    rename = "macro",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "macro_base"
)]
pub async fn macro_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Start recording up to 5 commands to replay
#[poise::command(
    slash_command,
    rename = "record",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "record_macro"
)]
pub async fn record_macro(
    ctx: Context<'_>,
    #[description = "Name for the new macro"] name: String,
    #[description = "Description for the new macro"] description: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let row = sqlx::query!(
        "
SELECT 1 as _e FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?",
        guild_id.0,
        name
    )
    .fetch_one(&ctx.data().database)
    .await;

    if row.is_ok() {
        ctx.send(|m| {
            m.ephemeral(true).embed(|e| {
                e.title("Unique Name Required")
                    .description(
                        "A macro already exists under this name.
Please select a unique name for your macro.",
                    )
                    .color(*THEME_COLOR)
            })
        })
        .await?;
    } else {
        let okay = {
            let mut lock = ctx.data().recording_macros.write().await;

            if let Entry::Vacant(e) = lock.entry((guild_id, ctx.author().id)) {
                e.insert(CommandMacro { guild_id, name, description, commands: vec![] });
                true
            } else {
                false
            }
        };

        if okay {
            ctx.send(|m| {
                m.ephemeral(true).embed(|e| {
                    e.title("Macro Recording Started")
                        .description(
                            "Run up to 5 commands, or type `/macro finish` to stop at any point.
Any commands ran as part of recording will be inconsequential",
                        )
                        .color(*THEME_COLOR)
                })
            })
            .await?;
        } else {
            ctx.send(|m| {
                m.ephemeral(true).embed(|e| {
                    e.title("Macro Already Recording")
                        .description(
                            "You are already recording a macro in this server.
Please use `/macro finish` to end this recording before starting another.",
                        )
                        .color(*THEME_COLOR)
                })
            })
            .await?;
        }
    }

    Ok(())
}

/// Finish current macro recording
#[poise::command(
    slash_command,
    rename = "finish",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "finish_macro"
)]
pub async fn finish_macro(ctx: Context<'_>) -> Result<(), Error> {
    let key = (ctx.guild_id().unwrap(), ctx.author().id);

    {
        let lock = ctx.data().recording_macros.read().await;
        let contained = lock.get(&key);

        if contained.map_or(true, |cmacro| cmacro.commands.is_empty()) {
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("No Macro Recorded")
                        .description("Use `/macro record` to start recording a macro")
                        .color(*THEME_COLOR)
                })
            })
            .await?;
        } else {
            let command_macro = contained.unwrap();
            let json = serde_json::to_string(&command_macro.commands).unwrap();

            sqlx::query!(
                "INSERT INTO macro (guild_id, name, description, commands) VALUES ((SELECT id FROM guilds WHERE guild = ?), ?, ?, ?)",
                command_macro.guild_id.0,
                command_macro.name,
                command_macro.description,
                json
            )
                .execute(&ctx.data().database)
                .await
                .unwrap();

            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Macro Recorded")
                        .description("Use `/macro run` to execute the macro")
                        .color(*THEME_COLOR)
                })
            })
            .await?;
        }
    }

    {
        let mut lock = ctx.data().recording_macros.write().await;
        lock.remove(&key);
    }

    Ok(())
}

/// List recorded macros
#[poise::command(
    slash_command,
    rename = "list",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "list_macro"
)]
pub async fn list_macro(ctx: Context<'_>) -> Result<(), Error> {
    let macros = ctx.command_macros().await?;

    let resp = show_macro_page(&macros, 0);

    ctx.send(|m| {
        *m = resp;
        m
    })
    .await?;

    Ok(())
}

/// Run a recorded macro
#[poise::command(
    slash_command,
    rename = "run",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "run_macro"
)]
pub async fn run_macro(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Name of macro to run"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    match guild_command_macro(&Context::Application(ctx), &name).await {
        Some(command_macro) => {
            ctx.defer_response(false).await?;

            for command in command_macro.commands {
                if let Some(action) = command.action {
                    match (action)(poise::ApplicationContext { args: &command.options, ..ctx })
                        .await
                    {
                        Ok(()) => {}
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                } else {
                    Context::Application(ctx)
                        .say(format!("Command \"{}\" not found", command.command_name))
                        .await?;
                }
            }
        }

        None => {
            Context::Application(ctx).say(format!("Macro \"{}\" not found", name)).await?;
        }
    }

    Ok(())
}

/// Delete a recorded macro
#[poise::command(
    slash_command,
    rename = "delete",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "delete_macro"
)]
pub async fn delete_macro(
    ctx: Context<'_>,
    #[description = "Name of macro to delete"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    match sqlx::query!(
        "
SELECT id FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?",
        ctx.guild_id().unwrap().0,
        name
    )
    .fetch_one(&ctx.data().database)
    .await
    {
        Ok(row) => {
            sqlx::query!("DELETE FROM macro WHERE id = ?", row.id)
                .execute(&ctx.data().database)
                .await
                .unwrap();

            ctx.say(format!("Macro \"{}\" deleted", name)).await?;
        }

        Err(sqlx::Error::RowNotFound) => {
            ctx.say(format!("Macro \"{}\" not found", name)).await?;
        }

        Err(e) => {
            panic!("{}", e);
        }
    }

    Ok(())
}

pub fn max_macro_page<U, E>(macros: &[CommandMacro<U, E>]) -> usize {
    let mut skipped_char_count = 0;

    macros
        .iter()
        .map(|m| {
            if let Some(description) = &m.description {
                format!("**{}**\n- *{}*\n- Has {} commands", m.name, description, m.commands.len())
            } else {
                format!("**{}**\n- Has {} commands", m.name, m.commands.len())
            }
        })
        .fold(1, |mut pages, p| {
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH {
                skipped_char_count = p.len();
                pages += 1;
            }

            pages
        })
}

pub fn show_macro_page<U, E>(macros: &[CommandMacro<U, E>], page: usize) -> CreateReply {
    let pager = MacroPager::new(page);

    if macros.is_empty() {
        let mut reply = CreateReply::default();

        reply.embed(|e| {
            e.title("Macros")
                .description("No Macros Set Up. Use `/macro record` to get started.")
                .color(*THEME_COLOR)
        });

        return reply;
    }

    let pages = max_macro_page(macros);

    let mut page = page;
    if page >= pages {
        page = pages - 1;
    }

    let mut char_count = 0;
    let mut skipped_char_count = 0;

    let mut skipped_pages = 0;

    let display_vec: Vec<String> = macros
        .iter()
        .map(|m| {
            if let Some(description) = &m.description {
                format!("**{}**\n- *{}*\n- Has {} commands", m.name, description, m.commands.len())
            } else {
                format!("**{}**\n- Has {} commands", m.name, m.commands.len())
            }
        })
        .skip_while(|p| {
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH {
                skipped_char_count = p.len();
                skipped_pages += 1;
            }

            skipped_pages < page
        })
        .take_while(|p| {
            char_count += p.len();

            char_count < EMBED_DESCRIPTION_MAX_LENGTH
        })
        .collect::<Vec<String>>();

    let display = display_vec.join("\n");

    let mut reply = CreateReply::default();

    reply
        .embed(|e| {
            e.title("Macros")
                .description(display)
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        })
        .components(|comp| {
            pager.create_button_row(pages, comp);

            comp
        });

    reply
}
