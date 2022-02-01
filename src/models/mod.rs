pub mod channel_data;
pub mod command_macro;
pub mod reminder;
pub mod timer;
pub mod user_data;

use chrono_tz::Tz;
use poise::serenity::{async_trait, model::id::UserId};

use crate::{
    models::{channel_data::ChannelData, user_data::UserData},
    Context,
};

#[async_trait]
pub trait CtxData {
    async fn user_data<U: Into<UserId> + Send>(
        &self,
        user_id: U,
    ) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>>;

    async fn author_data(&self) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>>;

    async fn timezone(&self) -> Tz;

    async fn channel_data(&self) -> Result<ChannelData, Box<dyn std::error::Error + Sync + Send>>;
}

#[async_trait]
impl CtxData for Context<'_> {
    async fn user_data<U: Into<UserId> + Send>(
        &self,
        user_id: U,
    ) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>> {
        UserData::from_user(user_id, &self.discord(), &self.data().database).await
    }

    async fn author_data(&self) -> Result<UserData, Box<dyn std::error::Error + Sync + Send>> {
        UserData::from_user(&self.author().id, &self.discord(), &self.data().database).await
    }

    async fn timezone(&self) -> Tz {
        UserData::timezone_of(self.author().id, &self.data().database).await
    }

    async fn channel_data(&self) -> Result<ChannelData, Box<dyn std::error::Error + Sync + Send>> {
        let channel = self.channel_id().to_channel_cached(&self.discord()).unwrap();

        ChannelData::from_channel(&channel, &self.data().database).await
    }
}
