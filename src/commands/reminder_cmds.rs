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
    models::{
        ChannelData,
        GuildData,
        UserData,
    },
    SQLPool,
    time_parser::TimeParser,
};

use chrono::NaiveDateTime;


#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn pause(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();
    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool).await.unwrap();

    if args.len() == 0 {
        channel.paused = !channel.paused;
        channel.paused_until = None;

        channel.commit_changes(&pool).await;

        if channel.paused {
            let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "pause/paused_indefinite").await).await;
        }
        else {
            let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "pause/unpaused").await).await;
        }
    }
    else {
        let parser = TimeParser::new(args, user_data.timezone.parse().unwrap());
        let pause_until = parser.timestamp();

        match pause_until {
            Ok(timestamp) => {
                channel.paused = true;
                channel.paused_until = Some(NaiveDateTime::from_timestamp(timestamp, 0));

                channel.commit_changes(&pool).await;

                let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "pause/paused_until").await).await;
            },

            Err(_) => {
                let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "pause/invalid_time").await).await;
            },
        }
    }

    Ok(())
}

#[command]
#[permission_level(Restricted)]
async fn offset(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();

    if args.len() == 0 {
        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "offset/help").await).await;
    }
    else {
        let parser = TimeParser::new(args, user_data.timezone());

        if let Ok(displacement) = parser.displacement() {
            if let Some(guild) = msg.guild(&ctx).await {
                let guild_data = GuildData::from_guild(guild, &pool).await.unwrap();

                sqlx::query!(
                    "
UPDATE reminders
    INNER JOIN `channels`
        ON `channels`.id = reminders.channel_id
    SET
        reminders.`time` = reminders.`time` + ?
    WHERE channels.guild_id = ?
                    ", displacement, guild_data.id)
                    .execute(&pool)
                    .await
                    .unwrap();
            } else {
                sqlx::query!(
                    "
UPDATE reminders SET `time` = `time` + ? WHERE reminders.channel_id = ?
                    ", displacement, user_data.dm_channel)
                    .execute(&pool)
                    .await
                    .unwrap();
            }

            let response = user_data.response(&pool, "offset/success").await.replacen("{}", &displacement.to_string(), 1);

            let _ = msg.channel_id.say(&ctx, response).await;
        } else {
            let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "offset/invalid_time").await).await;
        }
    }

    Ok(())
}

#[command]
#[permission_level(Restricted)]
async fn nudge(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    let user_data = UserData::from_id(&msg.author, &ctx, &pool).await.unwrap();
    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool).await.unwrap();

    if args.len() == 0 {
        let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "nudge/invalid_time").await).await;
    }
    else {
        let parser = TimeParser::new(args, user_data.timezone.parse().unwrap());
        let nudge_time = parser.displacement();

        match nudge_time {
            Ok(displacement) => {
                if displacement < i16::MIN as i64 || displacement > i16::MAX as i64 {
                    let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "nudge/invalid_time").await).await;
                }
                else {
                    channel.nudge = displacement as i16;

                    channel.commit_changes(&pool).await;

                    let response = user_data.response(&pool, "nudge/success").await.replacen("{}", &displacement.to_string(), 1);

                    let _ = msg.channel_id.say(&ctx, response).await;
                }
            },

            Err(_) => {
                let _ = msg.channel_id.say(&ctx, user_data.response(&pool, "nudge/invalid_time").await).await;
            },
        }
    }

    Ok(())
}
