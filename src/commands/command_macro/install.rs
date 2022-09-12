use poise::serenity_prelude::CommandType;

use crate::{
    commands::autocomplete::macro_name_autocomplete, models::command_macro::guild_command_macro,
    Context, Error,
};

/// Add a macro as a slash-command to this server. Enables controlling permissions per-macro.
#[poise::command(
    slash_command,
    rename = "install",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "install_macro"
)]
pub async fn install_macro(
    ctx: Context<'_>,
    #[description = "Name of macro to install"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    if let Some(command_macro) = guild_command_macro(&ctx, &name).await {
        guild_id
            .create_application_command(&ctx.discord(), |a| {
                a.kind(CommandType::ChatInput)
                    .name(command_macro.name)
                    .description(command_macro.description.unwrap_or_else(|| "".to_string()))
            })
            .await?;
        ctx.send(|r| r.ephemeral(true).content("Macro installed. Go to Server Settings 🠚 Integrations 🠚 Reminder Bot to configure permissions.")).await?;
    } else {
        ctx.send(|r| r.ephemeral(true).content("No macro found with that name")).await?;
    }

    Ok(())
}
