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

use crate::{
    models::ChannelData,
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
            ChannelData::from_id(capture.as_str().parse::<u64>().unwrap(), pool.clone()).await.unwrap(),

        None =>
            ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), pool.clone()).await.unwrap(),
    };

    channel.blacklisted = !channel.blacklisted;
    channel.commit_changes(pool).await;

    if channel.blacklisted {
        let _ = msg.channel_id.say(&ctx, "Blacklisted").await;
    }
    else {
        let _ = msg.channel_id.say(&ctx, "Unblacklisted").await;
    }

    Ok(())
}
