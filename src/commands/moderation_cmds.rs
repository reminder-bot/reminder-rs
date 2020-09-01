use regex_command_attr::command;

use serenity::{
    client::Context,
    model::{
        channel::{
            Message,
        },
    },
    framework::standard::CommandResult,
};

use regex::Regex;

use chrono_tz::Tz;

use crate::{
    models::{
        ChannelData,
        UserData,
        GuildData,
    },
    SQLPool,
};

lazy_static! {
    static ref REGEX_CHANNEL: Regex = Regex::new(r#"^\s*<#(\d+)>\s*$"#).unwrap();
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
#[can_blacklist(false)]
async fn blacklist(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let capture_opt = REGEX_CHANNEL.captures(&args).map(|cap| cap.get(1)).flatten();

    let mut channel = match capture_opt {
        Some(capture) =>
            ChannelData::from_id(capture.as_str().parse::<u64>().unwrap(), &pool).await.unwrap(),

        None =>
            ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool).await.unwrap(),
    };

    channel.blacklisted = !channel.blacklisted;
    channel.commit_changes(&pool).await;

    if channel.blacklisted {
        let _ = msg.channel_id.say(&ctx, "Blacklisted").await;
    }
    else {
        let _ = msg.channel_id.say(&ctx, "Unblacklisted").await;
    }

    Ok(())
}

#[command]
async fn timezone(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let mut user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();

    if args.len() > 0 {
        match args.parse::<Tz>() {
            Ok(_) => {
                user_data.timezone = args;

                user_data.commit_changes(&pool).await;

                let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "timezone/set_p").await).await;
            }

            Err(_) => {
                let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "timezone/no_timezone").await).await;
            }
        }
    }
    else {
        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "timezone/no_argument").await).await;
    }

    Ok(())
}

#[command]
async fn language(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let mut user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();

    match sqlx::query!(
        "
SELECT code FROM languages WHERE code = ? OR name = ?
        ", args, args)
        .fetch_one(&pool)
        .await {

        Ok(row) => {
            user_data.language = row.code;

            user_data.commit_changes(&pool).await;

            let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "lang/set_p").await).await;
        },

        Err(_) => {
            let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "lang/invalid").await).await;
        },
    }

    Ok(())
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn prefix(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let mut guild_data = GuildData::from_guild(msg.guild(&ctx).await.unwrap(), &pool).await.unwrap();
    let user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();

    if args.len() > 5 {
        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "prefix/too_long").await).await;
    }
    else if args.len() == 0 {
        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "prefix/no_argument").await).await;
    }
    else {
        guild_data.prefix = args;
        guild_data.commit_changes(&pool).await;

        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "prefix/success").await).await;
    }

    Ok(())
}
