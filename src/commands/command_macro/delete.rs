use super::super::autocomplete::macro_name_autocomplete;
use crate::{Context, Error};

/// Delete a recorded macro
#[poise::command(
    slash_command,
    rename = "delete",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "delete_macro"
)]
pub async fn delete_macro(
    ctx: Context<'_>,
    #[description = "Name of macro to delete"]
    #[autocomplete = "macro_name_autocomplete"]
    name: String,
) -> Result<(), Error> {
    match sqlx::query!(
        "
SELECT id FROM macro WHERE guild_id = ? AND name = ?",
        ctx.guild_id().unwrap().0,
        name
    )
    .fetch_one(&ctx.data().database)
    .await
    {
        Ok(row) => {
            sqlx::query!("DELETE FROM macro WHERE id = ?", row.id)
                .execute(&ctx.data().database)
                .await
                .unwrap();

            ctx.say(format!("Macro \"{}\" deleted", name)).await?;
        }

        Err(sqlx::Error::RowNotFound) => {
            ctx.say(format!("Macro \"{}\" not found", name)).await?;
        }

        Err(e) => {
            panic!("{}", e);
        }
    }

    Ok(())
}
