use std::{
    collections::HashSet,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use chrono_tz::Tz;
use num_integer::Integer;
use regex_command_attr::command;
use serenity::{builder::CreateEmbed, client::Context, model::channel::Channel};

use crate::{
    component_models::{
        pager::{DelPager, LookPager, Pager},
        ComponentDataModel, DelSelector,
    },
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, REGEX_CHANNEL_USER, SELECT_MAX_ENTRIES, THEME_COLOR},
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse, OptionValue},
    hooks::CHECK_GUILD_PERMISSIONS_HOOK,
    interval_parser::parse_duration,
    models::{
        reminder::{
            builder::{MultiReminderBuilder, ReminderScope},
            content::Content,
            errors::ReminderError,
            look_flags::{LookFlags, TimeDisplayType},
            Reminder,
        },
        timer::Timer,
        user_data::UserData,
        CtxData,
    },
    time_parser::natural_parser,
    utils::{check_guild_subscription, check_subscription},
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn pause(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let timezone = UserData::timezone_of(&invoke.author_id(), &pool).await;

    let mut channel = ctx.channel_data(invoke.channel_id()).await.unwrap();

    match args.get("until") {
        Some(OptionValue::String(until)) => {
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
        _ => {
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn offset(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let combined_time = args.get("hours").map_or(0, |h| h.as_i64().unwrap() * 3600)
        + args.get("minutes").map_or(0, |m| m.as_i64().unwrap() * 60)
        + args.get("seconds").map_or(0, |s| s.as_i64().unwrap());

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
                .filter(|(_, channel)| match channel {
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
SET reminders.`utc_time` = DATE_ADD(reminders.`utc_time`, INTERVAL ? SECOND)
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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn nudge(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let combined_time = args.get("minutes").map_or(0, |m| m.as_i64().unwrap() * 60)
        + args.get("seconds").map_or(0, |s| s.as_i64().unwrap());

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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn look(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

    let timezone = UserData::timezone_of(&invoke.author_id(), &pool).await;

    let flags = LookFlags {
        show_disabled: args.get("disabled").map(|i| i.as_bool()).flatten().unwrap_or(true),
        channel_id: args.get("channel").map(|i| i.as_channel_id()).flatten(),
        time_display: args.get("relative").map_or(TimeDisplayType::Relative, |b| {
            if b.as_bool() == Some(true) {
                TimeDisplayType::Relative
            } else {
                TimeDisplayType::Absolute
            }
        }),
    };

    let channel_opt = invoke.channel_id().to_channel_cached(&ctx);

    let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
        if Some(channel.guild_id) == invoke.guild_id() {
            flags.channel_id.unwrap_or_else(|| invoke.channel_id())
        } else {
            invoke.channel_id()
        }
    } else {
        invoke.channel_id()
    };

    let channel_name = if let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx) {
        Some(channel.name)
    } else {
        None
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

        let pager = LookPager::new(flags, timezone);

        invoke
            .respond(
                ctx.http.clone(),
                CreateGenericResponse::new()
                    .embed(|e| {
                        e.title(format!(
                            "Reminders{}",
                            channel_name.map_or(String::new(), |n| format!(" on #{}", n))
                        ))
                        .description(display)
                        .footer(|f| f.text(format!("Page {} of {}", 1, pages)))
                        .color(*THEME_COLOR)
                    })
                    .components(|comp| {
                        pager.create_button_row(pages, comp);

                        comp
                    }),
            )
            .await
            .unwrap();
    }
}

#[command("del")]
#[description("Delete reminders")]
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn delete(ctx: &Context, invoke: &mut CommandInvoke, _args: CommandOptions) {
    let timezone = ctx.timezone(invoke.author_id()).await;

    let reminders = Reminder::from_guild(ctx, invoke.guild_id(), invoke.author_id()).await;

    let resp = show_delete_page(&reminders, 0, timezone);

    let _ = invoke.respond(&ctx, resp).await;
}

pub fn max_delete_page(reminders: &[Reminder], timezone: &Tz) -> usize {
    let mut rows = 0;
    let mut char_count = 0;

    reminders
        .iter()
        .enumerate()
        .map(|(count, reminder)| reminder.display_del(count, timezone))
        .fold(1, |mut pages, reminder| {
            rows += 1;
            char_count += reminder.len();

            if char_count > EMBED_DESCRIPTION_MAX_LENGTH || rows > SELECT_MAX_ENTRIES {
                rows = 1;
                char_count = reminder.len();
                pages += 1;
            }

            pages
        })
}

pub fn show_delete_page(
    reminders: &[Reminder],
    page: usize,
    timezone: Tz,
) -> CreateGenericResponse {
    let pager = DelPager::new(page, timezone);

    if reminders.is_empty() {
        return CreateGenericResponse::new()
            .embed(|e| e.title("Delete Reminders").description("No Reminders").color(*THEME_COLOR))
            .components(|comp| {
                pager.create_button_row(0, comp);
                comp
            });
    }

    let pages = max_delete_page(reminders, &timezone);

    let mut page = page;
    if page >= pages {
        page = pages - 1;
    }

    let mut char_count = 0;
    let mut rows = 0;
    let mut skipped_rows = 0;
    let mut skipped_char_count = 0;
    let mut first_num = 0;

    let mut skipped_pages = 0;

    let (shown_reminders, display_vec): (Vec<&Reminder>, Vec<String>) = reminders
        .iter()
        .enumerate()
        .map(|(count, reminder)| (reminder, reminder.display_del(count, &timezone)))
        .skip_while(|(_, p)| {
            first_num += 1;
            skipped_rows += 1;
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH
                || skipped_rows > SELECT_MAX_ENTRIES
            {
                skipped_rows = 1;
                skipped_char_count = p.len();
                skipped_pages += 1;
            }

            skipped_pages < page
        })
        .take_while(|(_, p)| {
            rows += 1;
            char_count += p.len();

            char_count < EMBED_DESCRIPTION_MAX_LENGTH && rows <= SELECT_MAX_ENTRIES
        })
        .unzip();

    let display = display_vec.join("\n");

    let del_selector = ComponentDataModel::DelSelector(DelSelector { page, timezone });

    CreateGenericResponse::new()
        .embed(|e| {
            e.title("Delete Reminders")
                .description(display)
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        })
        .components(|comp| {
            pager.create_button_row(pages, comp);

            comp.create_action_row(|row| {
                row.create_select_menu(|menu| {
                    menu.custom_id(del_selector.to_custom_id()).options(|opt| {
                        for (count, reminder) in shown_reminders.iter().enumerate() {
                            opt.create_option(|o| {
                                o.label(count + first_num).value(reminder.id).description({
                                    let c = reminder.display_content();

                                    if c.len() > 100 {
                                        format!(
                                            "{}...",
                                            reminder
                                                .display_content()
                                                .chars()
                                                .take(97)
                                                .collect::<String>()
                                        )
                                    } else {
                                        c.to_string()
                                    }
                                })
                            });
                        }

                        opt
                    })
                })
            })
        })
}

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
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn timer(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
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

    match args.subcommand.clone().unwrap().as_str() {
        "start" => {
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
                let name = args.get("name").unwrap().to_string();

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
        "delete" => {
            let name = args.get("name").unwrap().to_string();

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
        "list" => {
            let timers = Timer::from_owner(owner, &pool).await;

            if !timers.is_empty() {
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

#[command("remind")]
#[description("Create a new reminder")]
#[arg(
    name = "time",
    description = "A description of the time to set the reminder for",
    kind = "String",
    required = true
)]
#[arg(
    name = "content",
    description = "The message content to send",
    kind = "String",
    required = true
)]
#[arg(
    name = "channels",
    description = "Channel or user mentions to set the reminder for",
    kind = "String",
    required = false
)]
#[arg(
    name = "interval",
    description = "(Patreon only) Time to wait before repeating the reminder. Leave blank for one-shot reminder",
    kind = "String",
    required = false
)]
#[arg(
    name = "expires",
    description = "(Patreon only) For repeating reminders, the time at which the reminder will stop sending",
    kind = "String",
    required = false
)]
#[arg(
    name = "tts",
    description = "Set the TTS flag on the reminder message (like the /tts command)",
    kind = "Boolean",
    required = false
)]
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn remind(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    if args.get("interval").is_none() && args.get("expires").is_some() {
        let _ = invoke
            .respond(
                &ctx,
                CreateGenericResponse::new().content("`expires` can only be used with `interval`"),
            )
            .await;

        return;
    }

    invoke.defer(&ctx).await;

    let user_data = ctx.user_data(invoke.author_id()).await.unwrap();
    let timezone = user_data.timezone();

    let time = {
        let time_str = args.get("time").unwrap().to_string();

        natural_parser(&time_str, &timezone.to_string()).await
    };

    match time {
        Some(time) => {
            let content = {
                let content = args.get("content").unwrap().to_string();
                let tts = args.get("tts").map_or(false, |arg| arg.as_bool().unwrap_or(false));

                Content { content, tts, attachment: None, attachment_name: None }
            };

            let scopes = {
                let list = args
                    .get("channels")
                    .map(|arg| parse_mention_list(&arg.to_string()))
                    .unwrap_or_default();

                if list.is_empty() {
                    if invoke.guild_id().is_some() {
                        vec![ReminderScope::Channel(invoke.channel_id().0)]
                    } else {
                        vec![ReminderScope::User(invoke.author_id().0)]
                    }
                } else {
                    list
                }
            };

            let (interval, expires) = if let Some(repeat) = args.get("interval") {
                if check_subscription(&ctx, invoke.author_id()).await
                    || (invoke.guild_id().is_some()
                        && check_guild_subscription(&ctx, invoke.guild_id().unwrap()).await)
                {
                    (
                        parse_duration(&repeat.to_string())
                            .or_else(|_| parse_duration(&format!("1 {}", repeat.to_string())))
                            .ok(),
                        {
                            if let Some(arg) = args.get("expires") {
                                natural_parser(&arg.to_string(), &timezone.to_string()).await
                            } else {
                                None
                            }
                        },
                    )
                } else {
                    let _ = invoke
                        .respond(&ctx, CreateGenericResponse::new()
                            .content("`repeat` is only available to Patreon subscribers or self-hosted users")
                        ).await;

                    return;
                }
            } else {
                (None, None)
            };

            if interval.is_none() && args.get("interval").is_some() {
                let _ = invoke
                    .respond(
                        &ctx,
                        CreateGenericResponse::new().content(
                            "Repeat interval could not be processed. Try and format the repetition similar to `1 hour` or `4 days`",
                        ),
                    )
                    .await;
            } else if expires.is_none() && args.get("expires").is_some() {
                let _ = invoke
                    .respond(
                        &ctx,
                        CreateGenericResponse::new().content(
                            "Expiry time failed to process. Please make it as clear as possible",
                        ),
                    )
                    .await;
            } else {
                let mut builder = MultiReminderBuilder::new(ctx, invoke.guild_id())
                    .author(user_data)
                    .content(content)
                    .time(time)
                    .timezone(timezone)
                    .expires(expires)
                    .interval(interval);

                builder.set_scopes(scopes);

                let (errors, successes) = builder.build().await;

                let embed = create_response(successes, errors, time);

                let _ = invoke
                    .respond(
                        &ctx,
                        CreateGenericResponse::new().embed(|c| {
                            *c = embed;
                            c
                        }),
                    )
                    .await;
            }
        }
        None => {
            let _ = invoke
                .respond(&ctx, CreateGenericResponse::new().content("Time could not be processed"))
                .await;
        }
    }
}

fn create_response(
    successes: HashSet<ReminderScope>,
    errors: HashSet<ReminderError>,
    time: i64,
) -> CreateEmbed {
    let success_part = match successes.len() {
        0 => "".to_string(),
        n => format!(
            "Reminder{s} for {locations} set for <t:{offset}:R>",
            s = if n > 1 { "s" } else { "" },
            locations = successes.iter().map(|l| l.mention()).collect::<Vec<String>>().join(", "),
            offset = time
        ),
    };

    let error_part = match errors.len() {
        0 => "".to_string(),
        n => format!(
            "{n} reminder{s} failed to set:\n{errors}",
            s = if n > 1 { "s" } else { "" },
            n = n,
            errors = errors.iter().map(|e| e.to_string()).collect::<Vec<String>>().join("\n")
        ),
    };

    let mut embed = CreateEmbed::default();
    embed
        .title(format!(
            "{n} Reminder{s} Set",
            n = successes.len(),
            s = if successes.len() > 1 { "s" } else { "" }
        ))
        .description(format!("{}\n\n{}", success_part, error_part))
        .color(*THEME_COLOR);

    embed
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
