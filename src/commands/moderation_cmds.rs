use regex_command_attr::command;

use serenity::{
    client::Context,
    framework::Framework,
    model::{channel::Message, id::RoleId},
};

use chrono_tz::Tz;

use chrono::offset::Utc;

use inflector::Inflector;

use crate::{
    consts::{REGEX_ALIAS, REGEX_CHANNEL, REGEX_COMMANDS, REGEX_ROLE},
    framework::SendIterator,
    models::{ChannelData, GuildData, UserData},
    FrameworkCtx, SQLPool,
};

use crate::language_manager::LanguageManager;
use serenity::model::id::ChannelId;
use std::iter;

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
#[can_blacklist(false)]
async fn blacklist(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

    let language = UserData::language_of(&msg.author, &pool).await;

    let capture_opt = REGEX_CHANNEL
        .captures(&args)
        .map(|cap| cap.get(1))
        .flatten();

    let (channel, local) = match capture_opt {
        Some(capture) => (
            ChannelId(capture.as_str().parse::<u64>().unwrap())
                .to_channel_cached(&ctx)
                .await,
            false,
        ),

        None => (msg.channel(&ctx).await, true),
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
    } else {
        if local {
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
}

#[command]
async fn timezone(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

    let mut user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

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

                let _ = msg.channel_id.say(&ctx, content).await;
            }

            Err(_) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&user_data.language, "timezone/no_timezone"))
                    .await;
            }
        }
    } else {
        let content = lm
            .get(&user_data.language, "timezone/no_argument")
            .replace(
                "{prefix}",
                &GuildData::prefix_from_id(msg.guild_id, &pool).await,
            )
            .replacen("{timezone}", &user_data.timezone, 1);

        let _ = msg.channel_id.say(&ctx, content).await;
    }
}

#[command]
async fn language(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

    let mut user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    match lm.get_language(&args) {
        Some(row) => {
            user_data.language = row.to_string();

            user_data.commit_changes(&pool).await;

            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&user_data.language, "lang/set_p"))
                .await;
        }

        None => {
            let language_codes = lm
                .all_languages()
                .map(|(k, v)| format!("{} ({})", v.to_title_case(), k.to_uppercase()))
                .collect::<Vec<String>>()
                .join("\n");

            let content =
                lm.get(&user_data.language, "lang/invalid")
                    .replacen("{}", &language_codes, 1);

            let _ = msg.channel_id.say(&ctx, content).await;
        }
    }
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn prefix(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

    let mut guild_data = GuildData::from_guild(msg.guild(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();
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
        guild_data.prefix = args;
        guild_data.commit_changes(&pool).await;

        let content =
            lm.get(&language, "prefix/success")
                .replacen("{prefix}", &guild_data.prefix, 1);

        let _ = msg.channel_id.say(&ctx, content).await;
    }
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn restrict(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

    let language = UserData::language_of(&msg.author, &pool).await;
    let guild_data = GuildData::from_guild(msg.guild(&ctx).await.unwrap(), &pool)
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

        let role_opt = role_id.to_role_cached(&ctx).await;

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

        let display_inner = rows
            .iter()
            .map(|row| format!("<@&{}> can use {}", row.role, row.command))
            .collect::<Vec<String>>()
            .join("\n");
        let display = lm
            .get(&language, "restrict/allowed")
            .replacen("{}", &display_inner, 1);

        let _ = msg.channel_id.say(&ctx, display).await;
    } else {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "restrict/help"))
            .await;
    }
}

#[command("alias")]
#[supports_dm(false)]
#[permission_level(Managed)]
async fn alias(ctx: &Context, msg: &Message, args: String) {
    let pool;
    let lm;

    {
        let data = ctx.data.read().await;

        pool = data
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        lm = data.get::<LanguageManager>().cloned().unwrap();
    }

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
                            new_msg.content = format!("<@{}> {}", &ctx.cache.current_user_id().await, row.command);

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
        let prefix = GuildData::prefix_from_id(msg.guild_id, &pool).await;
        let content = lm.get(&language, "alias/help").replace("{prefix}", &prefix);

        let _ = msg.channel_id.say(&ctx, content).await;
    }
}
