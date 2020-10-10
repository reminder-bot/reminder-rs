use serenity::{
    http::CacheHttp,
    model::{
        id::GuildId,
        guild::Guild,
        channel::Channel,
        user::User,
    }
};

use std::env;

use sqlx::MySqlPool;

use chrono::NaiveDateTime;
use chrono_tz::Tz;

use crate::consts::PREFIX;

pub struct GuildData {
    pub id: u32,
    pub name: String,
    pub prefix: String,
}

impl GuildData {
    pub async fn prefix_from_id<T: Into<GuildId>>(guild_id_opt: Option<T>, pool: &MySqlPool) -> String {
        if let Some(guild_id) = guild_id_opt {
            let guild_id = guild_id.into().as_u64().to_owned();

            let row = sqlx::query!(
                "
SELECT prefix FROM guilds WHERE guild = ?
                ", guild_id
            )
                .fetch_one(pool)
                .await;

            row.map_or_else(|_| env::var("DEFAULT_PREFIX").unwrap_or_else(|_| PREFIX.to_string()), |r| r.prefix)
        }
        else {
            env::var("DEFAULT_PREFIX").unwrap_or_else(|_| PREFIX.to_string())
        }
    }

    pub async fn from_guild(guild: Guild, pool: &MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let guild_id = guild.id.as_u64().to_owned();

        if let Ok(g) = sqlx::query_as!(Self,
            "
SELECT id, name, prefix FROM guilds WHERE guild = ?
            ", guild_id)
            .fetch_one(pool)
            .await {

            Ok(g)
        }
        else {
            sqlx::query!(
                "
INSERT INTO guilds (guild, name, prefix) VALUES (?, ?, ?)
                ", guild_id, guild.name, env::var("DEFAULT_PREFIX").unwrap_or_else(|_| PREFIX.to_string()))
                .execute(&pool.clone())
                .await?;

            Ok(sqlx::query_as!(Self,
            "
SELECT id, name, prefix FROM guilds WHERE guild = ?
            ", guild_id)
            .fetch_one(pool)
            .await?)
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE guilds SET name = ?, prefix = ? WHERE id = ?
            ", self.name, self.prefix, self.id)
            .execute(pool)
            .await.unwrap();
    }
}

pub struct ChannelData {
    pub id: u32,
    pub name: String,
    pub nudge: i16,
    pub blacklisted: bool,
    pub webhook_id: Option<u64>,
    pub webhook_token: Option<String>,
    pub paused: bool,
    pub paused_until: Option<NaiveDateTime>,
}

impl ChannelData {
    pub async fn from_id(channel_id: u64, pool: &MySqlPool) -> Option<Self> {
        sqlx::query_as_unchecked!(Self,
            "
SELECT id, name, nudge, blacklisted, webhook_id, webhook_token, paused, paused_until FROM channels WHERE channel = ?
            ", channel_id)
            .fetch_one(pool)
            .await.ok()
    }

    pub async fn from_channel(channel: Channel, pool: &MySqlPool)
        -> Result<Self, Box<dyn std::error::Error + Sync + Send>>
    {
        let channel_id = channel.id().as_u64().to_owned();

        if let Ok(c) = sqlx::query_as_unchecked!(Self,
            "
SELECT id, name, nudge, blacklisted, webhook_id, webhook_token, paused, paused_until FROM channels WHERE channel = ?
            ", channel_id)
            .fetch_one(pool)
            .await {

            Ok(c)
        }
        else {
            let props = channel.guild().map(|g| (g.guild_id.as_u64().to_owned(), g.name));

            let (guild_id, channel_name) = if let Some((a, b)) = props {
                (Some(a), Some(b))
            } else {
                (None, None)
            };

            sqlx::query!(
                "
INSERT INTO channels (channel, name, guild_id) VALUES (?, ?, (SELECT id FROM guilds WHERE guild = ?))
                ", channel_id, channel_name, guild_id)
                .execute(&pool.clone())
                .await?;

            Ok(sqlx::query_as_unchecked!(Self,
                "
SELECT id, name, nudge, blacklisted, webhook_id, webhook_token, paused, paused_until FROM channels WHERE channel = ?
                ", channel_id)
                .fetch_one(pool)
                .await?)
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE channels SET name = ?, nudge = ?, blacklisted = ?, webhook_id = ?, webhook_token = ?, paused = ?, paused_until = ? WHERE id = ?
            ", self.name, self.nudge, self.blacklisted, self.webhook_id, self.webhook_token, self.paused, self.paused_until, self.id)
            .execute(pool)
            .await.unwrap();
    }
}

pub struct UserData {
    id: u32,
    pub user: u64,
    pub name: String,
    pub dm_channel: u32,
    pub language: String,
    pub timezone: String,
}

impl UserData {
    pub async fn from_user(user: &User, ctx: impl CacheHttp, pool: &MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let user_id = user.id.as_u64().to_owned();

        if let Ok(c) = sqlx::query_as_unchecked!(Self,
            "
SELECT id, user, name, dm_channel, language, timezone FROM users WHERE user = ?
            ", user_id)
            .fetch_one(pool)
            .await {

            Ok(c)
        }
        else {
            let dm_channel = user.create_dm_channel(ctx).await?;
            let dm_id = dm_channel.id.as_u64().to_owned();

            let pool_c = pool.clone();

            sqlx::query!(
                "
INSERT INTO channels (channel) VALUES (?)
                ", dm_id)
                .execute(&pool_c)
                .await?;

            sqlx::query!(
                "
INSERT INTO users (user, name, dm_channel) VALUES (?, ?, (SELECT id FROM channels WHERE channel = ?))
                ", user_id, user.name, dm_id)
                .execute(&pool_c)
                .await?;

            Ok(sqlx::query_as_unchecked!(Self,
                "
SELECT id, user, name, dm_channel, language, timezone FROM users WHERE user = ?
                ", user_id)
                .fetch_one(pool)
                .await?)
        }
    }

    pub async fn commit_changes(&self, pool: &MySqlPool) {
        sqlx::query!(
            "
UPDATE users SET name = ?, language = ?, timezone = ? WHERE id = ?
            ", self.name, self.language, self.timezone, self.id)
            .execute(pool)
            .await.unwrap();
    }

    pub async fn response(&self, pool: &MySqlPool, name: &str) -> String {
        let row = sqlx::query!(
            "
SELECT value FROM strings WHERE (language = ? OR language = 'EN') AND name = ? ORDER BY language = 'EN'
            ", self.language, name)
            .fetch_one(pool)
            .await
            .unwrap_or_else(|_| panic!("No string with that name: {}", name));

        row.value.unwrap_or_else(|| panic!("Null string with that name: {}", name))
    }

    pub fn timezone(&self) -> Tz {
        self.timezone.parse().unwrap()
    }
}

pub struct Timer {
    pub name: String,
    pub start_time: NaiveDateTime,
    pub owner: u64,
}

impl Timer {
    pub async fn from_owner(owner: u64, pool: &MySqlPool) -> Vec<Self> {
        sqlx::query_as_unchecked!(Timer,
            "
SELECT name, start_time, owner FROM timers WHERE owner = ?
            ", owner)
            .fetch_all(pool)
            .await
            .unwrap()
    }

    pub async fn count_from_owner(owner: u64, pool: &MySqlPool) -> u32 {
        sqlx::query!(
            "
SELECT COUNT(1) as count FROM timers WHERE owner = ?
            ", owner)
            .fetch_one(pool)
            .await
            .unwrap()
            .count as u32
    }

    pub async fn create(name: &str, owner: u64, pool: &MySqlPool) {
        sqlx::query!(
            "
INSERT INTO timers (name, owner) VALUES (?, ?)
            ", name, owner)
            .execute(pool)
            .await
            .unwrap();
    }
}
