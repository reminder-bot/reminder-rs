use poise::serenity::{
    builder::CreateApplicationCommands,
    http::CacheHttp,
    model::id::{GuildId, UserId},
};

use crate::{
    consts::{CNC_GUILD, SUBSCRIPTION_ROLES},
    Data, Error,
};

pub async fn register_application_commands(
    ctx: &poise::serenity::client::Context,
    framework: &poise::Framework<Data, Error>,
    guild_id: Option<GuildId>,
) -> Result<(), poise::serenity::Error> {
    let mut commands_builder = CreateApplicationCommands::default();
    let commands = &framework.options().commands;
    for command in commands {
        if let Some(slash_command) = command.create_as_slash_command() {
            commands_builder.add_application_command(slash_command);
        }
        if let Some(context_menu_command) = command.create_as_context_menu_command() {
            commands_builder.add_application_command(context_menu_command);
        }
    }
    let commands_builder = poise::serenity::json::Value::Array(commands_builder.0);

    if let Some(guild_id) = guild_id {
        ctx.http.create_guild_application_commands(guild_id.0, &commands_builder).await?;
    } else {
        ctx.http.create_global_application_commands(&commands_builder).await?;
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
