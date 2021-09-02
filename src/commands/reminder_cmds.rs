use regex_command_attr::command;

use serenity::{
    client::Context,
    model::{channel::Channel, channel::Message},
};

use crate::{
    check_subscription_on_message, command_help,
    consts::{
        REGEX_CHANNEL_USER, REGEX_NATURAL_COMMAND_1, REGEX_NATURAL_COMMAND_2, REGEX_REMIND_COMMAND,
        THEME_COLOR,
    },
    framework::SendIterator,
    get_ctx_data,
    models::{
        channel_data::ChannelData,
        guild_data::GuildData,
        reminder::{builder::ReminderScope, content::Content, look_flags::LookFlags, Reminder},
        timer::Timer,
        user_data::UserData,
        CtxData,
    },
    time_parser::{natural_parser, TimeParser},
};

use chrono::NaiveDateTime;

use num_integer::Integer;

use crate::models::reminder::builder::MultiReminderBuilder;
use std::{
    default::Default,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn pause(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;

    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();

    if args.is_empty() {
        channel.paused = !channel.paused;
        channel.paused_until = None;

        channel.commit_changes(&pool).await;

        if channel.paused {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "pause/paused_indefinite"))
                .await;
        } else {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "pause/unpaused"))
                .await;
        }
    } else {
        let parser = TimeParser::new(&args, timezone);
        let pause_until = parser.timestamp();

        match pause_until {
            Ok(timestamp) => {
                let dt = NaiveDateTime::from_timestamp(timestamp, 0);

                channel.paused = true;
                channel.paused_until = Some(dt);

                channel.commit_changes(&pool).await;

                let content = lm
                    .get(&language, "pause/paused_until")
                    .replace("{}", &format!("<t:{}:D>", timestamp));

                let _ = msg.channel_id.say(&ctx, content).await;
            }

            Err(_) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "pause/invalid_time"))
                    .await;
            }
        }
    }
}

#[command]
#[permission_level(Restricted)]
async fn offset(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    if args.is_empty() {
        let prefix = ctx.prefix(msg.guild_id).await;

        command_help(ctx, msg, lm, &prefix, &user_data.language, "offset").await;
    } else {
        let parser = TimeParser::new(&args, user_data.timezone());

        if let Ok(displacement) = parser.displacement() {
            if let Some(guild) = msg.guild(&ctx) {
                let guild_data = GuildData::from_guild(guild, &pool).await.unwrap();

                sqlx::query!(
                    "
UPDATE reminders
    INNER JOIN `channels`
        ON `channels`.id = reminders.channel_id
    SET
        reminders.`utc_time` = reminders.`utc_time` + ?
    WHERE channels.guild_id = ?
                    ",
                    displacement,
                    guild_data.id
                )
                .execute(&pool)
                .await
                .unwrap();
            } else {
                sqlx::query!(
                    "
UPDATE reminders SET `utc_time` = `utc_time` + ? WHERE reminders.channel_id = ?
                    ",
                    displacement,
                    user_data.dm_channel
                )
                .execute(&pool)
                .await
                .unwrap();
            }

            let response = lm.get(&user_data.language, "offset/success").replacen(
                "{}",
                &displacement.to_string(),
                1,
            );

            let _ = msg.channel_id.say(&ctx, response).await;
        } else {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&user_data.language, "offset/invalid_time"))
                .await;
        }
    }
}

#[command]
#[permission_level(Restricted)]
async fn nudge(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;

    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();

    if args.is_empty() {
        let content = lm
            .get(&language, "nudge/no_argument")
            .replace("{nudge}", &format!("{}s", &channel.nudge.to_string()));

        let _ = msg.channel_id.say(&ctx, content).await;
    } else {
        let parser = TimeParser::new(&args, timezone);
        let nudge_time = parser.displacement();

        match nudge_time {
            Ok(displacement) => {
                if displacement < i16::MIN as i64 || displacement > i16::MAX as i64 {
                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "nudge/invalid_time"))
                        .await;
                } else {
                    channel.nudge = displacement as i16;

                    channel.commit_changes(&pool).await;

                    let response = lm.get(&language, "nudge/success").replacen(
                        "{}",
                        &displacement.to_string(),
                        1,
                    );

                    let _ = msg.channel_id.say(&ctx, response).await;
                }
            }

            Err(_) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "nudge/invalid_time"))
                    .await;
            }
        }
    }
}

#[command("look")]
#[permission_level(Managed)]
async fn look(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let flags = LookFlags::from_string(&args);

    let channel_opt = msg.channel_id.to_channel_cached(&ctx);

    let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
        if Some(channel.guild_id) == msg.guild_id {
            flags.channel_id.unwrap_or(msg.channel_id)
        } else {
            msg.channel_id
        }
    } else {
        msg.channel_id
    };

    let reminders = Reminder::from_channel(ctx, channel_id, &flags).await;

    if reminders.is_empty() {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "look/no_reminders"))
            .await;
    } else {
        let inter = lm.get(&language, "look/inter");

        let display = reminders
            .iter()
            .map(|reminder| reminder.display(&flags, inter));

        let _ = msg.channel_id.say_lines(&ctx, display).await;
    }
}

#[command("del")]
#[permission_level(Managed)]
async fn delete(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    let _ = msg
        .channel_id
        .say(&ctx, lm.get(&user_data.language, "del/listing"))
        .await;

    let mut reminder_ids: Vec<u32> = vec![];

    let reminders = Reminder::from_guild(ctx, msg.guild_id, msg.author.id).await;

    let enumerated_reminders = reminders.iter().enumerate().map(|(count, reminder)| {
        reminder_ids.push(reminder.id);

        format!(
            "**{}**: '{}' *<#{}>* at <t:{}>",
            count + 1,
            reminder.display_content(),
            reminder.channel,
            reminder.utc_time.timestamp()
        )
    });

    let _ = msg.channel_id.say_lines(&ctx, enumerated_reminders).await;
    let _ = msg
        .channel_id
        .say(&ctx, lm.get(&user_data.language, "del/listed"))
        .await;

    let reply = msg
        .channel_id
        .await_reply(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .await;

    if let Some(content) = reply.map(|m| m.content.replace(",", " ")) {
        let parts = content
            .split(' ')
            .filter(|i| !i.is_empty())
            .collect::<Vec<&str>>();

        let valid_parts = parts
            .iter()
            .filter_map(|i| {
                i.parse::<usize>()
                    .ok()
                    .filter(|val| val > &0)
                    .map(|val| reminder_ids.get(val - 1))
                    .flatten()
            })
            .map(|item| item.to_string())
            .collect::<Vec<String>>();

        if parts.len() == valid_parts.len() {
            let joined = valid_parts.join(",");

            let count_row = sqlx::query!(
                "
SELECT COUNT(1) AS count FROM reminders WHERE FIND_IN_SET(id, ?)
                ",
                joined
            )
            .fetch_one(&pool)
            .await
            .unwrap();

            sqlx::query!(
                "
DELETE FROM reminders WHERE FIND_IN_SET(id, ?)
                ",
                joined
            )
            .execute(&pool)
            .await
            .unwrap();

            if let Some(guild_id) = msg.guild_id {
                let _ = sqlx::query!(
                    "
INSERT INTO events (event_name, bulk_count, guild_id, user_id) VALUES ('delete', ?, ?, ?)
                    ",
                    count_row.count,
                    guild_id.as_u64(),
                    user_data.id
                )
                .execute(&pool)
                .await;
            }

            let content = lm.get(&user_data.language, "del/count").replacen(
                "{}",
                &count_row.count.to_string(),
                1,
            );

            let _ = msg.channel_id.say(&ctx, content).await;
        } else {
            let content = lm
                .get(&user_data.language, "del/count")
                .replacen("{}", "0", 1);

            let _ = msg.channel_id.say(&ctx, content).await;
        }
    }
}

#[command("timer")]
#[permission_level(Managed)]
async fn timer(ctx: &Context, msg: &Message, args: String) {
    fn time_difference(start_time: NaiveDateTime) -> String {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let now = NaiveDateTime::from_timestamp(unix_time, 0);

        let delta = (now - start_time).num_seconds();

        let (minutes, seconds) = delta.div_rem(&60);
        let (hours, minutes) = minutes.div_rem(&60);
        let (days, hours) = hours.div_rem(&24);

        format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    }

    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let mut args_iter = args.splitn(2, ' ');

    let owner = msg
        .guild_id
        .map(|g| g.as_u64().to_owned())
        .unwrap_or_else(|| msg.author.id.as_u64().to_owned());

    match args_iter.next() {
        Some("list") => {
            let timers = Timer::from_owner(owner, &pool).await;

            let _ = msg
                .channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.fields(timers.iter().map(|timer| {
                            (
                                &timer.name,
                                format!("⏳ `{}`", time_difference(timer.start_time)),
                                false,
                            )
                        }))
                    })
                })
                .await;
        }

        Some("start") => {
            let count = Timer::count_from_owner(owner, &pool).await;

            if count >= 25 {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "timer/limit"))
                    .await;
            } else {
                let name = args_iter
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or(format!("New timer #{}", count + 1));

                if name.len() <= 32 {
                    Timer::create(&name, owner, &pool).await;

                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/success"))
                        .await;
                } else {
                    let _ = msg
                        .channel_id
                        .say(
                            &ctx,
                            lm.get(&language, "timer/name_length")
                                .replace("{}", &name.len().to_string()),
                        )
                        .await;
                }
            }
        }

        Some("delete") => {
            if let Some(name) = args_iter.next() {
                let exists = sqlx::query!(
                    "
SELECT 1 as _r FROM timers WHERE owner = ? AND name = ?
                    ",
                    owner,
                    name
                )
                .fetch_one(&pool)
                .await;

                if exists.is_ok() {
                    sqlx::query!(
                        "
DELETE FROM timers WHERE owner = ? AND name = ?
                        ",
                        owner,
                        name
                    )
                    .execute(&pool)
                    .await
                    .unwrap();

                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/deleted"))
                        .await;
                } else {
                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/not_found"))
                        .await;
                }
            } else {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "timer/help"))
                    .await;
            }
        }

        _ => {
            let prefix = ctx.prefix(msg.guild_id).await;

            command_help(ctx, msg, lm, &prefix, &language, "timer").await;
        }
    }
}

#[derive(PartialEq)]
enum RemindCommand {
    Remind,
    Interval,
}

#[command("remind")]
#[permission_level(Managed)]
async fn remind(ctx: &Context, msg: &Message, args: String) {
    remind_command(ctx, msg, args, RemindCommand::Remind).await;
}

#[command("interval")]
#[permission_level(Managed)]
async fn interval(ctx: &Context, msg: &Message, args: String) {
    remind_command(ctx, msg, args, RemindCommand::Interval).await;
}

fn parse_mention_list(mentions: &str) -> Vec<ReminderScope> {
    REGEX_CHANNEL_USER
        .captures_iter(mentions)
        .map(|i| {
            let pref = i.get(1).unwrap().as_str();
            let id = i.get(2).unwrap().as_str().parse::<u64>().unwrap();

            if pref == "#" {
                ReminderScope::Channel(id)
            } else {
                ReminderScope::User(id)
            }
        })
        .collect::<Vec<ReminderScope>>()
}

async fn remind_command(ctx: &Context, msg: &Message, args: String, command: RemindCommand) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let timezone = UserData::timezone_of(&msg.author, &pool).await;
    let language = UserData::language_of(&msg.author, &pool).await;

    match REGEX_REMIND_COMMAND.captures(&args) {
        Some(captures) => {
            let parsed = parse_mention_list(captures.name("mentions").unwrap().as_str());

            let scopes = if parsed.is_empty() {
                vec![ReminderScope::Channel(msg.channel_id.into())]
            } else {
                parsed
            };

            let time_parser = TimeParser::new(captures.name("time").unwrap().as_str(), timezone);

            let expires_parser = captures
                .name("expires")
                .map(|mat| TimeParser::new(mat.as_str(), timezone));

            let interval_parser = captures
                .name("interval")
                .map(|mat| TimeParser::new(mat.as_str(), timezone))
                .map(|parser| parser.displacement())
                .transpose();

            if let Ok(interval) = interval_parser {
                if interval.is_some() && !check_subscription_on_message(&ctx, msg).await {
                    // no patreon
                    let _ = msg
                        .channel_id
                        .say(
                            &ctx,
                            lm.get(&language, "interval/donor")
                                .replace("{prefix}", &ctx.prefix(msg.guild_id).await),
                        )
                        .await;
                } else {
                    let content_res = Content::build(
                        captures.name("content").map(|mat| mat.as_str()).unwrap(),
                        msg,
                    )
                    .await;

                    match content_res {
                        Ok(mut content) => {
                            if let Some(guild) = msg.guild(&ctx) {
                                content.substitute(guild);
                            }

                            let user_data = ctx.user_data(&msg.author).await.unwrap();

                            let mut builder = MultiReminderBuilder::new(ctx, msg.guild_id)
                                .author(user_data)
                                .content(content)
                                .interval(interval)
                                .expires_parser(expires_parser)
                                .time_parser(time_parser.clone());

                            builder.set_scopes(scopes);

                            let (errors, successes) = builder.build().await;

                            let success_part = match successes.len() {
                                0 => "".to_string(),
                                n => format!(
                                    "Reminder{s} for {locations} set for <t:{offset}:R>",
                                    s = if n > 1 { "s" } else { "" },
                                    locations = successes
                                        .iter()
                                        .map(|l| l.mention())
                                        .collect::<Vec<String>>()
                                        .join(", "),
                                    offset = time_parser.timestamp().unwrap()
                                ),
                            };

                            let error_part = match errors.len() {
                                0 => "".to_string(),
                                n => format!(
                                    "{n} reminder{s} failed to set:\n{errors}",
                                    s = if n > 1 { "s" } else { "" },
                                    n = n,
                                    errors = errors
                                        .iter()
                                        .map(|e| e.display(false))
                                        .collect::<Vec<String>>()
                                        .join("\n")
                                ),
                            };

                            let _ = msg
                                .channel_id
                                .send_message(&ctx, |m| {
                                    m.embed(|e| {
                                        e.title(format!(
                                            "{n} Reminder{s} Set",
                                            n = successes.len(),
                                            s = if successes.len() > 1 { "s" } else { "" }
                                        ))
                                        .description(format!("{}\n\n{}", success_part, error_part))
                                        .color(*THEME_COLOR)
                                    })
                                })
                                .await;
                        }

                        Err(content_error) => {
                            let _ = msg
                                .channel_id
                                .send_message(ctx, |m| {
                                    m.embed(move |e| {
                                        e.title("0 Reminders Set")
                                            .description(content_error.to_string())
                                            .color(*THEME_COLOR)
                                    })
                                })
                                .await;
                        }
                    }
                }
            } else {
                let _ = msg
                    .channel_id
                    .send_message(ctx, |m| {
                        m.embed(move |e| {
                            e.title(lm.get(&language, "remind/title").replace("{number}", "0"))
                                .description(lm.get(&language, "interval/invalid_interval"))
                                .color(*THEME_COLOR)
                        })
                    })
                    .await;
            }
        }

        None => {
            let prefix = ctx.prefix(msg.guild_id).await;

            match command {
                RemindCommand::Remind => {
                    command_help(ctx, msg, lm, &prefix, &language, "remind").await
                }

                RemindCommand::Interval => {
                    command_help(ctx, msg, lm, &prefix, &language, "interval").await
                }
            }
        }
    }
}

#[command("natural")]
#[permission_level(Managed)]
async fn natural(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    match REGEX_NATURAL_COMMAND_1.captures(&args) {
        Some(captures) => {
            let (expires, interval, string_content) =
                if check_subscription_on_message(&ctx, msg).await {
                    let rest_content = captures.name("msg").unwrap().as_str();

                    match REGEX_NATURAL_COMMAND_2.captures(rest_content) {
                        Some(secondary_captures) => {
                            let expires =
                                if let Some(expires_crop) = secondary_captures.name("expires") {
                                    natural_parser(expires_crop.as_str(), &user_data.timezone).await
                                } else {
                                    None
                                };

                            let interval =
                                if let Some(interval_crop) = secondary_captures.name("interval") {
                                    humantime::parse_duration(interval_crop.as_str())
                                        .or_else(|_| {
                                            humantime::parse_duration(&format!(
                                                "1 {}",
                                                interval_crop.as_str()
                                            ))
                                        })
                                        .map(|duration| duration.as_secs() as i64)
                                        .ok()
                                } else {
                                    None
                                };

                            (
                                expires,
                                interval,
                                if interval.is_some() {
                                    secondary_captures.name("msg").unwrap().as_str()
                                } else {
                                    rest_content
                                },
                            )
                        }

                        None => (None, None, rest_content),
                    }
                } else {
                    (None, None, captures.name("msg").unwrap().as_str())
                };

            let location_ids = if let Some(mentions) = captures.name("mentions").map(|m| m.as_str())
            {
                parse_mention_list(mentions)
            } else {
                vec![ReminderScope::Channel(msg.channel_id.into())]
            };

            if let Some(timestamp) =
                natural_parser(captures.name("time").unwrap().as_str(), &user_data.timezone).await
            {
                let content_res = Content::build(string_content, msg).await;

                match content_res {
                    Ok(mut content) => {
                        if let Some(guild) = msg.guild(&ctx) {
                            content.substitute(guild);
                        }

                        let user_data = ctx.user_data(&msg.author).await.unwrap();

                        let mut builder = MultiReminderBuilder::new(ctx, msg.guild_id)
                            .author(user_data)
                            .content(content)
                            .interval(interval)
                            .expires(expires)
                            .time(timestamp);

                        builder.set_scopes(location_ids);

                        let (errors, successes) = builder.build().await;

                        let success_part = match successes.len() {
                            0 => "".to_string(),
                            n => format!(
                                "Reminder{s} for {locations} set for <t:{offset}:R>",
                                s = if n > 1 { "s" } else { "" },
                                locations = successes
                                    .iter()
                                    .map(|l| l.mention())
                                    .collect::<Vec<String>>()
                                    .join(", "),
                                offset = timestamp
                            ),
                        };

                        let error_part = match errors.len() {
                            0 => "".to_string(),
                            n => format!(
                                "{n} reminder{s} failed to set:\n{errors}",
                                s = if n > 1 { "s" } else { "" },
                                n = n,
                                errors = errors
                                    .iter()
                                    .map(|e| e.display(true))
                                    .collect::<Vec<String>>()
                                    .join("\n")
                            ),
                        };

                        let _ = msg
                            .channel_id
                            .send_message(&ctx, |m| {
                                m.embed(|e| {
                                    e.title(format!(
                                        "{n} Reminder{s} Set",
                                        n = successes.len(),
                                        s = if successes.len() > 1 { "s" } else { "" }
                                    ))
                                    .description(format!("{}\n\n{}", success_part, error_part))
                                    .color(*THEME_COLOR)
                                })
                            })
                            .await;
                    }

                    Err(content_error) => {
                        let _ = msg
                            .channel_id
                            .send_message(ctx, |m| {
                                m.embed(move |e| {
                                    e.title("0 Reminders Set")
                                        .description(content_error.to_string())
                                        .color(*THEME_COLOR)
                                })
                            })
                            .await;
                    }
                }
            } else {
                let _ = msg
                    .channel_id
                    .send_message(ctx, |m| {
                        m.embed(move |e| {
                            e.title(
                                lm.get(&user_data.language, "remind/title")
                                    .replace("{number}", "0"),
                            )
                            .description(lm.get(&user_data.language, "natural/invalid_time"))
                            .color(*THEME_COLOR)
                        })
                    })
                    .await;
            }
        }

        None => {
            command_help(
                ctx,
                msg,
                lm,
                &ctx.prefix(msg.guild_id).await,
                &user_data.language,
                "natural",
            )
            .await;
        }
    }
}
