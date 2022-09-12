use lazy_regex::regex;
use poise::serenity_prelude::command::CommandOptionType;
use regex::Captures;
use serde_json::{json, Value};

use crate::{models::command_macro::RawCommandMacro, Context, Error, GuildId};

struct Alias {
    name: String,
    command: String,
}

/// Migrate old $alias reminder commands to macros. Only macro names that are not taken will be used.
#[poise::command(
    slash_command,
    rename = "migrate",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "migrate_macro"
)]
pub async fn migrate_macro(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let mut transaction = ctx.data().database.begin().await?;

    let aliases = sqlx::query_as!(
        Alias,
        "SELECT name, command FROM command_aliases WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
        guild_id.0
    )
    .fetch_all(&mut transaction)
    .await?;

    let mut added_aliases = 0;

    for alias in aliases {
        match parse_text_command(guild_id, alias.name, &alias.command) {
            Some(cmd_macro) => {
                sqlx::query!(
                    "INSERT INTO macro (guild_id, name, description, commands) VALUES ((SELECT id FROM guilds WHERE guild = ?), ?, ?, ?)",
                    cmd_macro.guild_id.0,
                    cmd_macro.name,
                    cmd_macro.description,
                    cmd_macro.commands
                )
                .execute(&mut transaction)
                .await?;

                added_aliases += 1;
            }

            None => {}
        }
    }

    transaction.commit().await?;

    ctx.send(|b| b.content(format!("Added {} macros.", added_aliases))).await?;

    Ok(())
}

fn parse_text_command(
    guild_id: GuildId,
    alias_name: String,
    command: &str,
) -> Option<RawCommandMacro> {
    match command.split_once(" ") {
        Some((command_word, args)) => {
            let command_word = command_word.to_lowercase();

            if command_word == "r"
                || command_word == "i"
                || command_word == "remind"
                || command_word == "interval"
            {
                let matcher = regex!(
                    r#"(?P<mentions>(?:<@\d+>\s+|<@!\d+>\s+|<#\d+>\s+)*)(?P<time>(?:(?:\d+)(?:s|m|h|d|:|/|-|))+)(?:\s+(?P<interval>(?:(?:\d+)(?:s|m|h|d|))+))?(?:\s+(?P<expires>(?:(?:\d+)(?:s|m|h|d|:|/|-|))+))?\s+(?P<content>.*)"#s
                );

                match matcher.captures(&args) {
                    Some(captures) => {
                        let mut args: Vec<Value> = vec![];

                        if let Some(group) = captures.name("time") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "time",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("content") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "content",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("interval") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "interval",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("expires") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "expires",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("mentions") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "channels",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        Some(RawCommandMacro {
                            guild_id,
                            name: alias_name,
                            description: None,
                            commands: json!([
                                {
                                    "command_name": "remind",
                                    "options": args,
                                }
                            ]),
                        })
                    }

                    None => None,
                }
            } else if command_word == "n" || command_word == "natural" {
                let matcher_primary = regex!(
                    r#"(?P<time>.*?)(?:\s+)(?:send|say)(?:\s+)(?P<content>.*?)(?:(?:\s+)to(?:\s+)(?P<mentions>((?:<@\d+>)|(?:<@!\d+>)|(?:<#\d+>)|(?:\s+))+))?$"#s
                );
                let matcher_secondary = regex!(
                    r#"(?P<msg>.*)(?:\s+)every(?:\s+)(?P<interval>.*?)(?:(?:\s+)(?:until|for)(?:\s+)(?P<expires>.*?))?$"#s
                );

                match matcher_primary.captures(&args) {
                    Some(captures) => {
                        let captures_secondary = matcher_secondary.captures(&args);

                        let mut args: Vec<Value> = vec![];

                        if let Some(group) = captures.name("time") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "time",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("content") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "content",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) =
                            captures_secondary.as_ref().and_then(|c: &Captures| c.name("interval"))
                        {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "interval",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) =
                            captures_secondary.and_then(|c: Captures| c.name("expires"))
                        {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "expires",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        if let Some(group) = captures.name("mentions") {
                            let content = group.as_str();
                            args.push(json!({
                                "name": "channels",
                                "value": content,
                                "type": CommandOptionType::String,
                            }));
                        }

                        Some(RawCommandMacro {
                            guild_id,
                            name: alias_name,
                            description: None,
                            commands: json!([
                                {
                                    "command_name": "remind",
                                    "options": args,
                                }
                            ]),
                        })
                    }

                    None => None,
                }
            } else {
                None
            }
        }

        None => None,
    }
}
