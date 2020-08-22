use serenity::model::{
    guild::Guild,
    channel::Channel
};

use sqlx::MySqlPool;
use chrono::NaiveDateTime;

pub struct GuildData {
    id: u32,
    guild: u64,
    name: String,
    prefix: String,
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

impl GuildData {
    pub async fn from_id(guild: Guild, pool: MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        if let Ok(g) = sqlx::query_as!(Self,
            "
SELECT id, guild, name, prefix FROM guilds WHERE guild = ?
            ", guild.id.as_u64())
            .fetch_one(&pool)
            .await {

            Ok(g)
        }
        else {
            sqlx::query!(
                "
INSERT INTO guilds (guild, name) VALUES (?, ?)
                ", guild.id.as_u64(), guild.name)
                .execute(&pool)
                .await?;

            sqlx::query_as!(Self,
            "
SELECT id, guild, name, prefix FROM guilds WHERE guild = ?
            ", guild.id.as_u64())
            .fetch_one(&pool)
            .await
        }
    }
}

impl ChannelData {
    pub async fn from_id(channel: Channel, pool: MySqlPool) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        if let Ok(c) = sqlx::query_as_unchecked!(Self,
            "
SELECT * FROM channels WHERE channel = ?
            ", channel.id().as_u64())
            .fetch_one(&pool)
            .await {

            Ok(c)
        }
        else {
            let guild_id = channel.guild().map(|g| g.guild_id.as_u64());
            let channel_name = channel.guild().map(|g| g.name);

            sqlx::query!(
                "
INSERT INTO channels (channel, name, guild_id) VALUES (?, ?, (SELECT id FROM guilds WHERE guild = ?))
                ", channel.id().as_u64(), channel_name, guild_id)
                .execute(&pool)
                .await?;

            sqlx::query_as_unchecked!(Self,
            "
SELECT * FROM channels WHERE channel = ?
            ", channel.id().as_u64())
            .fetch_one(&pool)
            .await
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
