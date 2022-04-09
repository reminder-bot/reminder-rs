#[macro_use]
extern crate rocket;

mod consts;
#[macro_use]
mod macros;
mod routes;

use std::{collections::HashMap, env};

use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use rocket::{
    fs::FileServer,
    serde::json::{json, Value as JsonValue},
    tokio::sync::broadcast::Sender,
};
use rocket_dyn_templates::Template;
use serenity::{
    client::Context,
    http::CacheHttp,
    model::id::{GuildId, UserId},
};
use sqlx::{MySql, Pool};

use crate::consts::{CNC_GUILD, DISCORD_OAUTH_AUTHORIZE, DISCORD_OAUTH_TOKEN, SUBSCRIPTION_ROLES};

type Database = MySql;

#[derive(Debug)]
enum Error {
    SQLx(sqlx::Error),
    Serenity(serenity::Error),
}

#[catch(401)]
async fn not_authorized() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("errors/401", &map)
}

#[catch(403)]
async fn forbidden() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("errors/403", &map)
}

#[catch(404)]
async fn not_found() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("errors/404", &map)
}

#[catch(413)]
async fn payload_too_large() -> JsonValue {
    json!({"error": "Data too large.", "errors": ["Data too large."]})
}

#[catch(422)]
async fn unprocessable_entity() -> JsonValue {
    json!({"error": "Invalid request.", "errors": ["Invalid request."]})
}

#[catch(500)]
async fn internal_server_error() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("errors/500", &map)
}

pub async fn initialize(
    kill_channel: Sender<()>,
    serenity_context: Context,
    db_pool: Pool<Database>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking environment variables...");
    env::var("OAUTH2_CLIENT_ID").expect("`OAUTH2_CLIENT_ID' not supplied");
    env::var("OAUTH2_CLIENT_SECRET").expect("`OAUTH2_CLIENT_SECRET' not supplied");
    env::var("OAUTH2_DISCORD_CALLBACK").expect("`OAUTH2_DISCORD_CALLBACK' not supplied");
    env::var("PATREON_GUILD_ID").expect("`PATREON_GUILD' not supplied");
    info!("Done!");

    let oauth2_client = BasicClient::new(
        ClientId::new(env::var("OAUTH2_CLIENT_ID")?),
        Some(ClientSecret::new(env::var("OAUTH2_CLIENT_SECRET")?)),
        AuthUrl::new(DISCORD_OAUTH_AUTHORIZE.to_string())?,
        Some(TokenUrl::new(DISCORD_OAUTH_TOKEN.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(env::var("OAUTH2_DISCORD_CALLBACK")?)?);

    let reqwest_client = reqwest::Client::new();

    rocket::build()
        .attach(Template::fairing())
        .register(
            "/",
            catchers![
                not_authorized,
                forbidden,
                not_found,
                internal_server_error,
                unprocessable_entity,
                payload_too_large,
            ],
        )
        .manage(oauth2_client)
        .manage(reqwest_client)
        .manage(serenity_context)
        .manage(db_pool)
        .mount("/static", FileServer::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
        .mount(
            "/",
            routes![
                routes::index,
                routes::cookies,
                routes::privacy,
                routes::terms,
                routes::return_to_same_site
            ],
        )
        .mount(
            "/help",
            routes![
                routes::help,
                routes::help_timezone,
                routes::help_create_reminder,
                routes::help_delete_reminder,
                routes::help_timers,
                routes::help_todo_lists,
                routes::help_macros,
            ],
        )
        .mount("/login", routes![routes::login::discord_login, routes::login::discord_callback])
        .mount(
            "/dashboard",
            routes![
                routes::dashboard::dashboard,
                routes::dashboard::dashboard_home,
                routes::dashboard::user::get_user_info,
                routes::dashboard::user::update_user_info,
                routes::dashboard::user::get_user_guilds,
                routes::dashboard::guild::get_guild_channels,
                routes::dashboard::guild::get_guild_roles,
                routes::dashboard::guild::get_reminder_templates,
                routes::dashboard::guild::create_reminder_template,
                routes::dashboard::guild::delete_reminder_template,
                routes::dashboard::guild::create_reminder,
                routes::dashboard::guild::get_reminders,
                routes::dashboard::guild::edit_reminder,
                routes::dashboard::guild::delete_reminder,
            ],
        )
        .launch()
        .await?;

    warn!("Exiting rocket runtime");
    // distribute kill signal
    match kill_channel.send(()) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to issue kill signal: {:?}", e);
        }
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
