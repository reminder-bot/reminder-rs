use serenity::{
    http::CacheHttp,
    model::id::{GuildId, UserId},
};

use crate::consts::{CNC_GUILD, SUBSCRIPTION_ROLES};

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
