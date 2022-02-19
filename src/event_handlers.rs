use std::{
    collections::HashMap,
    env,
    sync::atomic::{AtomicBool, Ordering},
};

use log::{info, warn};
use poise::{
    serenity::{model::interactions::Interaction, utils::shard_id},
    serenity_prelude as serenity,
    serenity_prelude::{
        ApplicationCommandInteraction, ApplicationCommandInteractionData, ApplicationCommandType,
        InteractionType,
    },
    ApplicationCommandOrAutocompleteInteraction, ApplicationContext, Command,
};

use crate::{component_models::ComponentDataModel, Context, Data, Error};

pub async fn listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::CacheReady { .. } => {
            info!("Cache Ready!");
            info!("Preparing to send reminders");

            if !data.is_loop_running.load(Ordering::Relaxed) {
                let ctx1 = ctx.clone();
                let ctx2 = ctx.clone();

                let pool1 = data.database.clone();
                let pool2 = data.database.clone();

                let run_settings = env::var("DONTRUN").unwrap_or_else(|_| "".to_string());

                if !run_settings.contains("postman") {
                    tokio::spawn(async move {
                        postman::initialize(ctx1, &pool1).await;
                    });
                } else {
                    warn!("Not running postman")
                }

                if !run_settings.contains("web") {
                    tokio::spawn(async move {
                        reminder_web::initialize(ctx2, pool2).await.unwrap();
                    });
                } else {
                    warn!("Not running web")
                }

                data.is_loop_running.swap(true, Ordering::Relaxed);
            }
        }
        poise::Event::ChannelDelete { channel } => {
            sqlx::query!(
                "
DELETE FROM channels WHERE channel = ?
                ",
                channel.id.as_u64()
            )
            .execute(&data.database)
            .await
            .unwrap();
        }
        poise::Event::GuildCreate { guild, is_new } => {
            if *is_new {
                let guild_id = guild.id.as_u64().to_owned();

                sqlx::query!("INSERT INTO guilds (guild) VALUES (?)", guild_id)
                    .execute(&data.database)
                    .await
                    .unwrap();

                if let Ok(token) = env::var("DISCORDBOTS_TOKEN") {
                    let shard_count = ctx.cache.shard_count();
                    let current_shard_id = shard_id(guild_id, shard_count);

                    let guild_count = ctx
                        .cache
                        .guilds()
                        .iter()
                        .filter(|g| {
                            shard_id(g.as_u64().to_owned(), shard_count) == current_shard_id
                        })
                        .count() as u64;

                    let mut hm = HashMap::new();
                    hm.insert("server_count", guild_count);
                    hm.insert("shard_id", current_shard_id);
                    hm.insert("shard_count", shard_count);

                    let response = data
                        .http
                        .post(
                            format!(
                                "https://top.gg/api/bots/{}/stats",
                                ctx.cache.current_user_id().as_u64()
                            )
                            .as_str(),
                        )
                        .header("Authorization", token)
                        .json(&hm)
                        .send()
                        .await;

                    if let Err(res) = response {
                        println!("DiscordBots Response: {:?}", res);
                    }
                }
            }
        }
        poise::Event::GuildDelete { incomplete, .. } => {
            let _ = sqlx::query!("DELETE FROM guilds WHERE guild = ?", incomplete.id.0)
                .execute(&data.database)
                .await;
        }
        poise::Event::InteractionCreate { interaction } => match interaction {
            Interaction::MessageComponent(component) => {
                let component_model = ComponentDataModel::from_custom_id(&component.data.custom_id);

                // component_model.act(ctx, component).await;
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}
