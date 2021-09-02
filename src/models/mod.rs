pub mod channel_data;
pub mod guild_data;
pub mod reminder;
pub mod timer;
pub mod user_data;

use serenity::{
    async_trait,
    model::id::{GuildId, UserId},
    prelude::Context,
};

use crate::{consts::DEFAULT_PREFIX, GuildDataCache, SQLPool};

use guild_data::GuildData;

use crate::models::user_data::UserData;

use std::sync::Arc;

use tokio::sync::RwLock;

#[async_trait]
pub trait CtxData {
    async fn guild_data<G: Into<GuildId> + Send + Sync>(
        &self,
        guild_id: G,
    ) -> Result<Arc<RwLock<GuildData>>, sqlx::Error>;

    async fn user_data<U: Into<UserId> + Send + Sync>(
        &self,
        user_id: U,
    ) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>>;

    async fn prefix<G: Into<GuildId> + Send + Sync>(&self, guild_id: Option<G>) -> String;
}

#[async_trait]
impl CtxData for Context {
    async fn guild_data<G: Into<GuildId> + Send + Sync>(
        &self,
        guild_id: G,
    ) -> Result<Arc<RwLock<GuildData>>, sqlx::Error> {
        let guild_id = guild_id.into();

        let guild = guild_id.to_guild_cached(&self.cache).unwrap();

        let guild_cache = self
            .data
            .read()
            .await
            .get::<GuildDataCache>()
            .cloned()
            .unwrap();

        let x = if let Some(guild_data) = guild_cache.get(&guild_id) {
            Ok(guild_data.clone())
        } else {
            let pool = self.data.read().await.get::<SQLPool>().cloned().unwrap();

            match GuildData::from_guild(guild, &pool).await {
                Ok(d) => {
                    let lock = Arc::new(RwLock::new(d));

                    guild_cache.insert(guild_id, lock.clone());

                    Ok(lock)
                }

                Err(e) => Err(e),
            }
        };

        x
    }

    async fn user_data<U: Into<UserId> + Send + Sync>(
        &self,
        user_id: U,
    ) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>> {
        let user_id = user_id.into();
        let pool = self.data.read().await.get::<SQLPool>().cloned().unwrap();

        let user = user_id.to_user(self).await.unwrap();

        UserData::from_user(&user, &self, &pool).await
    }

    async fn prefix<G: Into<GuildId> + Send + Sync>(&self, guild_id: Option<G>) -> String {
        if let Some(guild_id) = guild_id {
            self.guild_data(guild_id)
                .await
                .unwrap()
                .read()
                .await
                .prefix
                .clone()
        } else {
            DEFAULT_PREFIX.clone()
        }
    }
}
