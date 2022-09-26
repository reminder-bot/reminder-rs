use super::super::autocomplete::macro_name_autocomplete;
use crate::{models::command_macro::guild_command_macro, Context, Data, Error, THEME_COLOR};

/// Run a recorded macro
#[poise::command(
    slash_command,
    rename = "run",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "run_macro"
)]
pub async fn run_macro(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Name of macro to run"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    match guild_command_macro(&Context::Application(ctx), &name).await {
        Some(command_macro) => {
            Context::Application(ctx)
                .send(|b| {
                    b.embed(|e| {
                        e.title("Running Macro").color(*THEME_COLOR).description(format!(
                            "Running macro {} ({} commands)",
                            command_macro.name,
                            command_macro.commands.len()
                        ))
                    })
                })
                .await?;

            for command in command_macro.commands {
                if let Some(action) = command.action {
                    match (action)(poise::ApplicationContext { args: &command.options, ..ctx })
                        .await
                    {
                        Ok(()) => {}
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                } else {
                    Context::Application(ctx)
                        .say(format!("Command \"{}\" not found", command.command_name))
                        .await?;
                }
            }
        }

        None => {
            Context::Application(ctx).say(format!("Macro \"{}\" not found", name)).await?;
        }
    }

    Ok(())
}
