use log::error;
use serenity::model::guild::Guild;
use sqlx::MySqlPool;

use crate::consts::DEFAULT_PREFIX;

pub struct GuildData {
    pub id: u32,
    pub name: Option<String>,
    pub prefix: String,
}

impl GuildData {
    pub async fn from_guild(guild: Guild, pool: &MySqlPool) -> Result<Self, sqlx::Error> {
        let guild_id = guild.id.as_u64().to_owned();

        match sqlx::query_as!(
            Self,
            "
SELECT id, name, prefix FROM guilds WHERE guild = ?
            ",
            guild_id
        )
        .fetch_one(pool)
        .await
        {
            Ok(mut g) => {
                g.name = Some(guild.name);

                Ok(g)
            }

            Err(sqlx::Error::RowNotFound) => {
                sqlx::query!(
                    "
INSERT INTO guilds (guild, name, prefix) VALUES (?, ?, ?)
                    ",
                    guild_id,
                    guild.name,
                    *DEFAULT_PREFIX
                )
                .execute(&pool.clone())
                .await?;

                Ok(sqlx::query_as!(
                    Self,
                    "
SELECT id, name, prefix FROM guilds WHERE guild = ?
                    ",
                    guild_id
                )
                .fetch_one(pool)
                .await?)
            }

            Err(e) => {
                error!("Unexpected error in guild query: {:?}", e);

                Err(e)
            }
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE guilds SET name = ?, prefix = ? WHERE id = ?
            ",
            self.name,
            self.prefix,
            self.id
        )
        .execute(pool)
        .await
        .unwrap();
    }
}
