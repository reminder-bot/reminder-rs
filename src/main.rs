#[macro_use]
extern crate lazy_static;

mod commands;
mod component_models;
mod consts;
mod framework;
mod hooks;
mod interval_parser;
mod models;
mod time_parser;

use std::{
    collections::HashMap,
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono_tz::Tz;
use dotenv::dotenv;
use log::info;
use postman::initialize;
use serenity::{
    async_trait,
    client::Client,
    http::{client::Http, CacheHttp},
    model::{
        channel::GuildChannel,
        gateway::{Activity, GatewayIntents, Ready},
        guild::{Guild, UnavailableGuild},
        id::{GuildId, UserId},
        interactions::Interaction,
    },
    prelude::{Context, EventHandler, TypeMapKey},
    utils::shard_id,
};
use sqlx::mysql::MySqlPool;
use tokio::sync::RwLock;

use crate::{
    commands::{info_cmds, moderation_cmds, reminder_cmds, todo_cmds},
    component_models::ComponentDataModel,
    consts::{CNC_GUILD, SUBSCRIPTION_ROLES, THEME_COLOR},
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

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx_base: Context, _guilds: Vec<GuildId>) {
        info!("Cache Ready!");
        info!("Preparing to send reminders");

        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx = ctx_base.clone();
            let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

            tokio::spawn(async move {
                initialize(ctx, &pool).await;
            });

            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn channel_delete(&self, ctx: Context, channel: &GuildChannel) {
        let pool = ctx
            .data
            .read()
            .await
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        sqlx::query!(
            "
DELETE FROM channels WHERE channel = ?
            ",
            channel.id.as_u64()
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            let guild_id = guild.id.as_u64().to_owned();

            {
                let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

                let _ = sqlx::query!("INSERT INTO guilds (guild) VALUES (?)", guild_id)
                    .execute(&pool)
                    .await;
            }

            if let Ok(token) = env::var("DISCORDBOTS_TOKEN") {
                let shard_count = ctx.cache.shard_count();
                let current_shard_id = shard_id(guild_id, shard_count);

                let guild_count = ctx
                    .cache
                    .guilds()
                    .iter()
                    .filter(|g| shard_id(g.as_u64().to_owned(), shard_count) == current_shard_id)
                    .count() as u64;

                let mut hm = HashMap::new();
                hm.insert("server_count", guild_count);
                hm.insert("shard_id", current_shard_id);
                hm.insert("shard_count", shard_count);

                let client = ctx
                    .data
                    .read()
                    .await
                    .get::<ReqwestClient>()
                    .cloned()
                    .expect("Could not get ReqwestClient from data");

                let response = client
                    .post(
                        format!(
                            "https://top.gg/api/bots/{}/stats",
                            ctx.cache.current_user_id().as_u64()
                        )
                        .as_str(),
                    )
                    .header("Authorization", token)
                    .json(&hm)
                    .send()
                    .await;

                if let Err(res) = response {
                    println!("DiscordBots Response: {:?}", res);
                }
            }
        }
    }

    async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, _full: Option<Guild>) {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
        let _ = sqlx::query!("DELETE FROM guilds WHERE guild = ?", incomplete.id.0)
            .execute(&pool)
            .await;
    }

    async fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::watching("for /remind")).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(application_command) => {
                let framework = ctx
                    .data
                    .read()
                    .await
                    .get::<RegexFramework>()
                    .cloned()
                    .expect("RegexFramework not found in context");

                framework.execute(ctx, application_command).await;
            }
            Interaction::MessageComponent(component) => {
                let component_model = ComponentDataModel::from_custom_id(&component.data.custom_id);
                component_model.act(&ctx, component).await;
            }
            _ => {}
        }
    }
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

    if let Ok((Some(lower), Some(upper))) = env::var("SHARD_RANGE").map(|sr| {
        let mut split =
            sr.split(',').map(|val| val.parse::<u64>().expect("SHARD_RANGE not an integer"));

        (split.next(), split.next())
    }) {
        let total_shards = env::var("SHARD_COUNT")
            .map(|shard_count| shard_count.parse::<u64>().ok())
            .ok()
            .flatten()
            .expect("No SHARD_COUNT provided, but SHARD_RANGE was provided");

        assert!(lower < upper, "SHARD_RANGE lower limit is not less than the upper limit");

        info!("Starting client fragment with shards {}-{}/{}", lower, upper, total_shards);

        client.start_shard_range([lower, upper], total_shards).await?;
    } else if let Ok(total_shards) = env::var("SHARD_COUNT")
        .map(|shard_count| shard_count.parse::<u64>().expect("SHARD_COUNT not an integer"))
    {
        info!("Starting client with {} shards", total_shards);

        client.start_shards(total_shards).await?;
    } else {
        info!("Starting client as autosharded");

        client.start_autosharded().await?;
    }

    Ok(())
}

pub async fn check_subscription(cache_http: impl CacheHttp, user_id: impl Into<UserId>) -> bool {
    if let Some(subscription_guild) = *CNC_GUILD {
        let guild_member = GuildId(subscription_guild).member(cache_http, user_id).await;

        if let Ok(member) = guild_member {
            for role in member.roles {
                if SUBSCRIPTION_ROLES.contains(role.as_u64()) {
                    return true;
                }
            }
        }

        false
    } else {
        true
    }
}

pub async fn check_guild_subscription(
    cache_http: impl CacheHttp,
    guild_id: impl Into<GuildId>,
) -> bool {
    if let Some(guild) = cache_http.cache().unwrap().guild(guild_id) {
        let owner = guild.owner_id;

        check_subscription(&cache_http, owner).await
    } else {
        false
    }
}
