use std::{collections::HashMap, env};

use log::error;
use poise::{
    serenity_prelude as serenity,
    serenity_prelude::{model::application::interaction::Interaction, utils::shard_id},
};

use crate::{component_models::ComponentDataModel, Data, Error, THEME_COLOR};

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

                sqlx::query!("INSERT IGNORE INTO guilds (guild) VALUES (?)", guild_id)
                    .execute(&data.database)
                    .await?;

                if let Err(e) = post_guild_count(ctx, &data.http, guild_id).await {
                    error!("DiscordBotList: {:?}", e);
                }

                let default_channel = guild.default_channel_guaranteed();

                if let Some(default_channel) = default_channel {
                    default_channel
                        .send_message(&ctx, |m| {
                            m.embed(|e| {
                                e.title("Thank you for adding Reminder Bot!").description(
                                    "To get started:
• Set your timezone with `/timezone`
• Set up permissions in Server Settings 🠚 Integrations 🠚 Reminder Bot (desktop only)
• Create your first reminder with `/remind`

__Support__
If you need any support, please come and ask us! Join our [Discord](https://discord.jellywx.com).

__Updates__
To stay up to date on the latest features and fixes, join our [Discord](https://discord.jellywx.com).
",
                                ).color(*THEME_COLOR)
                            })
                        })
                        .await?;
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

async fn post_guild_count(
    ctx: &serenity::Context,
    http: &reqwest::Client,
    guild_id: u64,
) -> Result<(), reqwest::Error> {
    if let Ok(token) = env::var("DISCORDBOTS_TOKEN") {
        let shard_count = ctx.cache.shard_count();
        let current_shard_id = shard_id(guild_id, shard_count);

        let guild_count = ctx
            .cache
            .guilds()
            .iter()
            .filter(|g| shard_id(g.as_u64().to_owned(), shard_count) == current_shard_id)
            .count() as u64;

        let mut hm = HashMap::new();
        hm.insert("server_count", guild_count);
        hm.insert("shard_id", current_shard_id);
        hm.insert("shard_count", shard_count);

        http.post(
            format!("https://top.gg/api/bots/{}/stats", ctx.cache.current_user_id().as_u64())
                .as_str(),
        )
        .header("Authorization", token)
        .json(&hm)
        .send()
        .await
        .map(|_| ())
    } else {
        Ok(())
    }
}
