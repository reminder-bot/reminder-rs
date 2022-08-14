use poise::serenity::model::{
    application::interaction::application_command::CommandDataOption, id::GuildId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Context, Data, Error};

type Func<U, E> = for<'a> fn(
    poise::ApplicationContext<'a, U, E>,
) -> poise::BoxFuture<'a, Result<(), poise::FrameworkError<'a, U, E>>>;

fn default_none<U, E>() -> Option<Func<U, E>> {
    None
}

#[derive(Serialize, Deserialize)]
pub struct RecordedCommand<U, E> {
    #[serde(skip)]
    #[serde(default = "default_none::<U, E>")]
    pub action: Option<Func<U, E>>,
    pub command_name: String,
    pub options: Vec<CommandDataOption>,
}

pub struct CommandMacro<U, E> {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Vec<RecordedCommand<U, E>>,
}

pub struct RawCommandMacro {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Value,
}

pub async fn guild_command_macro(
    ctx: &Context<'_>,
    name: &str,
) -> Option<CommandMacro<Data, Error>> {
    let row = sqlx::query!(
        "
SELECT * FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND name = ?
        ",
        ctx.guild_id().unwrap().0,
        name
    )
    .fetch_one(&ctx.data().database)
    .await
    .ok()?;

    let mut commands: Vec<RecordedCommand<Data, Error>> =
        serde_json::from_str(&row.commands).unwrap();

    for recorded_command in &mut commands {
        let command = &ctx
            .framework()
            .options()
            .commands
            .iter()
            .find(|c| c.identifying_name == recorded_command.command_name);

        recorded_command.action = command.map(|c| c.slash_action).flatten();
    }

    let command_macro = CommandMacro {
        guild_id: ctx.guild_id().unwrap(),
        name: row.name,
        description: row.description,
        commands,
    };

    Some(command_macro)
}
