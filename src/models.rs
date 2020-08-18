
use sqlx::MySqlPool;
use chrono::NaiveDateTime;

struct Channel {
    id: u32,
    channel: u64,
    name: String,
    nudge: i16,
    blacklisted: bool,
    webhook_id: u64,
    webhook_token: String,
    paused: bool,
    paused_until: NaiveDateTime,
    guild_id: u32,
}

impl Channel {
    async fn from_id(channel: u64, pool: MySqlPool) -> Result<Channel, Box<dyn std::error::Error + Sync + Send>> {
        if let Some(c) = sqlx::query_as!(Self,
            "
SELECT * FROM channels WHERE channel = ?
            ", channel)
            .fetch_one(&pool)
            .await? {

            c
        }
        else {
            sqlx::query!(
            "
INSERT INTO channels (channel, guild_id) VALUES ()
            "
            )
        }
    }
}
