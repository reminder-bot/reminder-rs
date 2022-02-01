use poise::{serenity::model::channel::Channel, ApplicationCommandOrAutocompleteInteraction};

use crate::{consts::MACRO_MAX_COMMANDS, models::command_macro::CommandOptions, Context, Error};

pub async fn guild_only(ctx: Context<'_>) -> Result<bool, Error> {
    if ctx.guild_id().is_some() {
        Ok(true)
    } else {
        let _ = ctx.say("This command can only be used in servers").await;

        Ok(false)
    }
}

async fn macro_check(ctx: Context<'_>) -> bool {
    if let Context::Application(app_ctx) = ctx {
        if let ApplicationCommandOrAutocompleteInteraction::ApplicationCommand(interaction) =
            app_ctx.interaction
        {
            if let Some(guild_id) = ctx.guild_id() {
                if ctx.command().identifying_name != "macro_finish" {
                    let mut lock = ctx.data().recording_macros.write().await;

                    if let Some(command_macro) = lock.get_mut(&(guild_id, ctx.author().id)) {
                        if command_macro.commands.len() >= MACRO_MAX_COMMANDS {
                            let _ = ctx.send(|m| {
                                m.ephemeral(false).content(
                                    "5 commands already recorded. Please use `/macro finish` to end recording.",
                                )
                            })
                            .await;
                        } else {
                            let mut command_options = CommandOptions::new(&ctx.command().name);
                            command_options.populate(&interaction);

                            command_macro.commands.push(command_options);

                            let _ = ctx
                                .send(|m| m.ephemeral(false).content("Command recorded to macro"))
                                .await;
                        }

                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    }
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
            .to_channel_cached(&ctx.discord())
            .map(|c| {
                if let Channel::Guild(channel) = c {
                    channel.permissions_for_user(&ctx.discord(), user_id).ok()
                } else {
                    None
                }
            })
            .flatten()
            .map_or((false, false, false), |p| {
                (p.read_messages(), p.send_messages(), p.embed_links())
            });

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
                        if manage_webhooks { "✅" } else { "❌" },
                        if embed_links { "✅" } else { "❌" },
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
