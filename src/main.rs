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
    },
    prelude::TypeMapKey,
};

use regex_command_attr::command;

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

use crate::framework::RegexFramework;

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

    println!("{:?}", HELP_COMMAND);

    let framework = RegexFramework::new(env::var("CLIENT_ID").expect("Missing CLIENT_ID from environment").parse()?)
        .ignore_bots(true)
        .default_prefix("$")
        .add_command("help".to_string(), &HELP_COMMAND)
        .add_command("h".to_string(), &HELP_COMMAND)
        .build();

    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment"))
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES)
        .framework(framework)
        .await.expect("Error occurred creating client");

    client.start_autosharded().await?;

    Ok(())
}

#[command]
async fn help(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    println!("Help command called");

    Ok(())
}
