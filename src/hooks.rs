use log::warn;
use regex_command_attr::check;
use serenity::{client::Context, model::channel::Channel};

use crate::{
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse, HookResult},
    moderation_cmds, RecordingMacros,
};

#[check]
pub async fn macro_check(
    ctx: &Context,
    invoke: &mut CommandInvoke,
    args: &CommandOptions,
) -> HookResult {
    if let Some(guild_id) = invoke.guild_id() {
        if args.command != moderation_cmds::MACRO_CMD_COMMAND.names[0] {
            let active_recordings =
                ctx.data.read().await.get::<RecordingMacros>().cloned().unwrap();
            let mut lock = active_recordings.write().await;

            if let Some(command_macro) = lock.get_mut(&(guild_id, invoke.author_id())) {
                if command_macro.commands.len() >= 5 {
                    let _ = invoke
                        .respond(
                            &ctx,
                            CreateGenericResponse::new().content("5 commands already recorded. Please use `/macro finish` to end recording."),
                        )
                        .await;
                } else {
                    command_macro.commands.push(args.clone());

                    let _ = invoke
                        .respond(
                            &ctx,
                            CreateGenericResponse::new().content("Command recorded to macro"),
                        )
                        .await;
                }

                HookResult::Halt
            } else {
                HookResult::Continue
            }
        } else {
            HookResult::Continue
        }
    } else {
        HookResult::Continue
    }
}

#[check]
pub async fn check_self_permissions(
    ctx: &Context,
    invoke: &mut CommandInvoke,
    _args: &CommandOptions,
) -> HookResult {
    if let Some(guild) = invoke.guild(&ctx) {
        let user_id = ctx.cache.current_user_id();

        let manage_webhooks =
            guild.member_permissions(&ctx, user_id).await.map_or(false, |p| p.manage_webhooks());
        let (send_messages, embed_links) = invoke
            .channel_id()
            .to_channel_cached(&ctx)
            .map(|c| {
                if let Channel::Guild(channel) = c {
                    channel.permissions_for_user(ctx, user_id).ok()
                } else {
                    None
                }
            })
            .flatten()
            .map_or((false, false), |p| (p.send_messages(), p.embed_links()));

        if manage_webhooks && send_messages && embed_links {
            HookResult::Continue
        } else {
            if send_messages {
                let _ = invoke
                    .respond(
                        &ctx,
                        CreateGenericResponse::new().content(format!(
                            "Please ensure the bot has the correct permissions:

✅     **Send Message**
{}     **Embed Links**
{}     **Manage Webhooks**",
                            if manage_webhooks { "✅" } else { "❌" },
                            if embed_links { "✅" } else { "❌" },
                        )),
                    )
                    .await;
            } else {
                warn!("Missing permissions in guild {}", guild.id);
            }

            HookResult::Halt
        }
    } else {
        HookResult::Continue
    }
}

#[check]
pub async fn check_guild_permissions(
    ctx: &Context,
    invoke: &mut CommandInvoke,
    _args: &CommandOptions,
) -> HookResult {
    if let Some(guild) = invoke.guild(&ctx) {
        let permissions = guild.member_permissions(&ctx, invoke.author_id()).await.unwrap();

        if !permissions.manage_guild() {
            let _ = invoke
                .respond(
                    &ctx,
                    CreateGenericResponse::new().content(
                        "You must have the \"Manage Server\" permission to use this command",
                    ),
                )
                .await;

            HookResult::Halt
        } else {
            HookResult::Continue
        }
    } else {
        HookResult::Continue
    }
}
