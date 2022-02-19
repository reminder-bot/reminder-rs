use std::{collections::HashMap, env, sync::atomic::Ordering};

use log::{info, warn};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::GuildChannel,
        gateway::{Activity, Ready},
        guild::{Guild, UnavailableGuild},
        id::GuildId,
        interactions::Interaction,
    },
    utils::shard_id,
};

use crate::{ComponentDataModel, Handler, RegexFramework, ReqwestClient, SQLPool};

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx_base: Context, _guilds: Vec<GuildId>) {
        info!("Cache Ready!");
        info!("Preparing to send reminders");

        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx1 = ctx_base.clone();
            let ctx2 = ctx_base.clone();

            let pool1 = ctx1.data.read().await.get::<SQLPool>().cloned().unwrap();
            let pool2 = ctx2.data.read().await.get::<SQLPool>().cloned().unwrap();

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

            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn channel_delete(&self, ctx: Context, channel: &GuildChannel) {
        let pool = ctx
            .data
            .read()
            .await
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        sqlx::query!(
            "
DELETE FROM channels WHERE channel = ?
            ",
            channel.id.as_u64()
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            let guild_id = guild.id.as_u64().to_owned();

            {
                let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

                let _ = sqlx::query!("INSERT INTO guilds (guild) VALUES (?)", guild_id)
                    .execute(&pool)
                    .await;
            }

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

                let client = ctx
                    .data
                    .read()
                    .await
                    .get::<ReqwestClient>()
                    .cloned()
                    .expect("Could not get ReqwestClient from data");

                let response = client
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

    async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, _full: Option<Guild>) {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
        let _ = sqlx::query!("DELETE FROM guilds WHERE guild = ?", incomplete.id.0)
            .execute(&pool)
            .await;
    }

    async fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::watching("for /remind")).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(application_command) => {
                let framework = ctx
                    .data
                    .read()
                    .await
                    .get::<RegexFramework>()
                    .cloned()
                    .expect("RegexFramework not found in context");

                framework.execute(ctx, application_command).await;
            }
            Interaction::MessageComponent(component) => {
                let component_model = ComponentDataModel::from_custom_id(&component.data.custom_id);
                component_model.act(&ctx, component).await;
            }
            _ => {}
        }
    }
}
