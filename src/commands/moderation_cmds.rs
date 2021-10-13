use chrono::offset::Utc;
use chrono_tz::{Tz, TZ_VARIANTS};
use levenshtein::levenshtein;
use regex_command_attr::command;
use serenity::{client::Context, model::misc::Mentionable};

use crate::{
    component_models::{ComponentDataModel, Restrict},
    consts::THEME_COLOR,
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse, OptionValue},
    hooks::{CHECK_GUILD_PERMISSIONS_HOOK, CHECK_MANAGED_PERMISSIONS_HOOK},
    models::{channel_data::ChannelData, command_macro::CommandMacro, CtxData},
    PopularTimezones, RecordingMacros, RegexFramework, SQLPool,
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
#[can_blacklist(false)]
async fn blacklist(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let channel = match args.get("channel") {
        Some(OptionValue::Channel(channel_id)) => *channel_id,

        _ => invoke.channel_id(),
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
async fn timezone(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
    let mut user_data = ctx.user_data(invoke.author_id()).await.unwrap();

    let footer_text = format!("Current timezone: {}", user_data.timezone);

    if let Some(OptionValue::String(timezone)) = args.get("timezone") {
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn prefix(ctx: &Context, invoke: &mut CommandInvoke, args: String) {
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn restrict(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
    let framework = ctx.data.read().await.get::<RegexFramework>().cloned().unwrap();

    if let Some(OptionValue::Role(role)) = args.get("role") {
        let restricted_commands =
            sqlx::query!("SELECT command FROM command_restrictions WHERE role_id = (SELECT id FROM roles WHERE role = ?)", role.0)
                .fetch_all(&pool)
                .await
                .unwrap()
                .iter()
                .map(|row| row.command.clone())
                .collect::<Vec<String>>();

        let restrictable_commands = framework
            .commands
            .iter()
            .filter(|c| c.hooks.contains(&&CHECK_MANAGED_PERMISSIONS_HOOK))
            .map(|c| c.names[0].to_string())
            .collect::<Vec<String>>();

        let len = restrictable_commands.len();

        let restrict_pl = ComponentDataModel::Restrict(Restrict {
            role_id: *role,
            author_id: invoke.author_id(),
            guild_id: invoke.guild_id().unwrap(),
        });

        invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content(format!(
                        "Select the commands to allow to {} from below:",
                        role.mention()
                    ))
                    .components(|c| {
                        c.create_action_row(|row| {
                            row.create_select_menu(|select| {
                                select
                                    .custom_id(restrict_pl.to_custom_id())
                                    .options(|options| {
                                        for command in restrictable_commands {
                                            options.create_option(|opt| {
                                                opt.label(&command)
                                                    .value(&command)
                                                    .default_selection(
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
}

#[command("macro")]
#[description("Record and replay command sequences")]
#[subcommand("record")]
#[description("Start recording up to 5 commands to replay")]
#[arg(name = "name", description = "Name for the new macro", kind = "String", required = true)]
#[arg(
    name = "description",
    description = "Description for the new macro",
    kind = "String",
    required = false
)]
#[subcommand("finish")]
#[description("Finish current recording")]
#[subcommand("list")]
#[description("List recorded macros")]
#[subcommand("run")]
#[description("Run a recorded macro")]
#[arg(name = "name", description = "Name of the macro to run", kind = "String", required = true)]
#[subcommand("delete")]
#[description("Delete a recorded macro")]
#[arg(name = "name", description = "Name of the macro to delete", kind = "String", required = true)]
#[supports_dm(false)]
#[hook(CHECK_MANAGED_PERMISSIONS_HOOK)]
async fn macro_cmd(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    match args.subcommand.clone().unwrap().as_str() {
        "record" => {
            let macro_buffer = ctx.data.read().await.get::<RecordingMacros>().cloned().unwrap();

            {
                let mut lock = macro_buffer.write().await;

                let guild_id = invoke.guild_id().unwrap();

                lock.insert(
                    (guild_id, invoke.author_id()),
                    CommandMacro {
                        guild_id,
                        name: args.get("name").unwrap().to_string(),
                        description: args.get("description").map(|d| d.to_string()),
                        commands: vec![],
                    },
                );
            }

            let _ = invoke
                .respond(
                    &ctx,
                    CreateGenericResponse::new().ephemeral().embed(|e| {
                        e
                                    .title("Macro Recording Started")
                                    .description(
"Run up to 5 commands, or type `/macro finish` to stop at any point.
Any commands ran as part of recording will be inconsequential")
                                    .color(*THEME_COLOR)
                    }),
                )
                .await;
        }
        "finish" => {
            let key = (invoke.guild_id().unwrap(), invoke.author_id());
            let macro_buffer = ctx.data.read().await.get::<RecordingMacros>().cloned().unwrap();

            {
                let lock = macro_buffer.read().await;
                let contained = lock.get(&key);

                if contained.map_or(true, |cmacro| cmacro.commands.is_empty()) {
                    let _ = invoke
                        .respond(
                            &ctx,
                            CreateGenericResponse::new().embed(|e| {
                                e.title("No Macro Recorded")
                                    .description("Use `/macro record` to start recording a macro")
                                    .color(*THEME_COLOR)
                            }),
                        )
                        .await;
                } else {
                    let command_macro = contained.unwrap();
                    let json = serde_json::to_string(&command_macro.commands).unwrap();

                    sqlx::query!(
                        "INSERT INTO macro (guild_id, name, description, commands) VALUES (?, ?, ?, ?)",
                        command_macro.guild_id.0,
                        command_macro.name,
                        command_macro.description,
                        json
                    )
                        .execute(&pool)
                        .await
                        .unwrap();

                    let _ = invoke
                        .respond(
                            &ctx,
                            CreateGenericResponse::new().embed(|e| {
                                e.title("Macro Recorded")
                                    .description("Use `/macro run` to execute the macro")
                                    .color(*THEME_COLOR)
                            }),
                        )
                        .await;
                }
            }

            {
                let mut lock = macro_buffer.write().await;
                lock.remove(&key);
            }
        }
        "list" => {}
        "run" => {
            let macro_name = args.get("name").unwrap().to_string();

            match sqlx::query!(
                "SELECT commands FROM macro WHERE guild_id = ? AND name = ?",
                invoke.guild_id().unwrap().0,
                macro_name
            )
            .fetch_one(&pool)
            .await
            {
                Ok(row) => {
                    invoke.defer(&ctx).await;

                    let commands: Vec<CommandOptions> =
                        serde_json::from_str(&row.commands).unwrap();
                    let framework = ctx.data.read().await.get::<RegexFramework>().cloned().unwrap();

                    for command in commands {
                        framework.run_command_from_options(ctx, invoke, command).await;
                    }
                }

                Err(sqlx::Error::RowNotFound) => {
                    let _ = invoke
                        .respond(
                            &ctx,
                            CreateGenericResponse::new()
                                .content(format!("Macro \"{}\" not found", macro_name)),
                        )
                        .await;
                }

                Err(e) => {
                    panic!("{}", e);
                }
            }
        }
        "delete" => {}
        _ => {}
    }
}
