use chrono_tz::Tz;
use log::error;
use serenity::{
    http::CacheHttp,
    model::{id::UserId, user::User},
};
use sqlx::MySqlPool;

use crate::consts::LOCAL_TIMEZONE;

pub struct UserData {
    pub id: u32,
    pub user: u64,
    pub name: String,
    pub dm_channel: u32,
    pub timezone: String,
}

impl UserData {
    pub async fn timezone_of<U>(user: U, pool: &MySqlPool) -> Tz
    where
        U: Into<UserId>,
    {
        let user_id = user.into().as_u64().to_owned();

        match sqlx::query!(
            "
SELECT timezone FROM users WHERE user = ?
            ",
            user_id
        )
        .fetch_one(pool)
        .await
        {
            Ok(r) => r.timezone,

            Err(_) => LOCAL_TIMEZONE.clone(),
        }
        .parse()
        .unwrap()
    }

    pub async fn from_user(
        user: &User,
        ctx: impl CacheHttp,
        pool: &MySqlPool,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let user_id = user.id.as_u64().to_owned();

        match sqlx::query_as_unchecked!(
            Self,
            "
SELECT id, user, name, dm_channel, IF(timezone IS NULL, ?, timezone) AS timezone FROM users WHERE user = ?
            ",
            *LOCAL_TIMEZONE, user_id
        )
        .fetch_one(pool)
        .await
        {
            Ok(c) => Ok(c),

            Err(sqlx::Error::RowNotFound) => {
                let dm_channel = user.create_dm_channel(ctx).await?;
                let dm_id = dm_channel.id.as_u64().to_owned();

                let pool_c = pool.clone();

                sqlx::query!(
                    "
INSERT IGNORE INTO channels (channel) VALUES (?)
                    ",
                    dm_id
                )
                .execute(&pool_c)
                .await?;

                sqlx::query!(
                    "
INSERT INTO users (user, name, dm_channel, timezone) VALUES (?, ?, (SELECT id FROM channels WHERE channel = ?), ?)
                    ", user_id, user.name, dm_id, *LOCAL_TIMEZONE)
                    .execute(&pool_c)
                    .await?;

                Ok(sqlx::query_as_unchecked!(
                    Self,
                    "
SELECT id, user, name, dm_channel, timezone FROM users WHERE user = ?
                    ",
                    user_id
                )
                .fetch_one(pool)
                .await?)
            }

            Err(e) => {
                error!("Error querying for user: {:?}", e);

                Err(Box::new(e))
            },
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE users SET name = ?, timezone = ? WHERE id = ?
            ",
            self.name,
            self.timezone,
            self.id
        )
        .execute(pool)
        .await
        .unwrap();
    }

    pub fn timezone(&self) -> Tz {
        self.timezone.parse().unwrap()
    }
}
