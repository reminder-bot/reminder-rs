#![feature(int_roundings)]

#[macro_use]
extern crate lazy_static;

mod commands;
mod component_models;
mod consts;
mod event_handlers;
mod hooks;
mod interval_parser;
mod models;
mod time_parser;
mod utils;

use std::{
    collections::HashMap,
    env,
    error::Error as StdError,
    fmt::{Debug, Display, Formatter},
};

use chrono_tz::Tz;
use dotenv::dotenv;
use log::{error, warn};
use poise::serenity_prelude::model::{
    gateway::GatewayIntents,
    id::{GuildId, UserId},
};
use sqlx::{MySql, Pool};
use tokio::sync::{broadcast, broadcast::Sender, RwLock};

use crate::{
    commands::{info_cmds, moderation_cmds, reminder_cmds, todo_cmds},
    consts::THEME_COLOR,
    event_handlers::listener,
    hooks::all_checks,
    models::command_macro::CommandMacro,
    utils::register_application_commands,
};

type Database = MySql;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;

pub struct Data {
    database: Pool<Database>,
    http: reqwest::Client,
    recording_macros: RwLock<HashMap<(GuildId, UserId), CommandMacro<Data, Error>>>,
    popular_timezones: Vec<Tz>,
    _broadcast: Sender<()>,
}

impl Debug for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{ .. }}")
    }
}

struct Ended;

impl Debug for Ended {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Process ended.")
    }
}

impl Display for Ended {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Process ended.")
    }
}

impl StdError for Ended {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (tx, mut rx) = broadcast::channel(16);

    tokio::select! {
        output = _main(tx) => output,
        _ = rx.recv() => Err(Box::new(Ended) as Box<dyn StdError + Send + Sync>)
    }
}

async fn _main(tx: Sender<()>) -> Result<(), Box<dyn StdError + Send + Sync>> {
    env_logger::init();

    dotenv()?;

    let discord_token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment");

    let options = poise::FrameworkOptions {
        commands: vec![
            info_cmds::help(),
            info_cmds::info(),
            info_cmds::donate(),
            info_cmds::clock(),
            info_cmds::clock_context_menu(),
            info_cmds::dashboard(),
            moderation_cmds::timezone(),
            poise::Command {
                subcommands: vec![
                    moderation_cmds::set_allowed_dm(),
                    moderation_cmds::unset_allowed_dm(),
                ],
                ..moderation_cmds::allowed_dm()
            },
            moderation_cmds::webhook(),
            poise::Command {
                subcommands: vec![
                    moderation_cmds::delete_macro(),
                    moderation_cmds::finish_macro(),
                    moderation_cmds::list_macro(),
                    moderation_cmds::record_macro(),
                    moderation_cmds::run_macro(),
                    moderation_cmds::migrate_macro(),
                ],
                ..moderation_cmds::macro_base()
            },
            reminder_cmds::pause(),
            reminder_cmds::offset(),
            reminder_cmds::nudge(),
            reminder_cmds::look(),
            reminder_cmds::delete(),
            poise::Command {
                subcommands: vec![
                    reminder_cmds::list_timer(),
                    reminder_cmds::start_timer(),
                    reminder_cmds::delete_timer(),
                ],
                ..reminder_cmds::timer_base()
            },
            reminder_cmds::remind(),
            poise::Command {
                subcommands: vec![
                    poise::Command {
                        subcommands: vec![
                            todo_cmds::todo_guild_add(),
                            todo_cmds::todo_guild_view(),
                        ],
                        ..todo_cmds::todo_guild_base()
                    },
                    poise::Command {
                        subcommands: vec![
                            todo_cmds::todo_channel_add(),
                            todo_cmds::todo_channel_view(),
                        ],
                        ..todo_cmds::todo_channel_base()
                    },
                    poise::Command {
                        subcommands: vec![todo_cmds::todo_user_add(), todo_cmds::todo_user_view()],
                        ..todo_cmds::todo_user_base()
                    },
                ],
                ..todo_cmds::todo_base()
            },
        ],
        allowed_mentions: None,
        command_check: Some(|ctx| Box::pin(all_checks(ctx))),
        listener: |ctx, event, _framework, data| Box::pin(listener(ctx, event, data)),
        ..Default::default()
    };

    let database =
        Pool::connect(&env::var("DATABASE_URL").expect("No database URL provided")).await.unwrap();

    let popular_timezones = sqlx::query!(
        "SELECT timezone FROM users GROUP BY timezone ORDER BY COUNT(timezone) DESC LIMIT 21"
    )
    .fetch_all(&database)
    .await
    .unwrap()
    .iter()
    .map(|t| t.timezone.parse::<Tz>().unwrap())
    .collect::<Vec<Tz>>();

    poise::Framework::builder()
        .token(discord_token)
        .user_data_setup(move |ctx, _bot, framework| {
            Box::pin(async move {
                register_application_commands(ctx, framework, None).await.unwrap();

                let kill_tx = tx.clone();
                let kill_recv = tx.subscribe();

                let ctx1 = ctx.clone();
                let ctx2 = ctx.clone();

                let pool1 = database.clone();
                let pool2 = database.clone();

                let run_settings = env::var("DONTRUN").unwrap_or_else(|_| "".to_string());

                if !run_settings.contains("postman") {
                    tokio::spawn(async move {
                        match postman::initialize(kill_recv, ctx1, &pool1).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("postman exiting: {}", e);
                            }
                        };
                    });
                } else {
                    warn!("Not running postman");
                }

                if !run_settings.contains("web") {
                    tokio::spawn(async move {
                        reminder_web::initialize(kill_tx, ctx2, pool2).await.unwrap();
                    });
                } else {
                    warn!("Not running web");
                }

                Ok(Data {
                    http: reqwest::Client::new(),
                    database,
                    popular_timezones,
                    recording_macros: Default::default(),
                    _broadcast: tx,
                })
            })
        })
        .options(options)
        .intents(GatewayIntents::GUILDS)
        .run_autosharded()
        .await?;

    Ok(())
}
