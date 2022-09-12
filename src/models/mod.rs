pub mod channel_data;
pub mod command_macro;
pub mod guild_data;
pub mod reminder;
pub mod timer;
pub mod user_data;

use chrono_tz::Tz;
use log::warn;
use poise::serenity_prelude::{async_trait, model::id::UserId, ChannelId};

use crate::{
    models::{channel_data::ChannelData, guild_data::GuildData, user_data::UserData},
    CommandMacro, Context, Data, Error, GuildId,
};

#[async_trait]
pub trait CtxData {
    async fn user_data<U: Into<UserId> + Send>(&self, user_id: U) -> Result<UserData, Error>;
    async fn author_data(&self) -> Result<UserData, Error>;
    async fn timezone(&self) -> Tz;
    async fn channel_data(&self) -> Result<ChannelData, Error>;
    async fn guild_data(&self) -> Option<GuildData>;
    async fn command_macros(&self) -> Result<Vec<CommandMacro<Data, Error>>, Error>;
    async fn default_channel(&self) -> Option<ChannelId>;
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

    async fn command_macros(&self) -> Result<Vec<CommandMacro<Data, Error>>, Error> {
        self.data().command_macros(self.guild_id().unwrap()).await
    }

    async fn default_channel(&self) -> Option<ChannelId> {
        match self.guild_id() {
            Some(guild_id) => {
                let guild_data = GuildData::from_guild(guild_id, &self.data().database).await;

                match guild_data {
                    Ok(data) => data.default_channel.map(|c| ChannelId(c)),

                    Err(e) => {
                        warn!("SQL error: {:?}", e);

                        None
                    }
                }
            }

            None => None,
        }
    }

    async fn guild_data(&self) -> Option<GuildData> {
        match self.guild_id() {
            Some(guild_id) => GuildData::from_guild(guild_id, &self.data().database).await.ok(),

            None => None,
        }
    }
}

impl Data {
    pub async fn command_macros(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<CommandMacro<Data, Error>>, Error> {
        let rows = sqlx::query!(
            "SELECT name, description, commands FROM macro WHERE guild_id = ?",
            guild_id.0
        )
        .fetch_all(&self.database)
        .await?
        .iter()
        .map(|row| CommandMacro {
            guild_id,
            name: row.name.clone(),
            description: row.description.clone(),
            commands: serde_json::from_str(&row.commands).unwrap(),
        })
        .collect();

        Ok(rows)
    }
}
