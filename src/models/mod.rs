pub mod channel_data;
pub mod guild_data;
pub mod reminder;
pub mod timer;
pub mod user_data;

use std::sync::Arc;

use chrono_tz::Tz;
use serenity::{
    async_trait,
    model::id::{ChannelId, GuildId, UserId},
    prelude::Context,
};
use tokio::sync::RwLock;

use crate::{
    consts::DEFAULT_PREFIX,
    models::{channel_data::ChannelData, guild_data::GuildData, user_data::UserData},
    GuildDataCache, SQLPool,
};

#[async_trait]
pub trait CtxData {
    async fn guild_data<G: Into<GuildId> + Send + Sync>(
        &self,
        guild_id: G,
    ) -> Result<Arc<RwLock<GuildData>>, sqlx::Error>;

    async fn prefix<G: Into<GuildId> + Send + Sync>(&self, guild_id: Option<G>) -> String;

    async fn user_data<U: Into<UserId> + Send + Sync>(
        &self,
        user_id: U,
    ) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>>;

    async fn timezone<U: Into<UserId> + Send + Sync>(&self, user_id: U) -> Tz;

    async fn channel_data<C: Into<ChannelId> + Send + Sync>(
        &self,
        channel_id: C,
    ) -> Result<ChannelData, Box<dyn std::error::Error + Sync + Send>>;
}

#[async_trait]
impl CtxData for Context {
    async fn guild_data<G: Into<GuildId> + Send + Sync>(
        &self,
        guild_id: G,
    ) -> Result<Arc<RwLock<GuildData>>, sqlx::Error> {
        let guild_id = guild_id.into();

        let guild = guild_id.to_guild_cached(&self.cache).unwrap();

        let guild_cache = self.data.read().await.get::<GuildDataCache>().cloned().unwrap();

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

    async fn prefix<G: Into<GuildId> + Send + Sync>(&self, guild_id: Option<G>) -> String {
        if let Some(guild_id) = guild_id {
            self.guild_data(guild_id).await.unwrap().read().await.prefix.clone()
        } else {
            DEFAULT_PREFIX.clone()
        }
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

    async fn timezone<U: Into<UserId> + Send + Sync>(&self, user_id: U) -> Tz {
        let user_id = user_id.into();
        let pool = self.data.read().await.get::<SQLPool>().cloned().unwrap();

        UserData::timezone_of(user_id, &pool).await
    }

    async fn channel_data<C: Into<ChannelId> + Send + Sync>(
        &self,
        channel_id: C,
    ) -> Result<ChannelData, Box<dyn std::error::Error + Sync + Send>> {
        let channel_id = channel_id.into();
        let pool = self.data.read().await.get::<SQLPool>().cloned().unwrap();

        let channel = channel_id.to_channel_cached(&self).unwrap();

        ChannelData::from_channel(&channel, &pool).await
    }
}
