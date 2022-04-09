use std::{collections::HashMap, env, sync::atomic::Ordering};

use log::{error, info, warn};
use poise::{
    async_trait,
    serenity::{model::interactions::Interaction, utils::shard_id},
    serenity_prelude as serenity,
};

use crate::{component_models::ComponentDataModel, Data, Error, GuildId};

struct Handler;

#[async_trait]
impl serenity::EventHandler for Handler {
    async fn guild_create(&self, ctx: serenity::Context, guild: serenity::Guild, is_new: bool) {
        if is_new {
            let guild_id = guild.id.as_u64().to_owned();

            // todo
            // sqlx::query!("INSERT INTO guilds (guild) VALUES (?)", guild_id)
            //    .execute(&data.database)
            //    .await
            //    .unwrap();

            //if let Ok(token) = env::var("DISCORDBOTS_TOKEN") {
            //    let shard_count = ctx.cache.shard_count();
            //    let current_shard_id = shard_id(guild_id, shard_count);

            //    let guild_count = ctx
            //        .cache
            //        .guilds()
            //        .iter()
            //        .filter(|g| {
            //            shard_id(g.as_u64().to_owned(), shard_count) == current_shard_id
            //        })
            //        .count() as u64;

            //    let mut hm = HashMap::new();
            //    hm.insert("server_count", guild_count);
            //    hm.insert("shard_id", current_shard_id);
            //    hm.insert("shard_count", shard_count);

            //    let response = data
            //        .http
            //        .post(
            //            format!(
            //                "https://top.gg/api/bots/{}/stats",
            //                ctx.cache.current_user_id().as_u64()
            //            )
            //            .as_str(),
            //        )
            //        .header("Authorization", token)
            //        .json(&hm)
            //        .send()
            //        .await;

            //    if let Err(res) = response {
            //        println!("DiscordBots Response: {:?}", res);
            //    }
            //}
        }
    }
}

pub async fn listener(
    ctx: &serenity::Context,
    event: &serenity::Event,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::Event::Ready(_) => {
            info!("Cache Ready! Preparing extra processes");

            if data
                .is_loop_running
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let kill_tx = data.broadcast.clone();
                let kill_recv = data.broadcast.subscribe();

                let ctx1 = ctx.clone();
                let ctx2 = ctx.clone();

                let pool1 = data.database.clone();
                let pool2 = data.database.clone();

                let run_settings = env::var("DONTRUN").unwrap_or_else(|_| "".to_string());

                if !run_settings.contains("postman") {
                    tokio::spawn(async move {
                        match postman::initialize(kill_recv, ctx1, &pool1).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("postman exiting: {}", e);
                            }
                        };
                    });
                } else {
                    warn!("Not running postman")
                }

                if !run_settings.contains("web") {
                    tokio::spawn(async move {
                        reminder_web::initialize(kill_tx, ctx2, pool2).await.unwrap();
                    });
                } else {
                    warn!("Not running web")
                }
            }
        }
        serenity::Event::ChannelDelete(event) => {
            sqlx::query!("DELETE FROM channels WHERE channel = ?", event.channel.id().0)
                .execute(&data.database)
                .await
                .unwrap();
        }
        serenity::Event::GuildDelete(event) => {
            let _ = sqlx::query!("DELETE FROM guilds WHERE guild = ?", event.guild.id.0)
                .execute(&data.database)
                .await;
        }
        serenity::Event::InteractionCreate(event) => match &event.interaction {
            Interaction::MessageComponent(component) => {
                let component_model = ComponentDataModel::from_custom_id(&component.data.custom_id);

                component_model.act(ctx, data, &component).await;
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}
