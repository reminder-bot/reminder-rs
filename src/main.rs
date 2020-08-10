mod framework;
mod commands;

use serenity::{
    client::{
        bridge::gateway::GatewayIntents,
        Client,
    },
    prelude::TypeMapKey,
};

use sqlx::{
    Pool,
    mysql::{
        MySqlPool,
        MySqlConnection,
    }
};

use dotenv::dotenv;

use std::{
    sync::Arc,
    env,
};

use crate::framework::RegexFramework;
use crate::commands::{
    info_cmds,
    reminder_cmds,
    todo_cmds,
};

struct SQLPool;

impl TypeMapKey for SQLPool {
    type Value = Pool<MySqlConnection>;
}

struct ReqwestClient;

impl TypeMapKey for ReqwestClient {
    type Value = Arc<reqwest::Client>;
}

static THEME_COLOR: u32 = 0x8fb677;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv()?;

    let framework = RegexFramework::new(env::var("CLIENT_ID").expect("Missing CLIENT_ID from environment").parse()?)
        .ignore_bots(true)
        .default_prefix("$")
        .add_command("help", &info_cmds::HELP_COMMAND)
        .add_command("info", &info_cmds::INFO_COMMAND)
        .add_command("donate", &info_cmds::DONATE_COMMAND)
        .build();

    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment"))
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES)
        .framework(framework)
        .await.expect("Error occurred creating client");

    {
        let pool = MySqlPool::new(&env::var("DATABASE_URL").expect("Missing DATABASE_URL from environment")).await.unwrap();

        let mut data = client.data.write().await;

        data.insert::<SQLPool>(pool);
        data.insert::<ReqwestClient>(Arc::new(reqwest::Client::new()));
    }

    client.start_autosharded().await?;

    Ok(())
}
