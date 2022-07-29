mod sender;

use std::env;

use log::{info, warn};
use serenity::client::Context;
use sqlx::{Executor, MySql};
use tokio::{
    sync::broadcast::Receiver,
    time::{sleep_until, Duration, Instant},
};

type Database = MySql;

pub async fn initialize(
    mut kill: Receiver<()>,
    ctx: Context,
    pool: impl Executor<'_, Database = Database> + Copy,
) -> Result<(), &'static str> {
    tokio::select! {
        output = _initialize(ctx, pool) => Ok(output),
        _ = kill.recv() => {
            warn!("Received terminate signal. Goodbye");
            Err("Received terminate signal. Goodbye")
        }
    }
}

async fn _initialize(ctx: Context, pool: impl Executor<'_, Database = Database> + Copy) {
    let remind_interval = env::var("REMIND_INTERVAL")
        .map(|inner| inner.parse::<u64>().ok())
        .ok()
        .flatten()
        .unwrap_or(10);

    loop {
        let sleep_to = Instant::now() + Duration::from_secs(remind_interval);
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
