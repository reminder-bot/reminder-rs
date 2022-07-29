use chrono_tz::Tz;
use log::error;
use poise::serenity::{http::CacheHttp, model::id::UserId};
use sqlx::MySqlPool;

use crate::consts::LOCAL_TIMEZONE;

pub struct UserData {
    pub id: u32,
    pub user: u64,
    pub dm_channel: u32,
    pub timezone: String,
    pub allowed_dm: bool,
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

    pub async fn from_user<U: Into<UserId>>(
        user: U,
        ctx: impl CacheHttp,
        pool: &MySqlPool,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let user_id = user.into();

        match sqlx::query_as_unchecked!(
            Self,
            "
SELECT id, user, dm_channel, IF(timezone IS NULL, ?, timezone) AS timezone, allowed_dm FROM users WHERE user = ?
            ",
            *LOCAL_TIMEZONE,
            user_id.0
        )
        .fetch_one(pool)
        .await
        {
            Ok(c) => Ok(c),

            Err(sqlx::Error::RowNotFound) => {
                let dm_channel = user_id.create_dm_channel(ctx).await?;
                let pool_c = pool.clone();

                sqlx::query!(
                    "
INSERT IGNORE INTO channels (channel) VALUES (?)
                    ",
                    dm_channel.id.0
                )
                .execute(&pool_c)
                .await?;

                sqlx::query!(
                    "
INSERT INTO users (name, user, dm_channel, timezone) VALUES ('', ?, (SELECT id FROM channels WHERE channel = ?), ?)
                    ",
                    user_id.0,
                    dm_channel.id.0,
                    *LOCAL_TIMEZONE
                )
                .execute(&pool_c)
                .await?;

                Ok(sqlx::query_as_unchecked!(
                    Self,
                    "
SELECT id, user, dm_channel, timezone, allowed_dm FROM users WHERE user = ?
                    ",
                    user_id.0
                )
                .fetch_one(pool)
                .await?)
            }

            Err(e) => {
                error!("Error querying for user: {:?}", e);

                Err(Box::new(e))
            }
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE users SET timezone = ?, allowed_dm = ? WHERE id = ?
            ",
            self.timezone,
            self.allowed_dm,
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
