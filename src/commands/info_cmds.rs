use regex_command_attr::command;

use serenity::{client::Context, model::channel::Message};

use chrono::offset::Utc;

use crate::{
    command_help,
    consts::{DEFAULT_PREFIX, HELP_STRINGS},
    get_ctx_data,
    language_manager::LanguageManager,
    models::{GuildData, UserData},
    THEME_COLOR,
};

use std::sync::Arc;
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
        lm: Arc<LanguageManager>,
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
                            "`lang` `timezone` `meridian`",
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

    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool);
    let prefix = GuildData::prefix_from_id(msg.guild_id, &pool);

    if !args.is_empty() {
        let matched = HELP_STRINGS
            .iter()
            .filter(|h| h.split_at(5).1 == args)
            .next();

        if let Some(help_str) = matched {
            let command_name = help_str.split_at(5).1;

            command_help(ctx, msg, lm, &prefix.await, &language.await, command_name).await
        } else {
            default_help(ctx, msg, lm, &prefix.await, &language.await).await;
        }
    } else {
        default_help(ctx, msg, lm, &prefix.await, &language.await).await;
    }
}

#[command]
async fn info(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool);
    let prefix = GuildData::prefix_from_id(msg.guild_id, &pool);
    let current_user = ctx.cache.current_user();

    let desc = lm
        .get(&language.await, "info")
        .replacen("{user}", &current_user.await.name, 1)
        .replace("{default_prefix}", &*DEFAULT_PREFIX)
        .replace("{prefix}", &prefix.await);

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
    let (pool, lm) = get_ctx_data(&ctx).await;

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
async fn clock(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;
    let meridian = UserData::meridian_of(&msg.author, &pool).await;

    let now = Utc::now().with_timezone(&timezone);

    let clock_display = lm.get(&language, "clock/time");

    let _ = msg
        .channel_id
        .say(
            &ctx,
            clock_display.replacen("{}", &now.format(meridian.fmt_str()).to_string(), 1),
        )
        .await;
}
