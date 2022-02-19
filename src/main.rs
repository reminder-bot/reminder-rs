#![feature(int_roundings)]
#[macro_use]
extern crate lazy_static;

mod commands;
mod component_models;
mod consts;
mod event_handlers;
mod framework;
mod hooks;
mod interval_parser;
mod models;
mod time_parser;
mod utils;

use std::{
    collections::HashMap,
    env,
    sync::{atomic::AtomicBool, Arc},
};

use chrono_tz::Tz;
use dotenv::dotenv;
use log::info;
use serenity::{
    client::Client,
    http::client::Http,
    model::{
        gateway::GatewayIntents,
        id::{GuildId, UserId},
    },
    prelude::TypeMapKey,
};
use sqlx::mysql::MySqlPool;
use tokio::sync::RwLock;

use crate::{
    commands::{info_cmds, moderation_cmds, reminder_cmds, todo_cmds},
    component_models::ComponentDataModel,
    consts::THEME_COLOR,
    framework::RegexFramework,
    models::command_macro::CommandMacro,
};

struct SQLPool;

impl TypeMapKey for SQLPool {
    type Value = MySqlPool;
}

struct ReqwestClient;

impl TypeMapKey for ReqwestClient {
    type Value = Arc<reqwest::Client>;
}

struct PopularTimezones;

impl TypeMapKey for PopularTimezones {
    type Value = Arc<Vec<Tz>>;
}

struct RecordingMacros;

impl TypeMapKey for RecordingMacros {
    type Value = Arc<RwLock<HashMap<(GuildId, UserId), CommandMacro>>>;
}

struct Handler {
    is_loop_running: AtomicBool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    dotenv()?;

    let token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment");

    let application_id = {
        let http = Http::new_with_token(&token);

        http.get_current_application_info().await?.id
    };

    let dm_enabled = env::var("DM_ENABLED").map_or(true, |var| var == "1");

    let framework = RegexFramework::new()
        .ignore_bots(env::var("IGNORE_BOTS").map_or(true, |var| var == "1"))
        .debug_guild(env::var("DEBUG_GUILD").map_or(None, |g| {
            Some(GuildId(g.parse::<u64>().expect("DEBUG_GUILD must be a guild ID")))
        }))
        .dm_enabled(dm_enabled)
        // info commands
        .add_command(&info_cmds::HELP_COMMAND)
        .add_command(&info_cmds::INFO_COMMAND)
        .add_command(&info_cmds::DONATE_COMMAND)
        .add_command(&info_cmds::DASHBOARD_COMMAND)
        .add_command(&info_cmds::CLOCK_COMMAND)
        // reminder commands
        .add_command(&reminder_cmds::TIMER_COMMAND)
        .add_command(&reminder_cmds::REMIND_COMMAND)
        // management commands
        .add_command(&reminder_cmds::DELETE_COMMAND)
        .add_command(&reminder_cmds::LOOK_COMMAND)
        .add_command(&reminder_cmds::PAUSE_COMMAND)
        .add_command(&reminder_cmds::OFFSET_COMMAND)
        .add_command(&reminder_cmds::NUDGE_COMMAND)
        // to-do commands
        .add_command(&todo_cmds::TODO_COMMAND)
        // moderation commands
        .add_command(&moderation_cmds::TIMEZONE_COMMAND)
        .add_command(&moderation_cmds::MACRO_CMD_COMMAND)
        .add_hook(&hooks::CHECK_SELF_PERMISSIONS_HOOK)
        .add_hook(&hooks::MACRO_CHECK_HOOK);

    let framework_arc = Arc::new(framework);

    let mut client = Client::builder(&token)
        .intents(GatewayIntents::GUILDS)
        .application_id(application_id.0)
        .event_handler(Handler { is_loop_running: AtomicBool::from(false) })
        .await
        .expect("Error occurred creating client");

    {
        let pool = MySqlPool::connect(
            &env::var("DATABASE_URL").expect("Missing DATABASE_URL from environment"),
        )
        .await
        .unwrap();

        let popular_timezones = sqlx::query!(
            "SELECT timezone FROM users GROUP BY timezone ORDER BY COUNT(timezone) DESC LIMIT 21"
        )
        .fetch_all(&pool)
        .await
        .unwrap()
        .iter()
        .map(|t| t.timezone.parse::<Tz>().unwrap())
        .collect::<Vec<Tz>>();

        let mut data = client.data.write().await;

        data.insert::<SQLPool>(pool);
        data.insert::<PopularTimezones>(Arc::new(popular_timezones));
        data.insert::<ReqwestClient>(Arc::new(reqwest::Client::new()));
        data.insert::<RegexFramework>(framework_arc.clone());
        data.insert::<RecordingMacros>(Arc::new(RwLock::new(HashMap::new())));
    }

    framework_arc.build_slash(&client.cache_and_http.http).await;

    info!("Starting client as autosharded");

    client.start_autosharded().await?;

    Ok(())
}
