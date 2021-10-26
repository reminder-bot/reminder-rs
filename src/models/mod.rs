pub mod channel_data;
pub mod command_macro;
pub mod reminder;
pub mod timer;
pub mod user_data;

use chrono_tz::Tz;
use serenity::{
    async_trait,
    model::id::{ChannelId, UserId},
    prelude::Context,
};

use crate::{
    models::{channel_data::ChannelData, user_data::UserData},
    SQLPool,
};

#[async_trait]
pub trait CtxData {
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
