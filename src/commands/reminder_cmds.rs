use std::{
    collections::HashSet,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use chrono_tz::Tz;
use num_integer::Integer;
use poise::{
    serenity_prelude::{
        builder::CreateEmbed, component::ButtonStyle, model::channel::Channel, ReactionType,
    },
    AutocompleteChoice, CreateReply, Modal,
};

use crate::{
    component_models::{
        pager::{DelPager, LookPager, Pager},
        ComponentDataModel, DelSelector, UndoReminder,
    },
    consts::{
        EMBED_DESCRIPTION_MAX_LENGTH, HOUR, MINUTE, REGEX_CHANNEL_USER, SELECT_MAX_ENTRIES,
        THEME_COLOR,
    },
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
        CtxData,
    },
    time_parser::natural_parser,
    utils::{check_guild_subscription, check_subscription},
    ApplicationContext, Context, Error,
};

/// Pause all reminders on the current channel until a certain time or indefinitely
#[poise::command(
    slash_command,
    identifying_name = "pause",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn pause(
    ctx: Context<'_>,
    #[description = "When to pause until"] until: Option<String>,
) -> Result<(), Error> {
    let timezone = ctx.timezone().await;

    let mut channel = ctx.channel_data().await.unwrap();

    match until {
        Some(until) => {
            let parsed = natural_parser(&until, &timezone.to_string()).await;

            if let Some(timestamp) = parsed {
                let dt = NaiveDateTime::from_timestamp(timestamp, 0);

                channel.paused = true;
                channel.paused_until = Some(dt);

                channel.commit_changes(&ctx.data().database).await;

                ctx.say(format!(
                    "Reminders in this channel have been silenced until **<t:{}:D>**",
                    timestamp
                ))
                .await?;
            } else {
                ctx.say(
                    "Time could not be processed. Please write the time as clearly as possible",
                )
                .await?;
            }
        }
        _ => {
            channel.paused = !channel.paused;
            channel.paused_until = None;

            channel.commit_changes(&ctx.data().database).await;

            if channel.paused {
                ctx.say("Reminders in this channel have been silenced indefinitely").await?;
            } else {
                ctx.say("Reminders in this channel have been unsilenced").await?;
            }
        }
    }

    Ok(())
}

/// Move all reminders in the current server by a certain amount of time. Times get added together
#[poise::command(
    slash_command,
    identifying_name = "offset",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn offset(
    ctx: Context<'_>,
    #[description = "Number of hours to offset by"] hours: Option<isize>,
    #[description = "Number of minutes to offset by"] minutes: Option<isize>,
    #[description = "Number of seconds to offset by"] seconds: Option<isize>,
) -> Result<(), Error> {
    let combined_time = hours.map_or(0, |h| h * HOUR as isize)
        + minutes.map_or(0, |m| m * MINUTE as isize)
        + seconds.map_or(0, |s| s);

    if combined_time == 0 {
        ctx.say("Please specify one of `hours`, `minutes` or `seconds`").await?;
    } else {
        if let Some(guild) = ctx.guild() {
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
                combined_time as i64,
                channels
            )
            .execute(&ctx.data().database)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "UPDATE reminders INNER JOIN `channels` ON `channels`.id = reminders.channel_id SET reminders.`utc_time` = reminders.`utc_time` + ? WHERE channels.`channel` = ?",
                combined_time as i64,
                ctx.channel_id().0
            )
            .execute(&ctx.data().database)
            .await
            .unwrap();
        }

        ctx.say(format!("All reminders offset by {} seconds", combined_time)).await?;
    }

    Ok(())
}

/// Nudge all future reminders on this channel by a certain amount (don't use for DST! See `/offset`)
#[poise::command(
    slash_command,
    identifying_name = "nudge",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn nudge(
    ctx: Context<'_>,
    #[description = "Number of minutes to nudge new reminders by"] minutes: Option<isize>,
    #[description = "Number of seconds to nudge new reminders by"] seconds: Option<isize>,
) -> Result<(), Error> {
    let combined_time = minutes.map_or(0, |m| m * MINUTE as isize) + seconds.map_or(0, |s| s);

    if combined_time < i16::MIN as isize || combined_time > i16::MAX as isize {
        ctx.say("Nudge times must be less than 500 minutes").await?;
    } else {
        let mut channel_data = ctx.channel_data().await.unwrap();

        channel_data.nudge = combined_time as i16;
        channel_data.commit_changes(&ctx.data().database).await;

        ctx.say(format!("Future reminders will be nudged by {} seconds", combined_time)).await?;
    }

    Ok(())
}

/// View reminders on a specific channel
#[poise::command(
    slash_command,
    identifying_name = "look",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn look(
    ctx: Context<'_>,
    #[description = "Channel to view reminders on"] channel: Option<Channel>,
    #[description = "Whether to show disabled reminders or not"] disabled: Option<bool>,
    #[description = "Whether to display times as relative or exact times"] relative: Option<bool>,
) -> Result<(), Error> {
    let timezone = ctx.timezone().await;

    let flags = LookFlags {
        show_disabled: disabled.unwrap_or(true),
        channel_id: channel.map(|c| c.id()),
        time_display: relative.map_or(TimeDisplayType::Relative, |b| {
            if b {
                TimeDisplayType::Relative
            } else {
                TimeDisplayType::Absolute
            }
        }),
    };

    let channel_opt = ctx.channel_id().to_channel_cached(&ctx.discord());

    let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
        if Some(channel.guild_id) == ctx.guild_id() {
            flags.channel_id.unwrap_or_else(|| ctx.channel_id())
        } else {
            ctx.channel_id()
        }
    } else {
        ctx.channel_id()
    };

    let channel_name =
        if let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx.discord()) {
            Some(channel.name)
        } else {
            None
        };

    let reminders = Reminder::from_channel(&ctx.data().database, channel_id, &flags).await;

    if reminders.is_empty() {
        let _ = ctx.say("No reminders on specified channel").await;
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

        ctx.send(|r| {
            r.ephemeral(true)
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
                })
        })
        .await?;
    }

    Ok(())
}

/// Delete reminders
#[poise::command(
    slash_command,
    rename = "del",
    identifying_name = "delete",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    let timezone = ctx.timezone().await;

    let reminders =
        Reminder::from_guild(&ctx.discord(), &ctx.data().database, ctx.guild_id(), ctx.author().id)
            .await;

    let resp = show_delete_page(&reminders, 0, timezone);

    ctx.send(|r| {
        *r = resp;
        r
    })
    .await?;

    Ok(())
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

pub fn show_delete_page(reminders: &[Reminder], page: usize, timezone: Tz) -> CreateReply {
    let pager = DelPager::new(page, timezone);

    if reminders.is_empty() {
        let mut reply = CreateReply::default();

        reply
            .embed(|e| e.title("Delete Reminders").description("No Reminders").color(*THEME_COLOR))
            .components(|comp| {
                pager.create_button_row(0, comp);
                comp
            });

        return reply;
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

    let mut reply = CreateReply::default();

    reply
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
        });

    reply
}

fn time_difference(start_time: NaiveDateTime) -> String {
    let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let now = NaiveDateTime::from_timestamp(unix_time, 0);

    let delta = (now - start_time).num_seconds();

    let (minutes, seconds) = delta.div_rem(&60);
    let (hours, minutes) = minutes.div_rem(&60);
    let (days, hours) = hours.div_rem(&24);

    format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
}

/// Manage timers
#[poise::command(
    slash_command,
    rename = "timer",
    identifying_name = "timer_base",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn timer_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// List the timers in this server or DM channel
#[poise::command(
    slash_command,
    rename = "list",
    identifying_name = "list_timer",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn list_timer(ctx: Context<'_>) -> Result<(), Error> {
    let owner = ctx.guild_id().map(|g| g.0).unwrap_or_else(|| ctx.author().id.0);

    let timers = Timer::from_owner(owner, &ctx.data().database).await;

    if !timers.is_empty() {
        ctx.send(|m| {
            m.embed(|e| {
                e.fields(timers.iter().map(|timer| {
                    (&timer.name, format!("⌚ `{}`", time_difference(timer.start_time)), false)
                }))
                .color(*THEME_COLOR)
            })
        })
        .await?;
    } else {
        ctx.say("No timers currently. Use `/timer start` to create a new timer").await?;
    }

    Ok(())
}

/// Start a new timer from now
#[poise::command(
    slash_command,
    rename = "start",
    identifying_name = "start_timer",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn start_timer(
    ctx: Context<'_>,
    #[description = "Name for the new timer"] name: String,
) -> Result<(), Error> {
    let owner = ctx.guild_id().map(|g| g.0).unwrap_or_else(|| ctx.author().id.0);

    let count = Timer::count_from_owner(owner, &ctx.data().database).await;

    if count >= 25 {
        ctx.say("You already have 25 timers. Please delete some timers before creating a new one")
            .await?;
    } else if name.len() <= 32 {
        Timer::create(&name, owner, &ctx.data().database).await;

        ctx.say("Created a new timer").await?;
    } else {
        ctx.say(format!(
            "Please name your timer something shorted (max. 32 characters, you used {})",
            name.len()
        ))
        .await?;
    }

    Ok(())
}

/// Delete a timer
#[poise::command(
    slash_command,
    rename = "delete",
    identifying_name = "delete_timer",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn delete_timer(
    ctx: Context<'_>,
    #[description = "Name of timer to delete"] name: String,
) -> Result<(), Error> {
    let owner = ctx.guild_id().map(|g| g.0).unwrap_or_else(|| ctx.author().id.0);

    let exists =
        sqlx::query!("SELECT 1 as _r FROM timers WHERE owner = ? AND name = ?", owner, name)
            .fetch_one(&ctx.data().database)
            .await;

    if exists.is_ok() {
        sqlx::query!("DELETE FROM timers WHERE owner = ? AND name = ?", owner, name)
            .execute(&ctx.data().database)
            .await
            .unwrap();

        ctx.say("Deleted a timer").await?;
    } else {
        ctx.say("Could not find a timer by that name").await?;
    }

    Ok(())
}

async fn multiline_autocomplete(
    _ctx: Context<'_>,
    partial: &str,
) -> Vec<AutocompleteChoice<String>> {
    if partial.is_empty() {
        vec![AutocompleteChoice { name: "Multiline content...".to_string(), value: "".to_string() }]
    } else {
        vec![
            AutocompleteChoice { name: partial.to_string(), value: partial.to_string() },
            AutocompleteChoice { name: "Multiline content...".to_string(), value: "".to_string() },
        ]
    }
}

#[derive(poise::Modal)]
#[name = "Reminder"]
struct ContentModal {
    #[name = "Content"]
    #[placeholder = "Message..."]
    #[paragraph]
    #[max_length = 2000]
    content: String,
}

/// Create a reminder. Press "+5 more" for other options. A modal will open if "content" is not provided
#[poise::command(
    slash_command,
    identifying_name = "remind",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn remind(
    ctx: ApplicationContext<'_>,
    #[description = "A description of the time to set the reminder for"] time: String,
    #[description = "The message content to send"]
    #[autocomplete = "multiline_autocomplete"]
    content: String,
    #[description = "Channel or user mentions to set the reminder for"] channels: Option<String>,
    #[description = "(Patreon only) Time to wait before repeating the reminder. Leave blank for one-shot reminder"]
    interval: Option<String>,
    #[description = "(Patreon only) For repeating reminders, the time at which the reminder will stop repeating"]
    expires: Option<String>,
    #[description = "Set the TTS flag on the reminder message, similar to the /tts command"]
    tts: Option<bool>,
) -> Result<(), Error> {
    if content.is_empty() {
        let data = ContentModal::execute(ctx).await?;

        create_reminder(
            Context::Application(ctx),
            time,
            data.content,
            channels,
            interval,
            expires,
            tts,
        )
        .await
    } else {
        create_reminder(Context::Application(ctx), time, content, channels, interval, expires, tts)
            .await
    }
}

async fn create_reminder(
    ctx: Context<'_>,
    time: String,
    content: String,
    channels: Option<String>,
    interval: Option<String>,
    expires: Option<String>,
    tts: Option<bool>,
) -> Result<(), Error> {
    if interval.is_none() && expires.is_some() {
        ctx.say("`expires` can only be used with `interval`").await?;

        return Ok(());
    }

    ctx.defer().await?;

    let user_data = ctx.author_data().await.unwrap();
    let timezone = ctx.timezone().await;

    let time = natural_parser(&time, &timezone.to_string()).await;

    match time {
        Some(time) => {
            let content = {
                let tts = tts.unwrap_or(false);

                Content { content, tts, attachment: None, attachment_name: None }
            };

            let scopes = {
                let list = channels.map(|arg| parse_mention_list(&arg)).unwrap_or_default();

                if list.is_empty() {
                    if ctx.guild_id().is_some() {
                        vec![ReminderScope::Channel(ctx.channel_id().0)]
                    } else {
                        vec![ReminderScope::User(ctx.author().id.0)]
                    }
                } else {
                    list
                }
            };

            let (processed_interval, processed_expires) = if let Some(repeat) = &interval {
                if check_subscription(&ctx.discord(), ctx.author().id).await
                    || (ctx.guild_id().is_some()
                        && check_guild_subscription(&ctx.discord(), ctx.guild_id().unwrap()).await)
                {
                    (
                        parse_duration(repeat)
                            .or_else(|_| parse_duration(&format!("1 {}", repeat)))
                            .ok(),
                        {
                            if let Some(arg) = &expires {
                                natural_parser(arg, &timezone.to_string()).await
                            } else {
                                None
                            }
                        },
                    )
                } else {
                    ctx.say(
                        "`repeat` is only available to Patreon subscribers or self-hosted users",
                    )
                    .await?;

                    return Ok(());
                }
            } else {
                (None, None)
            };

            if processed_interval.is_none() && interval.is_some() {
                ctx.say(
                    "Repeat interval could not be processed. Try similar to `1 hour` or `4 days`",
                )
                .await?;
            } else if processed_expires.is_none() && expires.is_some() {
                ctx.say("Expiry time failed to process. Please make it as clear as possible")
                    .await?;
            } else {
                let mut builder = MultiReminderBuilder::new(&ctx, ctx.guild_id())
                    .author(user_data)
                    .content(content)
                    .time(time)
                    .timezone(timezone)
                    .expires(processed_expires)
                    .interval(processed_interval);

                builder.set_scopes(scopes);

                let (errors, successes) = builder.build().await;

                let embed = create_response(&successes, &errors, time);

                if successes.len() == 1 {
                    let reminder = successes.iter().next().map(|(r, _)| r.id).unwrap();
                    let undo_button = ComponentDataModel::UndoReminder(UndoReminder {
                        user_id: ctx.author().id,
                        reminder_id: reminder,
                    });

                    ctx.send(|m| {
                        m.embed(|c| {
                            *c = embed;
                            c
                        })
                        .components(|c| {
                            c.create_action_row(|r| {
                                r.create_button(|b| {
                                    b.emoji(ReactionType::Unicode("🔕".to_string()))
                                        .label("Cancel")
                                        .style(ButtonStyle::Danger)
                                        .custom_id(undo_button.to_custom_id())
                                })
                                .create_button(|b| {
                                    b.emoji(ReactionType::Unicode("📝".to_string()))
                                        .label("Edit")
                                        .style(ButtonStyle::Link)
                                        .url("https://reminder-bot.com/dashboard")
                                })
                            })
                        })
                    })
                    .await?;
                } else {
                    ctx.send(|m| {
                        m.embed(|c| {
                            *c = embed;
                            c
                        })
                    })
                    .await?;
                }
            }
        }

        None => {
            ctx.say("Time could not be processed").await?;
        }
    }

    Ok(())
}

fn create_response(
    successes: &HashSet<(Reminder, ReminderScope)>,
    errors: &HashSet<ReminderError>,
    time: i64,
) -> CreateEmbed {
    let success_part = match successes.len() {
        0 => "".to_string(),
        n => format!(
            "Reminder{s} for {locations} set for <t:{offset}:R>",
            s = if n > 1 { "s" } else { "" },
            locations =
                successes.iter().map(|(_, l)| l.mention()).collect::<Vec<String>>().join(", "),
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
