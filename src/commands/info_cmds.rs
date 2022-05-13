use chrono::offset::Utc;
use poise::{serenity_prelude as serenity, serenity_prelude::Mentionable};

use crate::{models::CtxData, Context, Error, THEME_COLOR};

fn footer(
    ctx: Context<'_>,
) -> impl FnOnce(&mut serenity::CreateEmbedFooter) -> &mut serenity::CreateEmbedFooter {
    let shard_count = ctx.discord().cache.shard_count();
    let shard = ctx.discord().shard_id;

    move |f| {
        f.text(format!(
            "{}\nshard {} of {}",
            concat!(env!("CARGO_PKG_NAME"), " ver ", env!("CARGO_PKG_VERSION")),
            shard,
            shard_count,
        ))
    }
}

/// Get an overview of bot commands
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let footer = footer(ctx);

    ctx.send(|m| {
        m.ephemeral(true).embed(|e| {
            e.title("Help")
                .color(*THEME_COLOR)
                .description(
                    "__Info Commands__
`/help` `/info` `/donate` `/dashboard` `/clock`
*run these commands with no options*

__Reminder Commands__
`/remind` - Create a new reminder that will send a message at a certain time
`/timer` - Start a timer from now, that will count time passed. Also used to view and remove timers

__Reminder Management__
`/del` - Delete reminders
`/look` - View reminders
`/pause` - Pause all reminders on the channel
`/offset` - Move all reminders by a certain time
`/nudge` - Move all new reminders on this channel by a certain time

__Todo Commands__
`/todo` - Add, view and manage the server, channel or user todo lists

__Setup Commands__
`/timezone` - Set your timezone (necessary for `/remind` to work properly)

__Advanced Commands__
`/macro` - Record and replay command sequences
                    ",
                )
                .footer(footer)
        })
    })
    .await?;

    Ok(())
}

/// Get information about the bot
#[poise::command(slash_command)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let footer = footer(ctx);

    let _ = ctx
        .send(|m| {
            m.ephemeral(true).embed(|e| {
                e.title("Info")
                    .description(
                        "Help: `/help`

**Welcome to Reminder Bot!**
Developer: <@203532103185465344>
Icon: <@253202252821430272>
Find me on https://discord.jellywx.com and on https://github.com/JellyWX :)

Invite the bot: https://invite.reminder-bot.com/
Use our dashboard: https://reminder-bot.com/",
                    )
                    .footer(footer)
                    .color(*THEME_COLOR)
            })
        })
        .await;

    Ok(())
}

/// Details on supporting the bot and Patreon benefits
#[poise::command(slash_command)]
pub async fn donate(ctx: Context<'_>) -> Result<(), Error> {
    let footer = footer(ctx);

    ctx.send(|m| m.embed(|e| {
        e.title("Donate")
            .description("Thinking of adding a monthly contribution?
Click below for my Patreon and official bot server :)

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
    .await?;

    Ok(())
}

/// Get the link to the online dashboard
#[poise::command(slash_command)]
pub async fn dashboard(ctx: Context<'_>) -> Result<(), Error> {
    let footer = footer(ctx);

    ctx.send(|m| {
        m.ephemeral(true).embed(|e| {
            e.title("Dashboard")
                .description("**https://reminder-bot.com/dashboard**")
                .footer(footer)
                .color(*THEME_COLOR)
        })
    })
    .await?;

    Ok(())
}

/// View the current time in your selected timezone
#[poise::command(slash_command)]
pub async fn clock(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let tz = ctx.timezone().await;
    let now = Utc::now().with_timezone(&tz);

    ctx.send(|m| {
        m.ephemeral(true).content(format!("Time in **{}**: `{}`", tz, now.format("%H:%M")))
    })
    .await?;

    Ok(())
}

/// View the current time in a user's selected timezone
#[poise::command(context_menu_command = "View Local Time")]
pub async fn clock_context_menu(ctx: Context<'_>, user: serenity::User) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let user_data = ctx.user_data(user.id).await?;
    let tz = user_data.timezone();

    let now = Utc::now().with_timezone(&tz);

    ctx.send(|m| {
        m.ephemeral(true).content(format!(
            "Time in {}'s timezone: `{}`",
            user.mention(),
            now.format("%H:%M")
        ))
    })
    .await?;

    Ok(())
}
