#![feature(int_roundings)]
#[macro_use]
extern crate lazy_static;

mod commands;
// mod component_models;
mod consts;
mod event_handlers;
mod hooks;
mod models;
mod time_parser;
mod utils;

use std::{collections::HashMap, env};

use chrono_tz::Tz;
use dotenv::dotenv;
use poise::serenity::model::{
    gateway::{Activity, GatewayIntents},
    id::{GuildId, UserId},
};
use sqlx::{MySql, Pool};
use tokio::sync::RwLock;

use crate::{
    commands::{info_cmds, moderation_cmds},
    consts::THEME_COLOR,
    event_handlers::listener,
    hooks::all_checks,
    models::command_macro::CommandMacro,
    utils::register_application_commands,
};

type Database = MySql;

pub struct Data {
    database: Pool<Database>,
    http: reqwest::Client,
    recording_macros: RwLock<HashMap<(GuildId, UserId), CommandMacro>>,
    popular_timezones: Vec<Tz>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    dotenv()?;

    let discord_token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment");

    let options = poise::FrameworkOptions {
        commands: vec![
            info_cmds::help(),
            info_cmds::info(),
            info_cmds::donate(),
            info_cmds::clock(),
            info_cmds::dashboard(),
            moderation_cmds::timezone(),
            poise::Command {
                subcommands: vec![
                    moderation_cmds::delete_macro(),
                    moderation_cmds::finish_macro(),
                    moderation_cmds::list_macro(),
                    moderation_cmds::record_macro(),
                    moderation_cmds::run_macro(),
                ],
                ..moderation_cmds::macro_base()
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
        "
SELECT timezone FROM users GROUP BY timezone ORDER BY COUNT(timezone) DESC LIMIT 21"
    )
    .fetch_all(&database)
    .await
    .unwrap()
    .iter()
    .map(|t| t.timezone.parse::<Tz>().unwrap())
    .collect::<Vec<Tz>>();

    poise::Framework::build()
        .token(discord_token)
        .user_data_setup(move |ctx, _bot, framework| {
            Box::pin(async move {
                ctx.set_activity(Activity::watching("for /remind")).await;

                register_application_commands(
                    ctx,
                    framework,
                    env::var("DEBUG_GUILD")
                        .map(|inner| GuildId(inner.parse().expect("DEBUG_GUILD not valid")))
                        .ok(),
                )
                .await
                .unwrap();

                Ok(Data {
                    http: reqwest::Client::new(),
                    database,
                    popular_timezones,
                    recording_macros: Default::default(),
                })
            })
        })
        .options(options)
        .client_settings(move |client_builder| client_builder.intents(GatewayIntents::GUILDS))
        .run_autosharded()
        .await?;

    Ok(())
}
