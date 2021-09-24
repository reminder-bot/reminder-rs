use regex_command_attr::command;
use serenity::client::Context;

use crate::framework::{CommandInvoke, CommandOptions};

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
async fn todo(ctx: &Context, invoke: CommandInvoke, args: CommandOptions) {}
