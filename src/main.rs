#[macro_use]
extern crate lazy_static;

mod models;
mod framework;
mod commands;
mod time_parser;
mod consts;

use serenity::{
    cache::Cache,
    http::CacheHttp,
    client::{
        bridge::gateway::GatewayIntents,
        Client,
    },
    model::{
        id::{
            GuildId, UserId,
        },
        channel::Message,
    },
    framework::Framework,
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

use crate::{
    framework::RegexFramework,
    consts::{
        PREFIX, DAY, HOUR, MINUTE,
    },
    commands::{
        info_cmds,
        reminder_cmds,
        todo_cmds,
        moderation_cmds,
    },
};
use num_integer::Integer;

struct SQLPool;

impl TypeMapKey for SQLPool {
    type Value = Pool<MySqlConnection>;
}

struct ReqwestClient;

impl TypeMapKey for ReqwestClient {
    type Value = Arc<reqwest::Client>;
}

struct FrameworkCtx;

impl TypeMapKey for FrameworkCtx {
    type Value = Arc<Box<dyn Framework + Send + Sync>>;
}

static THEME_COLOR: u32 = 0x8fb677;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv()?;

    let framework = RegexFramework::new(env::var("CLIENT_ID").expect("Missing CLIENT_ID from environment").parse()?)
        .ignore_bots(true)
        .default_prefix(&env::var("DEFAULT_PREFIX").unwrap_or_else(|_| PREFIX.to_string()))

        .add_command("ping", &info_cmds::PING_COMMAND)

        .add_command("help", &info_cmds::HELP_COMMAND)
        .add_command("info", &info_cmds::INFO_COMMAND)
        .add_command("invite", &info_cmds::INFO_COMMAND)
        .add_command("donate", &info_cmds::DONATE_COMMAND)
        .add_command("dashboard", &info_cmds::DASHBOARD_COMMAND)
        .add_command("clock", &info_cmds::CLOCK_COMMAND)

        .add_command("timer", &reminder_cmds::TIMER_COMMAND)

        .add_command("remind", &reminder_cmds::REMIND_COMMAND)
        .add_command("r", &reminder_cmds::REMIND_COMMAND)
        .add_command("interval", &reminder_cmds::INTERVAL_COMMAND)
        .add_command("i", &reminder_cmds::INTERVAL_COMMAND)
        .add_command("natural", &reminder_cmds::NATURAL_COMMAND)
        .add_command("n", &reminder_cmds::NATURAL_COMMAND)
        .add_command("", &reminder_cmds::NATURAL_COMMAND)

        .add_command("look", &reminder_cmds::LOOK_COMMAND)
        .add_command("del", &reminder_cmds::DELETE_COMMAND)

        .add_command("todo", &todo_cmds::TODO_PARSE_COMMAND)

        .add_command("blacklist", &moderation_cmds::BLACKLIST_COMMAND)
        .add_command("restrict", &moderation_cmds::RESTRICT_COMMAND)
        .add_command("timezone", &moderation_cmds::TIMEZONE_COMMAND)
        .add_command("prefix", &moderation_cmds::PREFIX_COMMAND)
        .add_command("lang", &moderation_cmds::LANGUAGE_COMMAND)

        .add_command("pause", &reminder_cmds::PAUSE_COMMAND)
        .add_command("offset", &reminder_cmds::OFFSET_COMMAND)
        .add_command("nudge", &reminder_cmds::NUDGE_COMMAND)

        .add_command("alias", &moderation_cmds::ALIAS_COMMAND)
        .add_command("a", &moderation_cmds::ALIAS_COMMAND)

        .build();

    let framework_arc = Arc::new(Box::new(framework) as Box<dyn Framework + Send + Sync>);

    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment"))
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES)
        .framework_arc(framework_arc.clone())
        .await.expect("Error occurred creating client");

    {
        let pool = MySqlPool::new(&env::var("DATABASE_URL").expect("Missing DATABASE_URL from environment")).await.unwrap();

        let mut data = client.data.write().await;

        data.insert::<SQLPool>(pool);
        data.insert::<ReqwestClient>(Arc::new(reqwest::Client::new()));
        data.insert::<FrameworkCtx>(framework_arc);
    }

    client.start_autosharded().await?;

    Ok(())
}


pub async fn check_subscription(cache_http: impl CacheHttp, user_id: impl Into<UserId>) -> bool {
    let role_ids = env::var("SUBSCRIPTION_ROLES")
        .map(
            |var| var
                .split(',')
                .filter_map(|item| {
                    item.parse::<u64>().ok()
                })
                .collect::<Vec<u64>>()
        );

    if let Some(subscription_guild) = env::var("CNC_GUILD").map(|var| var.parse::<u64>().ok()).ok().flatten() {
        if let Ok(role_ids) = role_ids {
            // todo remove unwrap and propagate error
            let guild_member = GuildId(subscription_guild).member(cache_http, user_id).await.unwrap();

            for role in guild_member.roles {
                if role_ids.contains(role.as_u64()) {
                    return true
                }
            }
        }

        false
    }
    else {
        true
    }
}

pub async fn check_subscription_on_message(cache_http: impl CacheHttp + AsRef<Cache>, msg: &Message) -> bool {
    check_subscription(&cache_http, &msg.author).await ||
        if let Some(guild) = msg.guild(&cache_http).await { check_subscription(&cache_http, guild.owner_id).await } else { false }
}

pub fn shorthand_displacement(seconds: u64) -> String {
    let (hours, seconds) = seconds.div_rem(&HOUR);
    let (minutes, seconds) = seconds.div_rem(&MINUTE);

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn longhand_displacement(seconds: u64) -> String {
    let (days, seconds) = seconds.div_rem(&DAY);
    let (hours, seconds) = seconds.div_rem(&HOUR);
    let (minutes, seconds) = seconds.div_rem(&MINUTE);

    let mut sections = vec![];

    for (var, name) in [days, hours, minutes, seconds].iter().zip(["days", "hours", "minutes", "seconds"].iter()) {
        if *var > 0 {
            sections.push(format!("{} {}", var, name));
        }
    }

    sections.join(", ")
}
