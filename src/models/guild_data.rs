use poise::serenity_prelude::{model::channel::Channel, Guild};
use sqlx::MySqlPool;

pub struct GuildData {
    pub id: u64,
    pub default_channel: Option<u64>,
}

impl GuildData {
    pub async fn from_guild(guild: &Guild, pool: &MySqlPool) -> Result<Self, sqlx::Error> {
        let guild_id = guild.id.0;

        if let Ok(row) = sqlx::query_as_unchecked!(
            Self,
            "
SELECT id, default_channel FROM guilds WHERE id = ?
            ",
            guild_id
        )
        .fetch_one(pool)
        .await
        {
            Ok(row)
        } else {
            sqlx::query!(
                "
INSERT IGNORE INTO guilds (id) VALUES (?)
                ",
                guild_id
            )
            .execute(&pool.clone())
            .await?;

            Ok(Self { id: guild_id, default_channel: None })
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "
UPDATE guilds SET default_channel = ? WHERE id = ?
            ",
            self.default_channel,
            self.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
