use serenity::{client::Context, model::id::GuildId};

use crate::{framework::CommandOptions, SQLPool};

pub struct CommandMacro {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Vec<CommandOptions>,
}

impl CommandMacro {
    pub async fn from_guild(ctx: &Context, guild_id: impl Into<GuildId>) -> Vec<Self> {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
        let guild_id = guild_id.into();

        sqlx::query!(
            "SELECT * FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
            guild_id.0
        )
        .fetch_all(&pool)
        .await
        .unwrap()
        .iter()
        .map(|row| Self {
            guild_id,
            name: row.name.clone(),
            description: row.description.clone(),
            commands: serde_json::from_str(&row.commands).unwrap(),
        })
        .collect::<Vec<Self>>()
    }
}
