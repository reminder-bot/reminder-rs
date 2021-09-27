use regex_command_attr::command;
use serenity::client::Context;

use crate::{
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse},
    SQLPool,
};

#[command]
#[description("Manage todo lists")]
#[subcommandgroup("server")]
#[description("Manage the server todo list")]
#[subcommand("add")]
#[description("Add an item to the server todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from the server todo list")]
#[subcommandgroup("channel")]
#[description("Manage the channel todo list")]
#[subcommand("add")]
#[description("Add to the channel todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from the channel todo list")]
#[subcommandgroup("user")]
#[description("Manage your personal todo list")]
#[subcommand("add")]
#[description("Add to your personal todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from your personal todo list")]
async fn todo(ctx: &Context, invoke: CommandInvoke, args: CommandOptions) {
    if invoke.guild_id().is_none() && args.subcommand_group != Some("user".to_string()) {
        let _ = invoke
            .respond(
                &ctx,
                CreateGenericResponse::new().content("Please use `/todo user` in direct messages"),
            )
            .await;
    } else {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

        let keys = match args.subcommand_group.as_ref().unwrap().as_str() {
            "server" => (None, None, invoke.guild_id().map(|g| g.0)),
            "channel" => (None, Some(invoke.channel_id().0), invoke.guild_id().map(|g| g.0)),
            _ => (Some(invoke.author_id().0), None, None),
        };

        match args.get("task") {
            Some(task) => {
                let task = task.to_string();

                sqlx::query!(
                    "INSERT INTO todos (user_id, channel_id, guild_id, value) VALUES (?, ?, ?, ?)",
                    keys.0,
                    keys.1,
                    keys.2,
                    task
                )
                .execute(&pool)
                .await
                .unwrap();

                let _ = invoke
                    .respond(&ctx, CreateGenericResponse::new().content("Item added to todo list"))
                    .await;
            }
            None => {
                let values = sqlx::query!(
                    "SELECT value FROM todos WHERE user_id = ? AND channel_id = ? AND guild_id = ?",
                    keys.0,
                    keys.1,
                    keys.2,
                )
                .fetch_all(&pool)
                .await
                .unwrap()
                .iter()
                .map(|row| &row.value);
            }
        }
    }
}
