use regex_command_attr::command;

use serenity::{
    builder::CreateActionRow,
    client::Context,
    framework::Framework,
    model::{
        channel::Message,
        id::{ChannelId, MessageId, RoleId},
        interactions::message_component::ButtonStyle,
    },
};

use chrono_tz::{Tz, TZ_VARIANTS};

use chrono::offset::Utc;

use inflector::Inflector;

use levenshtein::levenshtein;

use crate::{
    command_help,
    consts::{REGEX_ALIAS, REGEX_CHANNEL, REGEX_COMMANDS, REGEX_ROLE, THEME_COLOR},
    framework::SendIterator,
    get_ctx_data,
    models::{channel_data::ChannelData, guild_data::GuildData, user_data::UserData, CtxData},
    FrameworkCtx, PopularTimezones,
};

use std::{collections::HashMap, iter};

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
#[can_blacklist(false)]
async fn blacklist(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let capture_opt = REGEX_CHANNEL
        .captures(&args)
        .map(|cap| cap.get(1))
        .flatten();

    let (channel, local) = match capture_opt {
        Some(capture) => (
            ChannelId(capture.as_str().parse::<u64>().unwrap()).to_channel_cached(&ctx),
            false,
        ),

        None => (msg.channel(&ctx).await.ok(), true),
    };

    let mut channel_data = ChannelData::from_channel(channel.unwrap(), &pool)
        .await
        .unwrap();

    channel_data.blacklisted = !channel_data.blacklisted;
    channel_data.commit_changes(&pool).await;

    if channel_data.blacklisted {
        if local {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "blacklist/added"))
                .await;
        } else {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "blacklist/added_from"))
                .await;
        }
    } else if local {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "blacklist/removed"))
            .await;
    } else {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "blacklist/removed_from"))
            .await;
    }
}

#[command]
async fn timezone(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let mut user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    let footer_text = lm.get(&user_data.language, "timezone/footer").replacen(
        "{timezone}",
        &user_data.timezone,
        1,
    );

    if !args.is_empty() {
        match args.parse::<Tz>() {
            Ok(_) => {
                user_data.timezone = args;
                user_data.commit_changes(&pool).await;

                let now = Utc::now().with_timezone(&user_data.timezone());

                let content = lm
                    .get(&user_data.language, "timezone/set_p")
                    .replacen("{timezone}", &user_data.timezone, 1)
                    .replacen("{time}", &now.format("%H:%M").to_string(), 1);

                let _ =
                    msg.channel_id
                        .send_message(&ctx, |m| {
                            m.embed(|e| {
                                e.title(lm.get(&user_data.language, "timezone/set_p_title"))
                                    .description(content)
                                    .color(*THEME_COLOR)
                                    .footer(|f| {
                                        f.text(
                                            lm.get(&user_data.language, "timezone/footer")
                                                .replacen("{timezone}", &user_data.timezone, 1),
                                        )
                                    })
                            })
                        })
                        .await;
            }

            Err(_) => {
                let filtered_tz = TZ_VARIANTS
                    .iter()
                    .filter(|tz| {
                        args.contains(&tz.to_string())
                            || tz.to_string().contains(&args)
                            || levenshtein(&tz.to_string(), &args) < 4
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

                let _ = msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.embed(|e| {
                            e.title(lm.get(&user_data.language, "timezone/no_timezone_title"))
                                .description(lm.get(&user_data.language, "timezone/no_timezone"))
                                .color(*THEME_COLOR)
                                .fields(fields)
                                .footer(|f| f.text(footer_text))
                                .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
                        }).components(|c| {
                            for row in filtered_tz.as_slice().chunks(5) {
                                let mut action_row = CreateActionRow::default();
                                for timezone in row {
                                    action_row.create_button(|b| {
                                        b.style(ButtonStyle::Secondary)
                                            .label(timezone.to_string())
                                            .custom_id(format!("timezone:{}", timezone.to_string()))
                                    });
                                }

                                c.add_action_row(action_row);
                            }

                            c
                        })
                    })
                    .await;
            }
        }
    } else {
        let content = lm
            .get(&user_data.language, "timezone/no_argument")
            .replace("{prefix}", &ctx.prefix(msg.guild_id).await);

        let popular_timezones = ctx
            .data
            .read()
            .await
            .get::<PopularTimezones>()
            .cloned()
            .unwrap();

        let popular_timezones_iter = popular_timezones.iter().map(|t| {
            (
                t.to_string(),
                format!(
                    "🕗 `{}`",
                    Utc::now().with_timezone(t).format("%H:%M").to_string()
                ),
                true,
            )
        });

        let _ = msg
            .channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    e.title(lm.get(&user_data.language, "timezone/no_argument_title"))
                        .description(content)
                        .color(*THEME_COLOR)
                        .fields(popular_timezones_iter)
                        .footer(|f| f.text(footer_text))
                        .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
                })
                .components(|c| {
                    for row in popular_timezones.as_slice().chunks(5) {
                        let mut action_row = CreateActionRow::default();
                        for timezone in row {
                            action_row.create_button(|b| {
                                b.style(ButtonStyle::Secondary)
                                    .label(timezone.to_string())
                                    .custom_id(format!("timezone:{}", timezone.to_string()))
                            });
                        }

                        c.add_action_row(action_row);
                    }

                    c
                })
            })
            .await;
    }
}

#[command("lang")]
async fn language(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let mut user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    if !args.is_empty() {
        match lm.get_language(&args) {
            Some(lang) => {
                user_data.language = lang.to_string();

                user_data.commit_changes(&pool).await;

                let _ = msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.embed(|e| {
                            e.title(lm.get(&user_data.language, "lang/set_p_title"))
                                .color(*THEME_COLOR)
                                .description(lm.get(&user_data.language, "lang/set_p"))
                        })
                    })
                    .await;
            }

            None => {
                let language_codes = lm.all_languages().map(|(k, v)| {
                    (
                        format!("{} {}", lm.get(k, "flag"), v.to_title_case()),
                        format!("`$lang {}`", k.to_uppercase()),
                        true,
                    )
                });

                let _ = msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.embed(|e| {
                            e.title(lm.get(&user_data.language, "lang/invalid_title"))
                                .color(*THEME_COLOR)
                                .description(lm.get(&user_data.language, "lang/invalid"))
                                .fields(language_codes)
                        })
                        .components(|c| {
                            for row in lm
                                .all_languages()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect::<Vec<(String, String)>>()
                                .as_slice()
                                .chunks(5)
                            {
                                let mut action_row = CreateActionRow::default();
                                for (code, name) in row {
                                    action_row.create_button(|b| {
                                        b.style(ButtonStyle::Primary)
                                            .label(name.to_title_case())
                                            .custom_id(format!("lang:{}", code.to_uppercase()))
                                    });
                                }

                                c.add_action_row(action_row);
                            }

                            c
                        })
                    })
                    .await;
            }
        }
    } else {
        let language_codes = lm.all_languages().map(|(k, v)| {
            (
                format!("{} {}", lm.get(k, "flag"), v.to_title_case()),
                format!("`$lang {}`", k.to_uppercase()),
                true,
            )
        });

        let _ = msg
            .channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    e.title(lm.get(&user_data.language, "lang/select_title"))
                        .color(*THEME_COLOR)
                        .description(lm.get(&user_data.language, "lang/select"))
                        .fields(language_codes)
                })
                .components(|c| {
                    for row in lm
                        .all_languages()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        .as_slice()
                        .chunks(5)
                    {
                        let mut action_row = CreateActionRow::default();
                        for (code, name) in row {
                            action_row.create_button(|b| {
                                b.style(ButtonStyle::Primary)
                                    .label(name.to_title_case())
                                    .custom_id(format!("lang:{}", code.to_uppercase()))
                            });
                        }

                        c.add_action_row(action_row);
                    }

                    c
                })
            })
            .await;
    }
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn prefix(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let guild_data = ctx.guild_data(msg.guild_id.unwrap()).await.unwrap();
    let language = UserData::language_of(&msg.author, &pool).await;

    if args.len() > 5 {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "prefix/too_long"))
            .await;
    } else if args.is_empty() {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "prefix/no_argument"))
            .await;
    } else {
        guild_data.write().await.prefix = args;

        guild_data.read().await.commit_changes(&pool).await;

        let content = lm.get(&language, "prefix/success").replacen(
            "{prefix}",
            &guild_data.read().await.prefix,
            1,
        );

        let _ = msg.channel_id.say(&ctx, content).await;
    }
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn restrict(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let guild_data = GuildData::from_guild(msg.guild(&ctx).unwrap(), &pool)
        .await
        .unwrap();

    let role_tag_match = REGEX_ROLE.find(&args);

    if let Some(role_tag) = role_tag_match {
        let commands = REGEX_COMMANDS
            .find_iter(&args.to_lowercase())
            .map(|c| c.as_str().to_string())
            .collect::<Vec<String>>();
        let role_id = RoleId(
            role_tag.as_str()[3..role_tag.as_str().len() - 1]
                .parse::<u64>()
                .unwrap(),
        );

        let role_opt = role_id.to_role_cached(&ctx);

        if let Some(role) = role_opt {
            let _ = sqlx::query!(
                "
DELETE FROM command_restrictions WHERE role_id = (SELECT id FROM roles WHERE role = ?)
                ",
                role.id.as_u64()
            )
            .execute(&pool)
            .await;

            if commands.is_empty() {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "restrict/disabled"))
                    .await;
            } else {
                let _ = sqlx::query!(
                    "
INSERT IGNORE INTO roles (role, name, guild_id) VALUES (?, ?, ?)
                    ",
                    role.id.as_u64(),
                    role.name,
                    guild_data.id
                )
                .execute(&pool)
                .await;

                for command in commands {
                    let res = sqlx::query!(
                        "
INSERT INTO command_restrictions (role_id, command) VALUES ((SELECT id FROM roles WHERE role = ?), ?)
                        ", role.id.as_u64(), command)
                        .execute(&pool)
                        .await;

                    if res.is_err() {
                        println!("{:?}", res);

                        let content = lm.get(&language, "restrict/failure").replacen(
                            "{command}",
                            &command,
                            1,
                        );

                        let _ = msg.channel_id.say(&ctx, content).await;
                    }
                }

                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "restrict/enabled"))
                    .await;
            }
        }
    } else if args.is_empty() {
        let guild_id = msg.guild_id.unwrap().as_u64().to_owned();

        let rows = sqlx::query!(
            "
SELECT
    roles.role, command_restrictions.command
FROM
    command_restrictions
INNER JOIN
    roles
ON
    roles.id = command_restrictions.role_id
WHERE
    roles.guild_id = (SELECT id FROM guilds WHERE guild = ?)
            ",
            guild_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let mut commands_roles: HashMap<&str, Vec<String>> = HashMap::new();

        rows.iter().for_each(|row| {
            if let Some(vec) = commands_roles.get_mut(&row.command.as_str()) {
                vec.push(format!("<@&{}>", row.role));
            } else {
                commands_roles.insert(&row.command, vec![format!("<@&{}>", row.role)]);
            }
        });

        let fields = commands_roles
            .iter()
            .map(|(key, value)| (key.to_title_case(), value.join("\n"), true));

        let title = lm.get(&language, "restrict/title");

        let _ = msg
            .channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| e.title(title).fields(fields).color(*THEME_COLOR))
            })
            .await;
    } else {
        let prefix = ctx.prefix(msg.guild_id).await;

        command_help(ctx, msg, lm, &prefix, &language, "restrict").await;
    }
}

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
