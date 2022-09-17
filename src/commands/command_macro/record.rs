use std::collections::hash_map::Entry;

use crate::{consts::THEME_COLOR, models::command_macro::CommandMacro, Context, Error};

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
    if name.len() > 100 {
        ctx.say("Name must be less than 100 characters").await?;

        return Ok(());
    }

    if description.as_ref().map_or(0, |d| d.len()) > 100 {
        ctx.say("Description must be less than 100 characters").await?;

        return Ok(());
    }

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
