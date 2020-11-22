use regex_command_attr::command;

use serenity::{client::Context, model::channel::Message};

use chrono::offset::Utc;

use crate::{
    consts::DEFAULT_PREFIX,
    models::{GuildData, UserData},
    SQLPool, THEME_COLOR,
};

use crate::language_manager::LanguageManager;
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
async fn help(ctx: &Context, msg: &Message, _args: String) {
    let data = ctx.data.read().await;

    let pool = data
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let lm = data.get::<LanguageManager>().unwrap();

    let language = UserData::language_of(&msg.author, &pool).await;

    let desc = lm.get(&language, "help");

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Help")
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
    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    let now = Utc::now().with_timezone(&user_data.timezone());

    let clock_display = lm.get(&user_data.language, "clock/time");

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
