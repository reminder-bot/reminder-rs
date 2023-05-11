use poise::{
    serenity_prelude::model::channel::Channel, ApplicationCommandOrAutocompleteInteraction,
};

use crate::{consts::MACRO_MAX_COMMANDS, models::command_macro::RecordedCommand, Context, Error};

async fn macro_check(ctx: Context<'_>) -> bool {
    if let Context::Application(app_ctx) = ctx {
        if let ApplicationCommandOrAutocompleteInteraction::ApplicationCommand(_) =
            app_ctx.interaction
        {
            if let Some(guild_id) = ctx.guild_id() {
                if ctx.command().identifying_name != "finish_macro" {
                    let mut lock = ctx.data().recording_macros.write().await;

                    if let Some(command_macro) = lock.get_mut(&(guild_id, ctx.author().id)) {
                        if command_macro.commands.len() >= MACRO_MAX_COMMANDS {
                            let _ = ctx.send(|m| {
                            m.ephemeral(true).content(
                                format!("{} commands already recorded. Please use `/macro finish` to end recording.", MACRO_MAX_COMMANDS),
                            )
                        })
                            .await;
                        } else {
                            let recorded = RecordedCommand {
                                action: None,
                                command_name: ctx.command().identifying_name.clone(),
                                options: Vec::from(app_ctx.args),
                            };

                            command_macro.commands.push(recorded);

                            let _ = ctx
                                .send(|m| m.ephemeral(true).content("Command recorded to macro"))
                                .await;
                        }

                        return false;
                    }
                }
            }
        }
    }

    true
}

async fn check_self_permissions(ctx: Context<'_>) -> bool {
    if let Some(guild) = ctx.guild() {
        let user_id = ctx.discord().cache.current_user_id();

        let manage_webhooks = guild
            .member_permissions(&ctx.discord(), user_id)
            .await
            .map_or(false, |p| p.manage_webhooks());

        let (view_channel, send_messages, embed_links) = ctx
            .channel_id()
            .to_channel(&ctx.discord())
            .await
            .ok()
            .and_then(|c| {
                if let Channel::Guild(channel) = c {
                    let perms = channel.permissions_for_user(&ctx.discord(), user_id).ok()?;

                    Some((perms.view_channel(), perms.send_messages(), perms.embed_links()))
                } else {
                    None
                }
            })
            .unwrap_or((false, false, false));

        if manage_webhooks && send_messages && embed_links {
            true
        } else {
            let _ = ctx
                .send(|m| {
                    m.content(format!(
                        "Please ensure the bot has the correct permissions:

{}     **View Channel**
{}     **Send Message**
{}     **Embed Links**
{}     **Manage Webhooks**",
                        if view_channel { "✅" } else { "❌" },
                        if send_messages { "✅" } else { "❌" },
                        if embed_links { "✅" } else { "❌" },
                        if manage_webhooks { "✅" } else { "❌" },
                    ))
                })
                .await;

            false
        }
    } else {
        true
    }
}

pub async fn all_checks(ctx: Context<'_>) -> Result<bool, Error> {
    Ok(macro_check(ctx).await && check_self_permissions(ctx).await)
}
