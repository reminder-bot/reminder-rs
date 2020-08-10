use regex_command_attr::command;

use serenity::{
    client::Context,
    model::{
        channel::{
            Message,
        },
    },
    framework::standard::CommandResult,
};

enum TodoTarget {
    User,
    Channel,
    Guild,
}

impl TodoTarget {
    fn from_str(string: &str) -> Option<Self> {
        match string {
            "user" => Some(Self::User),

            "channel" => Some(Self::Channel),

            "server" | "guild" => Some(Self::Guild),

            _ => None
        }
    }
}

enum SubCommand {
    View,
    Add,
    Remove,
    Clear,
}

#[command]
async fn todo_parse(ctx: &Context, msg: &Message, args: String) -> CommandResult {
    Ok(())
}

async fn todo(ctx: &Context, target: TodoTarget, subcommand: SubCommand) {

}
