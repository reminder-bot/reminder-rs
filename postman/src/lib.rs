mod sender;

use log::info;
use serenity::client::Context;
use sqlx::{Executor, MySql};
use std::env;
use tokio::time::sleep_until;
use tokio::time::{Duration, Instant};

type Database = MySql;

pub async fn initialize(ctx: Context, pool: impl Executor<'_, Database = Database> + Copy) {
    let REMIND_INTERVAL = env::var("REMIND_INTERVAL")
        .map(|inner| inner.parse::<u64>().ok())
        .ok()
        .flatten()
        .unwrap_or(10);

    loop {
        let sleep_to = Instant::now() + Duration::from_secs(REMIND_INTERVAL);
        let reminders = sender::Reminder::fetch_reminders(pool).await;

        if reminders.len() > 0 {
            info!("Preparing to send {} reminders.", reminders.len());

            for reminder in reminders {
                reminder.send(pool, ctx.clone()).await;
            }
        }

        sleep_until(sleep_to).await;
    }
}
