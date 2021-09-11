use std::{collections::HashMap, iter};

use chrono::offset::Utc;
use chrono_tz::{Tz, TZ_VARIANTS};
use levenshtein::levenshtein;
use regex::Regex;
use regex_command_attr::command;
use serenity::{
    client::Context,
    model::{
        channel::Message,
        guild::ActionRole::Create,
        id::{ChannelId, MessageId, RoleId},
        interactions::message_component::ButtonStyle,
        misc::Mentionable,
    },
};

use crate::{
    component_models::{ComponentDataModel, Restrict},
    consts::{REGEX_ALIAS, REGEX_COMMANDS, THEME_COLOR},
    framework::{CommandInvoke, CreateGenericResponse, PermissionLevel},
    models::{channel_data::ChannelData, guild_data::GuildData, user_data::UserData, CtxData},
    PopularTimezones, RegexFramework, SQLPool,
};

#[command("blacklist")]
#[description("Block channels from using bot commands")]
#[arg(
    name = "channel",
    description = "The channel to blacklist",
    kind = "Channel",
    required = false
)]
#[supports_dm(false)]
#[required_permissions(Restricted)]
#[can_blacklist(false)]
async fn blacklist(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let channel = match args.get("channel") {
        Some(channel_id) => ChannelId(channel_id.parse::<u64>().unwrap()),

        None => invoke.channel_id(),
    }
    .to_channel_cached(&ctx)
    .unwrap();

    let mut channel_data = ChannelData::from_channel(&channel, &pool).await.unwrap();

    channel_data.blacklisted = !channel_data.blacklisted;
    channel_data.commit_changes(&pool).await;

    if channel_data.blacklisted {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content(format!("{} has been blacklisted", channel.mention())),
            )
            .await;
    } else {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content(format!("{} has been removed from the blacklist", channel.mention())),
            )
            .await;
    }
}

#[command("timezone")]
#[description("Select your timezone")]
#[arg(
    name = "timezone",
    description = "Timezone to use from this list: https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee",
    kind = "String",
    required = false
)]
async fn timezone(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
    let mut user_data = ctx.user_data(invoke.author_id()).await.unwrap();

    let footer_text = format!("Current timezone: {}", user_data.timezone);

    if let Some(timezone) = args.get("timezone") {
        match timezone.parse::<Tz>() {
            Ok(tz) => {
                user_data.timezone = timezone.clone();
                user_data.commit_changes(&pool).await;

                let now = Utc::now().with_timezone(&tz);

                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().embed(|e| {
                            e.title("Timezone Set")
                                .description(format!(
                                    "Timezone has been set to **{}**. Your current time should be `{}`",
                                    timezone,
                                    now.format("%H:%M").to_string()
                                ))
                                .color(*THEME_COLOR)
                        }),
                    )
                    .await;
            }

            Err(_) => {
                let filtered_tz = TZ_VARIANTS
                    .iter()
                    .filter(|tz| {
                        timezone.contains(&tz.to_string())
                            || tz.to_string().contains(timezone)
                            || levenshtein(&tz.to_string(), timezone) < 4
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

                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().embed(|e| {
                            e.title("Timezone Not Recognized")
                                .description("Possibly you meant one of the following timezones, otherwise click [here](https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee):")
                                .color(*THEME_COLOR)
                                .fields(fields)
                                .footer(|f| f.text(footer_text))
                                .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
                        }),
                    )
                    .await;
            }
        }
    } else {
        let popular_timezones = ctx.data.read().await.get::<PopularTimezones>().cloned().unwrap();

        let popular_timezones_iter = popular_timezones.iter().map(|t| {
            (
                t.to_string(),
                format!("🕗 `{}`", Utc::now().with_timezone(t).format("%H:%M").to_string()),
                true,
            )
        });

        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new().embed(|e| {
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
                }),
            )
            .await;
    }
}

#[command("prefix")]
#[description("Configure a prefix for text-based commands (deprecated)")]
#[supports_dm(false)]
#[required_permissions(Restricted)]
async fn prefix(ctx: &Context, invoke: &(dyn CommandInvoke + Send + Sync), args: String) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let guild_data = ctx.guild_data(invoke.guild_id().unwrap()).await.unwrap();

    if args.len() > 5 {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new().content("Please select a prefix under 5 characters"),
            )
            .await;
    } else if args.is_empty() {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content("Please use this command as `@reminder-bot prefix <prefix>`"),
            )
            .await;
    } else {
        guild_data.write().await.prefix = args;
        guild_data.read().await.commit_changes(&pool).await;

        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content(format!("Prefix changed to {}", guild_data.read().await.prefix)),
            )
            .await;
    }
}

#[command("restrict")]
#[description("Configure which roles can use commands on the bot")]
#[arg(
    name = "role",
    description = "The role to configure command permissions for",
    kind = "Role",
    required = true
)]
#[supports_dm(false)]
#[required_permissions(Restricted)]
async fn restrict(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
    let framework = ctx.data.read().await.get::<RegexFramework>().cloned().unwrap();

    let role = RoleId(args.get("role").unwrap().parse::<u64>().unwrap());

    let restricted_commands =
        sqlx::query!("SELECT command FROM command_restrictions WHERE role_id = ?", role.0)
            .fetch_all(&pool)
            .await
            .unwrap()
            .iter()
            .map(|row| row.command.clone())
            .collect::<Vec<String>>();

    let restrictable_commands = framework
        .commands
        .iter()
        .filter(|c| c.required_permissions == PermissionLevel::Managed)
        .map(|c| c.names[0].to_string())
        .collect::<Vec<String>>();

    let len = restrictable_commands.len();

    let restrict_pl = ComponentDataModel::Restrict(Restrict { role_id: role });

    invoke
        .respond(
            ctx.http.clone(),
            CreateGenericResponse::new()
                .content(format!("Select the commands to allow to {} from below:", role.mention()))
                .components(|c| {
                    c.create_action_row(|row| {
                        row.create_select_menu(|select| {
                            select
                                .custom_id(restrict_pl.to_custom_id())
                                .options(|options| {
                                    for command in restrictable_commands {
                                        options.create_option(|opt| {
                                            opt.label(&command).value(&command).default_selection(
                                                restricted_commands.contains(&command),
                                            )
                                        });
                                    }

                                    options
                                })
                                .min_values(0)
                                .max_values(len as u64)
                        })
                    })
                }),
        )
        .await
        .unwrap();
}

/*
#[command("alias")]
#[supports_dm(false)]
#[permission_level(Managed)]
async fn alias(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let guild_id = msg.guild_id.unwrap().as_u64().to_owned();

    let matches_opt = REGEX_ALIAS.captures(&args);

    if let Some(matches) = matches_opt {
        let name = matches.name("name").unwrap().as_str();
        let command_opt = matches.name("cmd").map(|m| m.as_str());

        match name {
            "list" => {
                let aliases = sqlx::query!(
                    "
SELECT name, command FROM command_aliases WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)
                    ",
                    guild_id
                )
                .fetch_all(&pool)
                .await
                .unwrap();

                let content = iter::once("Aliases:".to_string()).chain(
                    aliases
                        .iter()
                        .map(|row| format!("**{}**: `{}`", row.name, row.command)),
                );

                let _ = msg.channel_id.say_lines(&ctx, content).await;
            }

            "remove" => {
                if let Some(command) = command_opt {
                    let deleted_count = sqlx::query!(
                        "
SELECT COUNT(1) AS count FROM command_aliases WHERE name = ? AND guild_id = (SELECT id FROM guilds WHERE guild = ?)
                        ", command, guild_id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();

                    sqlx::query!(
                        "
DELETE FROM command_aliases WHERE name = ? AND guild_id = (SELECT id FROM guilds WHERE guild = ?)
                        ",
                        command,
                        guild_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();

                    let content = lm
                        .get(&language, "alias/removed")
                        .replace("{count}", &deleted_count.count.to_string());

                    let _ = msg.channel_id.say(&ctx, content).await;
                } else {
                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "alias/help"))
                        .await;
                }
            }

            name => {
                if let Some(command) = command_opt {
                    let res = sqlx::query!(
                        "
INSERT INTO command_aliases (guild_id, name, command) VALUES ((SELECT id FROM guilds WHERE guild = ?), ?, ?)
                        ", guild_id, name, command)
                        .execute(&pool)
                        .await;

                    if res.is_err() {
                        sqlx::query!(
                            "
UPDATE command_aliases SET command = ? WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?
                            ", command, guild_id, name)
                            .execute(&pool)
                            .await
                            .unwrap();
                    }

                    let content = lm.get(&language, "alias/created").replace("{name}", name);

                    let _ = msg.channel_id.say(&ctx, content).await;
                } else {
                    match sqlx::query!(
                        "
SELECT command FROM command_aliases WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?
                        ", guild_id, name)
                        .fetch_one(&pool)
                        .await {

                        Ok(row) => {
                            let framework = ctx.data.read().await
                                .get::<FrameworkCtx>().cloned().expect("Could not get FrameworkCtx from data");

                            let mut new_msg = msg.clone();
                            new_msg.content = format!("<@{}> {}", &ctx.cache.current_user_id(), row.command);
                            new_msg.id = MessageId(0);

                            framework.dispatch(ctx.clone(), new_msg).await;
                        },

                        Err(_) => {
                            let content = lm.get(&language, "alias/not_found").replace("{name}", name);

                            let _ = msg.channel_id.say(&ctx, content).await;
                        },
                    }
                }
            }
        }
    } else {
        let prefix = ctx.prefix(msg.guild_id).await;

        command_help(ctx, msg, lm, &prefix, &language, "alias").await;
    }
}
*/
