use poise::serenity::{
    client::Context,
    model::{
        id::GuildId, interactions::application_command::ApplicationCommandInteractionDataOption,
    },
};
use serde::Serialize;

#[derive(Serialize)]
pub struct RecordedCommand<U, E> {
    #[serde(skip)]
    action: for<'a> fn(
        poise::ApplicationContext<'a, U, E>,
        &'a [ApplicationCommandInteractionDataOption],
    ) -> poise::BoxFuture<'a, Result<(), poise::FrameworkError<'a, U, E>>>,
    command_name: String,
    options: Vec<ApplicationCommandInteractionDataOption>,
}

pub struct CommandMacro<U, E> {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Vec<RecordedCommand<U, E>>,
}
