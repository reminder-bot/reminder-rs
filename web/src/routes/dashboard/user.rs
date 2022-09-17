use std::env;

use chrono_tz::Tz;
use reqwest::Client;
use rocket::{
    http::CookieJar,
    serde::json::{json, Json, Value as JsonValue},
    State,
};
use serde::{Deserialize, Serialize};
use serenity::{
    client::Context,
    model::{
        id::{GuildId, RoleId},
        permissions::Permissions,
    },
};
use sqlx::{MySql, Pool};

use crate::consts::DISCORD_API;

#[derive(Serialize)]
struct UserInfo {
    name: String,
    patreon: bool,
    timezone: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    timezone: String,
}

#[derive(Serialize)]
struct GuildInfo {
    id: String,
    name: String,
}

#[derive(Deserialize)]
pub struct PartialGuild {
    pub id: GuildId,
    pub icon: Option<String>,
    pub name: String,
    #[serde(default)]
    pub owner: bool,
    #[serde(rename = "permissions_new")]
    pub permissions: Option<String>,
}

#[get("/api/user")]
pub async fn get_user_info(
    cookies: &CookieJar<'_>,
    ctx: &State<Context>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    if let Some(user_id) =
        cookies.get_private("userid").map(|u| u.value().parse::<u64>().ok()).flatten()
    {
        let member_res = GuildId(env::var("PATREON_GUILD_ID").unwrap().parse().unwrap())
            .member(&ctx.inner(), user_id)
            .await;

        let timezone = sqlx::query!(
            "SELECT IFNULL(timezone, 'UTC') AS timezone FROM users WHERE user = ?",
            user_id
        )
        .fetch_one(pool.inner())
        .await
        .map_or(None, |q| Some(q.timezone));

        let user_info = UserInfo {
            name: cookies
                .get_private("username")
                .map_or("DiscordUser#0000".to_string(), |c| c.value().to_string()),
            patreon: member_res.map_or(false, |member| {
                member
                    .roles
                    .contains(&RoleId(env::var("PATREON_ROLE_ID").unwrap().parse().unwrap()))
            }),
            timezone,
        };

        json!(user_info)
    } else {
        json!({"error": "Not authorized"})
    }
}

#[patch("/api/user", data = "<user>")]
pub async fn update_user_info(
    cookies: &CookieJar<'_>,
    user: Json<UpdateUser>,
    pool: &State<Pool<MySql>>,
) -> JsonValue {
    if let Some(user_id) =
        cookies.get_private("userid").map(|u| u.value().parse::<u64>().ok()).flatten()
    {
        if user.timezone.parse::<Tz>().is_ok() {
            let _ = sqlx::query!(
                "UPDATE users SET timezone = ? WHERE user = ?",
                user.timezone,
                user_id,
            )
            .execute(pool.inner())
            .await;

            json!({})
        } else {
            json!({"error": "Timezone not recognized"})
        }
    } else {
        json!({"error": "Not authorized"})
    }
}

#[get("/api/user/guilds")]
pub async fn get_user_guilds(cookies: &CookieJar<'_>, reqwest_client: &State<Client>) -> JsonValue {
    if let Some(access_token) = cookies.get_private("access_token") {
        let request_res = reqwest_client
            .get(format!("{}/users/@me/guilds", DISCORD_API))
            .bearer_auth(access_token.value())
            .send()
            .await;

        match request_res {
            Ok(response) => {
                let guilds_res = response.json::<Vec<PartialGuild>>().await;

                match guilds_res {
                    Ok(guilds) => {
                        let reduced_guilds = guilds
                            .iter()
                            .filter(|g| {
                                g.owner
                                    || g.permissions.as_ref().map_or(false, |p| {
                                        let permissions =
                                            Permissions::from_bits_truncate(p.parse().unwrap());

                                        permissions.manage_messages()
                                            || permissions.manage_guild()
                                            || permissions.administrator()
                                    })
                            })
                            .map(|g| GuildInfo { id: g.id.to_string(), name: g.name.to_string() })
                            .collect::<Vec<GuildInfo>>();

                        json!(reduced_guilds)
                    }

                    Err(e) => {
                        warn!("Error constructing user from request: {:?}", e);

                        json!({"error": "Could not get user details"})
                    }
                }
            }

            Err(e) => {
                warn!("Error getting user guilds: {:?}", e);

                json!({"error": "Could not reach Discord"})
            }
        }
    } else {
        json!({"error": "Not authorized"})
    }
}
