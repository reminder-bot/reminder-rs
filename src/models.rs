use serenity::{
    prelude::Context,
    model::{
        id::ChannelId,
        guild::Guild,
        channel::Channel,
        user::User,
    }
};

use sqlx::MySqlPool;
use chrono::NaiveDateTime;

pub struct GuildData {
    id: u32,
    guild: u64,
    name: String,
    prefix: String,
}

impl GuildData {
    pub async fn from_id(guild: Guild, pool: MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let guild_id = guild.id.as_u64().clone();

        if let Ok(g) = sqlx::query_as!(Self,
            "
SELECT id, guild, name, prefix FROM guilds WHERE guild = ?
            ", guild_id)
            .fetch_one(&pool)
            .await {

            Ok(g)
        }
        else {
            sqlx::query!(
                "
INSERT INTO guilds (guild, name) VALUES (?, ?)
                ", guild_id, guild.name)
                .execute(&pool)
                .await?;

            Ok(sqlx::query_as!(Self,
            "
SELECT id, guild, name, prefix FROM guilds WHERE guild = ?
            ", guild_id)
            .fetch_one(&pool)
            .await?)
        }
    }
}

pub struct ChannelData {
    id: u32,
    channel: u64,
    pub name: String,
    pub nudge: i16,
    pub blacklisted: bool,
    pub webhook_id: Option<u64>,
    pub webhook_token: Option<String>,
    pub paused: bool,
    pub paused_until: Option<NaiveDateTime>,
    guild_id: u32,
}

impl ChannelData {
    pub async fn from_id(channel_id: u64, pool: MySqlPool) -> Option<Self> {
        sqlx::query_as_unchecked!(Self,
            "
SELECT * FROM channels WHERE channel = ?
            ", channel_id)
            .fetch_one(&pool)
            .await.ok()
    }

    pub async fn from_channel(channel: Channel, pool: MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let channel_id = channel.id().as_u64().clone();

        if let Ok(c) = sqlx::query_as_unchecked!(Self,
            "
SELECT * FROM channels WHERE channel = ?
            ", channel_id)
            .fetch_one(&pool)
            .await {

            Ok(c)
        }
        else {
            let props = channel.guild().map(|g| (g.guild_id.as_u64().clone(), g.name));

            let (guild_id, channel_name) = if let Some((a, b)) = props {
                (Some(a), Some(b))
            } else {
                (None, None)
            };

            sqlx::query!(
                "
INSERT INTO channels (channel, name, guild_id) VALUES (?, ?, (SELECT id FROM guilds WHERE guild = ?))
                ", channel_id, channel_name, guild_id)
                .execute(&pool)
                .await?;

            Ok(sqlx::query_as_unchecked!(Self,
                "
SELECT * FROM channels WHERE channel = ?
                ", channel_id)
                .fetch_one(&pool)
                .await?)
        }
    }

    pub async fn commit_changes(&self, pool: MySqlPool) {
        sqlx::query!(
            "
UPDATE channels SET name = ?, nudge = ?, blacklisted = ?, webhook_id = ?, webhook_token = ?, paused = ?, paused_until = ? WHERE id = ?
            ", self.name, self.nudge, self.blacklisted, self.webhook_id, self.webhook_token, self.paused, self.paused_until, self.id)
            .execute(&pool)
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
    pub async fn from_id(user: &User, ctx: &&Context, pool: MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let user_id = user.id.as_u64().clone();

        if let Ok(c) = sqlx::query_as_unchecked!(Self,
            "
SELECT id, user, name, dm_channel, language, timezone FROM users WHERE user = ?
            ", user_id)
            .fetch_one(&pool)
            .await {

            Ok(c)
        }
        else {
            let dm_channel = user.create_dm_channel(ctx).await?;
            let dm_id = dm_channel.id.as_u64().clone();

            sqlx::query!(
                "
INSERT INTO channels (channel) VALUES (?)
                ", dm_id)
                .execute(&pool)
                .await?;

            sqlx::query!(
                "
INSERT INTO users (user, name, dm_channel) VALUES (?, ?, (SELECT id FROM channels WHERE channel = ?))
                ", user_id, user.name, dm_id)
                .execute(&pool)
                .await?;

            Ok(sqlx::query_as_unchecked!(Self,
                "
SELECT id, user, name, dm_channel, language, timezone FROM users WHERE user = ?
                ", user_id)
                .fetch_one(&pool)
                .await?)
        }
    }

    pub async fn commit_changes(&self, pool: MySqlPool) {
        sqlx::query!(
            "
UPDATE users SET name = ?, language = ?, timezone = ? WHERE id = ?
            ", self.name, self.language, self.timezone, self.id)
            .execute(&pool)
            .await.unwrap();
    }
}
