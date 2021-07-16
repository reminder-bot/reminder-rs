use sqlx::MySqlPool;

use chrono::NaiveDateTime;

pub struct Timer {
    pub name: String,
    pub start_time: NaiveDateTime,
    pub owner: u64,
}

impl Timer {
    pub async fn from_owner(owner: u64, pool: &MySqlPool) -> Vec<Self> {
        sqlx::query_as_unchecked!(
            Timer,
            "
SELECT name, start_time, owner FROM timers WHERE owner = ?
            ",
            owner
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    pub async fn count_from_owner(owner: u64, pool: &MySqlPool) -> u32 {
        sqlx::query!(
            "
SELECT COUNT(1) as count FROM timers WHERE owner = ?
            ",
            owner
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .count as u32
    }

    pub async fn create(name: &str, owner: u64, pool: &MySqlPool) {
        sqlx::query!(
            "
INSERT INTO timers (name, owner) VALUES (?, ?)
            ",
            name,
            owner
        )
        .execute(pool)
        .await
        .unwrap();
    }
}
