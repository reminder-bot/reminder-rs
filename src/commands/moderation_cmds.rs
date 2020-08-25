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

use crate::{
    models::ChannelData,
    SQLPool,
};

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn blacklist(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let mut channel = ChannelData::from_id(msg.channel(&ctx).await.unwrap(), pool.clone()).await.unwrap();

    channel.blacklisted = !channel.blacklisted;
    channel.commit_changes(pool).await;

    if channel.blacklisted {

    }
    else {

    }

    Ok(())
}
