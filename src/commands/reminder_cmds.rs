use std::{
    collections::HashMap,
    default::Default,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use num_integer::Integer;
use regex_command_attr::command;
use serenity::{
    client::Context,
    futures::StreamExt,
    model::{
        channel::{Channel, Message},
        id::ChannelId,
        misc::Mentionable,
    },
};

use crate::{
    check_subscription_on_message,
    consts::{
        EMBED_DESCRIPTION_MAX_LENGTH, REGEX_CHANNEL_USER, REGEX_NATURAL_COMMAND_1,
        REGEX_NATURAL_COMMAND_2, REGEX_REMIND_COMMAND, THEME_COLOR,
    },
    framework::{CommandInvoke, CreateGenericResponse},
    models::{
        channel_data::ChannelData,
        guild_data::GuildData,
        reminder::{
            builder::{MultiReminderBuilder, ReminderScope},
            content::Content,
            look_flags::{LookFlags, TimeDisplayType},
            Reminder,
        },
        timer::Timer,
        user_data::UserData,
        CtxData,
    },
    time_parser::{natural_parser, TimeParser},
    SQLPool,
};

#[command("pause")]
#[description("Pause all reminders on the current channel until a certain time or indefinitely")]
#[arg(
    name = "until",
    description = "When to pause until (hint: try 'next Wednesday', or '10 minutes')",
    kind = "String",
    required = false
)]
#[supports_dm(false)]
#[required_permissions(Restricted)]
async fn pause(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let timezone = UserData::timezone_of(&invoke.author_id(), &pool).await;

    let mut channel = ctx.channel_data(invoke.channel_id()).await.unwrap();

    match args.get("until") {
        Some(until) => {
            let parsed = natural_parser(until, &timezone.to_string()).await;

            if let Some(timestamp) = parsed {
                let dt = NaiveDateTime::from_timestamp(timestamp, 0);

                channel.paused = true;
                channel.paused_until = Some(dt);

                channel.commit_changes(&pool).await;

                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().content(format!(
                            "Reminders in this channel have been silenced until **<t:{}:D>**",
                            timestamp
                        )),
                    )
                    .await;
            } else {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new()
                            .content("Time could not be processed. Please write the time as clearly as possible"),
                    )
                    .await;
            }
        }
        None => {
            channel.paused = !channel.paused;
            channel.paused_until = None;

            channel.commit_changes(&pool).await;

            if channel.paused {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new()
                            .content("Reminders in this channel have been silenced indefinitely"),
                    )
                    .await;
            } else {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new()
                            .content("Reminders in this channel have been unsilenced"),
                    )
                    .await;
            }
        }
    }
}

#[command("offset")]
#[description("Move all reminders in the current server by a certain amount of time. Times get added together")]
#[arg(
    name = "hours",
    description = "Number of hours to offset by",
    kind = "Integer",
    required = false
)]
#[arg(
    name = "minutes",
    description = "Number of minutes to offset by",
    kind = "Integer",
    required = false
)]
#[arg(
    name = "seconds",
    description = "Number of seconds to offset by",
    kind = "Integer",
    required = false
)]
#[required_permissions(Restricted)]
async fn offset(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let combined_time = args.get("hours").map_or(0, |h| h.parse::<i64>().unwrap() * 3600)
        + args.get("minutes").map_or(0, |m| m.parse::<i64>().unwrap() * 60)
        + args.get("seconds").map_or(0, |s| s.parse::<i64>().unwrap());

    if combined_time == 0 {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content("Please specify one of `hours`, `minutes` or `seconds`"),
            )
            .await;
    } else {
        if let Some(guild) = invoke.guild(ctx.cache.clone()) {
            let channels = guild
                .channels
                .iter()
                .filter(|(channel_id, channel)| match channel {
                    Channel::Guild(guild_channel) => guild_channel.is_text_based(),
                    _ => false,
                })
                .map(|(id, _)| id.0.to_string())
                .collect::<Vec<String>>()
                .join(",");

            sqlx::query!(
                "
UPDATE reminders
INNER JOIN
    `channels` ON `channels`.id = reminders.channel_id
SET reminders.`utc_time` = reminders.`utc_time` + ?
WHERE FIND_IN_SET(channels.`channel`, ?)",
                combined_time,
                channels
            )
            .execute(&pool)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "UPDATE reminders INNER JOIN `channels` ON `channels`.id = reminders.channel_id SET reminders.`utc_time` = reminders.`utc_time` + ? WHERE channels.`channel` = ?",
                combined_time,
                invoke.channel_id().0
            )
            .execute(&pool)
            .await
            .unwrap();
        }

        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .content(format!("All reminders offset by {} seconds", combined_time)),
            )
            .await;
    }
}

#[command("nudge")]
#[description("Nudge all future reminders on this channel by a certain amount (don't use for DST! See `/offset`)")]
#[arg(
    name = "minutes",
    description = "Number of minutes to nudge new reminders by",
    kind = "Integer",
    required = false
)]
#[arg(
    name = "seconds",
    description = "Number of seconds to nudge new reminders by",
    kind = "Integer",
    required = false
)]
#[required_permissions(Restricted)]
async fn nudge(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let combined_time = args.get("minutes").map_or(0, |m| m.parse::<i64>().unwrap() * 60)
        + args.get("seconds").map_or(0, |s| s.parse::<i64>().unwrap());

    if combined_time < i16::MIN as i64 || combined_time > i16::MAX as i64 {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new().content("Nudge times must be less than 500 minutes"),
            )
            .await;
    } else {
        let mut channel_data = ctx.channel_data(invoke.channel_id()).await.unwrap();

        channel_data.nudge = combined_time as i16;
        channel_data.commit_changes(&pool).await;

        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new().content(format!(
                    "Future reminders will be nudged by {} seconds",
                    combined_time
                )),
            )
            .await;
    }
}

#[command("look")]
#[description("View reminders on a specific channel")]
#[arg(
    name = "channel",
    description = "The channel to view reminders on",
    kind = "Channel",
    required = false
)]
#[arg(
    name = "disabled",
    description = "Whether to show disabled reminders or not",
    kind = "Boolean",
    required = false
)]
#[arg(
    name = "relative",
    description = "Whether to display times as relative or exact times",
    kind = "Boolean",
    required = false
)]
#[required_permissions(Managed)]
async fn look(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let timezone = UserData::timezone_of(&invoke.author_id(), &pool).await;

    let flags = LookFlags {
        show_disabled: args.get("disabled").map(|b| b == "true").unwrap_or(true),
        channel_id: args.get("channel").map(|c| ChannelId(c.parse::<u64>().unwrap())),
        time_display: args.get("relative").map_or(TimeDisplayType::Relative, |b| {
            if b == "true" {
                TimeDisplayType::Relative
            } else {
                TimeDisplayType::Absolute
            }
        }),
    };

    let channel_opt = invoke.channel_id().to_channel_cached(&ctx);

    let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
        if Some(channel.guild_id) == invoke.guild_id() {
            flags.channel_id.unwrap_or(invoke.channel_id())
        } else {
            invoke.channel_id()
        }
    } else {
        invoke.channel_id()
    };

    let reminders = Reminder::from_channel(ctx, channel_id, &flags).await;

    if reminders.is_empty() {
        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new().content("No reminders on specified channel"),
            )
            .await;
    } else {
        let mut char_count = 0;

        let display = reminders
            .iter()
            .map(|reminder| reminder.display(&flags, &timezone))
            .take_while(|p| {
                char_count += p.len();

                char_count < EMBED_DESCRIPTION_MAX_LENGTH
            })
            .collect::<Vec<String>>()
            .join("\n");

        let pages = reminders
            .iter()
            .map(|reminder| reminder.display(&flags, &timezone))
            .fold(0, |t, r| t + r.len())
            .div_ceil(EMBED_DESCRIPTION_MAX_LENGTH);

        let _ = invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .embed(|e| {
                        e.title(format!("Reminders on {}", channel_id.mention()))
                            .description(display)
                            .footer(|f| f.text(format!("Page {} of {}", 1, pages)))
                    })
                    .components(|comp| {
                        comp.create_action_row(|row| {
                            row.create_button(|b| b.label("⏮️").custom_id(".1"))
                                .create_button(|b| b.label("◀️").custom_id(".2"))
                                .create_button(|b| b.label("▶️").custom_id(".3"))
                                .create_button(|b| b.label("⏭️").custom_id(".4"))
                        })
                    }),
            )
            .await;
    }
}

/*
#[command("del")]
#[permission_level(Managed)]
async fn delete(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    let _ = msg.channel_id.say(&ctx, lm.get(&user_data.language, "del/listing")).await;

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
    let _ = msg.channel_id.say(&ctx, lm.get(&user_data.language, "del/listed")).await;

    let reply =
        msg.channel_id.await_reply(&ctx).author_id(msg.author.id).channel_id(msg.channel_id).await;

    if let Some(content) = reply.map(|m| m.content.replace(",", " ")) {
        let parts = content.split(' ').filter(|i| !i.is_empty()).collect::<Vec<&str>>();

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
            let content = lm.get(&user_data.language, "del/count").replacen("{}", "0", 1);

            let _ = msg.channel_id.say(&ctx, content).await;
        }
    }
}
*/

#[command("timer")]
#[description("Manage timers")]
#[subcommand("list")]
#[description("List the timers in this server or DM channel")]
#[subcommand("start")]
#[description("Start a new timer from now")]
#[arg(name = "name", description = "Name for the new timer", kind = "String", required = true)]
#[subcommand("delete")]
#[description("Delete a timer")]
#[arg(name = "name", description = "Name of the timer to delete", kind = "String", required = true)]
#[required_permissions(Managed)]
async fn timer(
    ctx: &Context,
    invoke: &(dyn CommandInvoke + Send + Sync),
    args: HashMap<String, String>,
) {
    fn time_difference(start_time: NaiveDateTime) -> String {
        let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let now = NaiveDateTime::from_timestamp(unix_time, 0);

        let delta = (now - start_time).num_seconds();

        let (minutes, seconds) = delta.div_rem(&60);
        let (hours, minutes) = minutes.div_rem(&60);
        let (days, hours) = hours.div_rem(&24);

        format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    }

    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let owner = invoke.guild_id().map(|g| g.0).unwrap_or_else(|| invoke.author_id().0);

    match args.get("").map(|s| s.as_str()) {
        Some("start") => {
            let count = Timer::count_from_owner(owner, &pool).await;

            if count >= 25 {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new()
                            .content("You already have 25 timers. Please delete some timers before creating a new one"),
                    )
                    .await;
            } else {
                let name = args.get("name").unwrap();

                if name.len() <= 32 {
                    Timer::create(&name, owner, &pool).await;

                    let _ = invoke
                        .respond(
                            ctx.http.clone(),
                            CreateGenericResponse::new().content("Created a new timer"),
                        )
                        .await;
                } else {
                    let _ = invoke
                        .respond(
                            ctx.http.clone(),
                            CreateGenericResponse::new()
                                .content(format!("Please name your timer something shorted (max. 32 characters, you used {})", name.len())),
                        )
                        .await;
                }
            }
        }
        Some("delete") => {
            let name = args.get("name").unwrap();

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

                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().content("Deleted a timer"),
                    )
                    .await;
            } else {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().content("Could not find a timer by that name"),
                    )
                    .await;
            }
        }
        Some("list") => {
            let timers = Timer::from_owner(owner, &pool).await;

            if timers.len() > 0 {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().embed(|e| {
                            e.fields(timers.iter().map(|timer| {
                                (
                                    &timer.name,
                                    format!("⌚ `{}`", time_difference(timer.start_time)),
                                    false,
                                )
                            }))
                            .color(*THEME_COLOR)
                        }),
                    )
                    .await;
            } else {
                let _ = invoke
                    .respond(
                        ctx.http.clone(),
                        CreateGenericResponse::new().content(
                            "No timers currently. Use `/timer start` to create a new timer",
                        ),
                    )
                    .await;
            }
        }
        _ => {}
    }
}

/*
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

            let expires_parser =
                captures.name("expires").map(|mat| TimeParser::new(mat.as_str(), timezone));

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
*/
