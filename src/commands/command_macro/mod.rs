use crate::{Context, Error};

pub mod delete;
pub mod list;
pub mod migrate;
pub mod record;
pub mod run;

/// Record and replay command sequences
#[poise::command(
    slash_command,
    rename = "macro",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "macro_base"
)]
pub async fn macro_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
