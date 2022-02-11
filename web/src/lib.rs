#[macro_use]
extern crate rocket;

mod consts;
mod routes;

use rocket::fs::{relative, FileServer};
use std::collections::HashMap;

use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

use crate::consts::{DISCORD_OAUTH_AUTHORIZE, DISCORD_OAUTH_TOKEN};
use rocket_dyn_templates::Template;
use serenity::client::Context;
use sqlx::{MySql, Pool};
use std::env;

type Database = MySql;

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

#[catch(500)]
async fn internal_server_error() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("errors/500", &map)
}

pub async fn initialize(
    serenity_context: Context,
    db_pool: Pool<Database>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        .register("/", catchers![not_authorized, forbidden, not_found, internal_server_error])
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
                routes::help,
                routes::return_to_same_site
            ],
        )
        .mount("/login", routes![routes::login::discord_login, routes::login::discord_callback])
        .mount(
            "/dashboard",
            routes![
                routes::dashboard::dashboard_home,
                routes::dashboard::user::get_user_info,
                routes::dashboard::user::update_user_info,
                routes::dashboard::user::get_user_guilds,
                routes::dashboard::user::create_reminder,
                routes::dashboard::user::get_reminders,
                routes::dashboard::user::overwrite_reminder,
                routes::dashboard::user::delete_reminder,
                routes::dashboard::guild::get_guild_channels,
                routes::dashboard::guild::get_guild_roles,
                routes::dashboard::guild::create_reminder,
                routes::dashboard::guild::get_reminders,
                routes::dashboard::guild::edit_reminder,
                routes::dashboard::guild::delete_reminder,
            ],
        )
        .launch()
        .await?;

    Ok(())
}
