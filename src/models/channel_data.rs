use serenity::model::channel::Channel;

use sqlx::MySqlPool;

use chrono::NaiveDateTime;

pub struct ChannelData {
    pub id: u32,
    pub name: Option<String>,
    pub nudge: i16,
    pub blacklisted: bool,
    pub webhook_id: Option<u64>,
    pub webhook_token: Option<String>,
    pub paused: bool,
    pub paused_until: Option<NaiveDateTime>,
}

impl ChannelData {
    pub async fn from_channel(
        channel: Channel,
        pool: &MySqlPool,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
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
INSERT IGNORE INTO channels (channel, name, guild_id) VALUES (?, ?, (SELECT id FROM guilds WHERE guild = ?))
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
