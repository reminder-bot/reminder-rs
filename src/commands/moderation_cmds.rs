use chrono::offset::Utc;
use chrono_tz::{Tz, TZ_VARIANTS};
use levenshtein::levenshtein;
use poise::CreateReply;

use crate::{
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    hooks::guild_only,
    models::{command_macro::CommandMacro, CtxData},
    Context, Error,
};

async fn timezone_autocomplete(ctx: Context<'_>, partial: String) -> Vec<String> {
    if partial.is_empty() {
        ctx.data().popular_timezones.iter().map(|t| t.to_string()).collect::<Vec<String>>()
    } else {
        TZ_VARIANTS
            .iter()
            .filter(|tz| {
                partial.contains(&tz.to_string())
                    || tz.to_string().contains(&partial)
                    || levenshtein(&tz.to_string(), &partial) < 4
            })
            .take(25)
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
    }
}

/// Select your timezone
#[poise::command(slash_command)]
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
                                now.format("%H:%M").to_string()
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
                        format!(
                            "🕗 `{}`",
                            Utc::now().with_timezone(tz).format("%H:%M").to_string()
                        ),
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
            (
                t.to_string(),
                format!("🕗 `{}`", Utc::now().with_timezone(t).format("%H:%M").to_string()),
                true,
            )
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
    .unwrap_or(vec![])
    .iter()
    .map(|s| s.name.clone())
    .collect()
}

/// Record and replay command sequences
#[poise::command(slash_command, rename = "macro", check = "guild_only")]
pub async fn macro_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Start recording up to 5 commands to replay
#[poise::command(slash_command, rename = "record", check = "guild_only")]
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

            if lock.contains_key(&(guild_id, ctx.author().id)) {
                false
            } else {
                lock.insert(
                    (guild_id, ctx.author().id),
                    CommandMacro { guild_id, name, description, commands: vec![] },
                );
                true
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
    check = "guild_only",
    identifying_name = "macro_finish"
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
#[poise::command(slash_command, rename = "list", check = "guild_only")]
pub async fn list_macro(ctx: Context<'_>) -> Result<(), Error> {
    let macros = CommandMacro::from_guild(&ctx.data().database, ctx.guild_id().unwrap()).await;

    let resp = show_macro_page(&macros, 0);

    ctx.send(|m| {
        *m = resp;
        m
    })
    .await?;

    Ok(())
}

/// Run a recorded macro
#[poise::command(slash_command, rename = "run", check = "guild_only")]
pub async fn run_macro(
    ctx: Context<'_>,
    #[description = "Name of macro to run"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    match sqlx::query!(
        "
SELECT commands FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?",
        ctx.guild_id().unwrap().0,
        name
    )
    .fetch_one(&ctx.data().database)
    .await
    {
        Ok(row) => {
            ctx.defer().await?;

            // TODO TODO TODO!!!!!!!! RUN COMMAND FROM MACRO
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

/// Delete a recorded macro
#[poise::command(slash_command, rename = "delete", check = "guild_only")]
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

pub fn max_macro_page(macros: &[CommandMacro]) -> usize {
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

pub fn show_macro_page(macros: &[CommandMacro], page: usize) -> CreateReply {
    let mut reply = CreateReply::default();

    reply.embed(|e| {
        e.title("Macros")
            .description("No Macros Set Up. Use `/macro record` to get started.")
            .color(*THEME_COLOR)
    });

    reply

    /*
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
     */
}
