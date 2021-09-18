use chrono::offset::Utc;
use regex_command_attr::command;
use serenity::{builder::CreateEmbedFooter, client::Context};

use crate::{
    consts::DEFAULT_PREFIX,
    framework::{CommandInvoke, CreateGenericResponse},
    models::CtxData,
    THEME_COLOR,
};

fn footer(ctx: &Context) -> impl FnOnce(&mut CreateEmbedFooter) -> &mut CreateEmbedFooter {
    let shard_count = ctx.cache.shard_count();
    let shard = ctx.shard_id;

    move |f| {
        f.text(format!(
            "{}\nshard {} of {}",
            concat!(env!("CARGO_PKG_NAME"), " ver ", env!("CARGO_PKG_VERSION")),
            shard,
            shard_count,
        ))
    }
}

#[command]
#[aliases("invite")]
#[description("Get information about the bot")]
#[group("Info")]
async fn info(ctx: &Context, invoke: &(dyn CommandInvoke + Send + Sync)) {
    let prefix = ctx.prefix(invoke.guild_id()).await;
    let current_user = ctx.cache.current_user();
    let footer = footer(ctx);

    let _ = invoke
        .respond(
            ctx.http.clone(),
            CreateGenericResponse::new().embed(|e| {
                e.title("Info")
                    .description(format!(
                        "Default prefix: `{default_prefix}`
Reset prefix: `@{user} prefix {default_prefix}`
Help: `{prefix}help`**Welcome to Reminder Bot!**
Developer: <@203532103185465344>
Icon: <@253202252821430272>
Find me on https://discord.jellywx.com and on https://github.com/JellyWX :)

Invite the bot: https://invite.reminder-bot.com/
Use our dashboard: https://reminder-bot.com/",
                        default_prefix = *DEFAULT_PREFIX,
                        user = current_user.name,
                        prefix = prefix
                    ))
                    .footer(footer)
                    .color(*THEME_COLOR)
            }),
        )
        .await;
}

#[command]
#[description("Details on supporting the bot and Patreon benefits")]
#[group("Info")]
async fn donate(ctx: &Context, invoke: &(dyn CommandInvoke + Send + Sync)) {
    let footer = footer(ctx);

    let _ = invoke
        .respond(
            ctx.http.clone(),
            CreateGenericResponse::new().embed(|e| {
                e.title("Donate")
                    .description("Thinking of adding a monthly contribution? Click below for my Patreon and official bot server :)

**https://www.patreon.com/jellywx/**
**https://discord.jellywx.com/**

When you subscribe, Patreon will automatically rank you up on our Discord server (make sure you link your Patreon and Discord accounts!)
With your new rank, you'll be able to:
• Set repeating reminders with `interval`, `natural` or the dashboard
• Use unlimited uploads on SoundFX

(Also, members of servers you __own__ will be able to set repeating reminders via commands)

Just $2 USD/month!

*Please note, you must be in the JellyWX Discord server to receive Patreon features*")
                    .footer(footer)
                    .color(*THEME_COLOR)
            }),
        )
        .await;
}

#[command]
#[description("Get the link to the online dashboard")]
#[group("Info")]
async fn dashboard(ctx: &Context, invoke: &(dyn CommandInvoke + Send + Sync)) {
    let footer = footer(ctx);

    let _ = invoke
        .respond(
            ctx.http.clone(),
            CreateGenericResponse::new().embed(|e| {
                e.title("Dashboard")
                    .description("**https://reminder-bot.com/dashboard**")
                    .footer(footer)
                    .color(*THEME_COLOR)
            }),
        )
        .await;
}

#[command]
#[description("View the current time in your selected timezone")]
#[group("Info")]
async fn clock(ctx: &Context, invoke: &(dyn CommandInvoke + Send + Sync)) {
    let ud = ctx.user_data(&invoke.author_id()).await.unwrap();
    let now = Utc::now().with_timezone(&ud.timezone());

    let _ = invoke
        .respond(
            ctx.http.clone(),
            CreateGenericResponse::new().content(format!("Current time: {}", now.format("%H:%M"))),
        )
        .await;
}
