use regex_command_attr::command;

use serenity::{client::Context, model::channel::Message};

use chrono::offset::Utc;

use crate::{
    consts::{DEFAULT_PREFIX, HELP_STRINGS},
    language_manager::LanguageManager,
    models::{GuildData, UserData},
    SQLPool, THEME_COLOR,
};

use levenshtein::levenshtein;

use inflector::Inflector;
use std::time::{SystemTime, UNIX_EPOCH};

#[command]
#[can_blacklist(false)]
async fn ping(ctx: &Context, msg: &Message, _args: String) {
    let now = SystemTime::now();
    let since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Time calculated as going backwards. Very bad");

    let delta = since_epoch.as_millis() as i64 - msg.timestamp.timestamp_millis();

    let _ = msg
        .channel_id
        .say(&ctx, format!("Time taken to receive message: {}ms", delta))
        .await;
}

#[command]
#[can_blacklist(false)]
async fn help(ctx: &Context, msg: &Message, args: String) {
    async fn default_help(
        ctx: &Context,
        msg: &Message,
        lm: &LanguageManager,
        prefix: &str,
        language: &str,
    ) {
        let desc = lm.get(language, "help/desc").replace("{prefix}", prefix);

        let _ = msg
            .channel_id
            .send_message(ctx, |m| {
                m.embed(move |e| {
                    e.title("Help Menu")
                        .description(desc)
                        .field(
                            lm.get(language, "help/setup_title"),
                            "`lang` `timezone`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/mod_title"),
                            "`prefix` `blacklist` `restrict` `alias`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/reminder_title"),
                            "`remind` `interval` `natural` `look`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/reminder_mod_title"),
                            "`del` `offset` `pause` `nudge`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/info_title"),
                            "`help` `info` `donate` `clock`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/todo_title"),
                            "`todo` `todos` `todoc`",
                            true,
                        )
                        .field(lm.get(language, "help/other_title"), "`timer`", true)
                        .footer(|f| {
                            f.text(concat!(
                                env!("CARGO_PKG_NAME"),
                                " ver ",
                                env!("CARGO_PKG_VERSION")
                            ))
                        })
                        .color(*THEME_COLOR)
                })
            })
            .await;
    }

    let data = ctx.data.read().await;

    let pool = data
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let lm = data.get::<LanguageManager>().unwrap();

    let language = UserData::language_of(&msg.author, &pool).await;
    let prefix = GuildData::prefix_from_id(msg.guild_id, &pool).await;

    if !args.is_empty() {
        let closest_match = HELP_STRINGS
            .iter()
            .map(|h| (levenshtein(h.split_at(5).1, &args), h))
            .filter(|(dist, _)| dist < &3)
            .min_by(|(dist_a, _), (dist_b, _)| dist_a.cmp(dist_b))
            .map(|(_, string)| string);

        if let Some(help_str) = closest_match {
            let desc = lm.get(&language, help_str);
            let command_name = help_str.split_at(5).1;

            let _ = msg
                .channel_id
                .send_message(ctx, |m| {
                    m.embed(move |e| {
                        e.title(format!("{} Help", command_name.to_title_case()))
                            .description(desc.replace("{prefix}", &prefix))
                            .footer(|f| {
                                f.text(concat!(
                                    env!("CARGO_PKG_NAME"),
                                    " ver ",
                                    env!("CARGO_PKG_VERSION")
                                ))
                            })
                            .color(*THEME_COLOR)
                    })
                })
                .await;
        } else {
            default_help(ctx, msg, lm, &prefix, &language).await;
        }
    } else {
        default_help(ctx, msg, lm, &prefix, &language).await;
    }
}

#[command]
async fn info(ctx: &Context, msg: &Message, _args: String) {
    let data = ctx.data.read().await;

    let pool = data
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let lm = data.get::<LanguageManager>().unwrap();

    let language = UserData::language_of(&msg.author, &pool).await;
    let guild_data = GuildData::from_guild(msg.guild(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();

    let desc = lm
        .get(&language, "info")
        .replacen("{user}", &ctx.cache.current_user().await.name, 1)
        .replace("{default_prefix}", &*DEFAULT_PREFIX)
        .replace("{prefix}", &guild_data.prefix);

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Info")
                    .description(desc)
                    .footer(|f| {
                        f.text(concat!(
                            env!("CARGO_PKG_NAME"),
                            " ver ",
                            env!("CARGO_PKG_VERSION")
                        ))
                    })
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

#[command]
async fn donate(ctx: &Context, msg: &Message, _args: String) {
    let data = ctx.data.read().await;

    let pool = data
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let lm = data.get::<LanguageManager>().unwrap();

    let language = UserData::language_of(&msg.author, &pool).await;
    let desc = lm.get(&language, "donate");

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Donate")
                    .description(desc)
                    .footer(|f| {
                        f.text(concat!(
                            env!("CARGO_PKG_NAME"),
                            " ver ",
                            env!("CARGO_PKG_VERSION")
                        ))
                    })
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

#[command]
async fn dashboard(ctx: &Context, msg: &Message, _args: String) {
    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Dashboard")
                    .description("https://reminder-bot.com/dashboard")
                    .footer(|f| {
                        f.text(concat!(
                            env!("CARGO_PKG_NAME"),
                            " ver ",
                            env!("CARGO_PKG_VERSION")
                        ))
                    })
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

#[command]
async fn clock(ctx: &Context, msg: &Message, args: String) {
    let data = ctx.data.read().await;

    let pool = data
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let lm = data.get::<LanguageManager>().unwrap();

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;

    let now = Utc::now().with_timezone(&timezone);

    let clock_display = lm.get(&language, "clock/time");

    if args == "12" {
        let _ = msg
            .channel_id
            .say(
                &ctx,
                clock_display.replacen("{}", &now.format("%I:%M:%S %p").to_string(), 1),
            )
            .await;
    } else {
        let _ = msg
            .channel_id
            .say(
                &ctx,
                clock_display.replacen("{}", &now.format("%H:%M:%S").to_string(), 1),
            )
            .await;
    }
}
