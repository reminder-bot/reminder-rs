use std::{collections::HashMap, env};

use poise::{
    serenity::{model::application::interaction::Interaction, utils::shard_id},
    serenity_prelude as serenity,
};

use crate::{component_models::ComponentDataModel, Data, Error};

pub async fn listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { .. } => {
            ctx.set_activity(serenity::Activity::watching("for /remind")).await;
        }
        poise::Event::ChannelDelete { channel } => {
            sqlx::query!("DELETE FROM channels WHERE channel = ?", channel.id.as_u64())
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
        poise::Event::InteractionCreate { interaction } => {
            if let Interaction::MessageComponent(component) = interaction {
                let component_model = ComponentDataModel::from_custom_id(&component.data.custom_id);

                component_model.act(ctx, data, component).await;
            }
        }
        _ => {}
    }

    Ok(())
}
