mod framework;

use serenity::{
    client::{
        bridge::gateway::GatewayIntents,
        Client, Context,
    },
    model::{
        channel::{
            Message,
        },
    },
    framework::standard::{
        Args, CommandResult,
        macros::{
            command,
        }
    },
    prelude::TypeMapKey,
};

use sqlx::{
    Pool,
    mysql::{
        MySqlConnection,
    }
};

use dotenv::dotenv;

use std::{
    sync::Arc,
    env,
};

use crate::framework::{RegexFramework, Command, PermissionLevel};

struct SQLPool;

impl TypeMapKey for SQLPool {
    type Value = Pool<MySqlConnection>;
}

struct ReqwestClient;

impl TypeMapKey for ReqwestClient {
    type Value = Arc<reqwest::Client>;
}

static THEME_COLOR: u32 = 0x00e0f3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv()?;

    let framework = RegexFramework::new()
        .ignore_bots(true)
        .default_prefix("$")
        .add_command(Command::from("help", PermissionLevel::Unrestricted, help_command))
        .build();

    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment"))
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES)
        .framework(framework)
        .await.expect("Error occured creating client");

    client.start_autosharded().await?;

    Ok(())
}

#[command]
async fn help_command(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    println!("Help command called");

    Ok(())
}
