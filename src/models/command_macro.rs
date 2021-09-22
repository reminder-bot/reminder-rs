use serenity::model::id::GuildId;

use crate::framework::CommandOptions;

pub struct CommandMacro {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Vec<CommandOptions>,
}
