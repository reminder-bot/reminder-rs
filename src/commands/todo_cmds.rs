use regex_command_attr::command;

use serenity::{
    client::Context,
    model::{
        id::{
            UserId, GuildId, ChannelId,
        },
        channel::{
            Message,
        },
    },
    framework::standard::CommandResult,
};

enum TodoTarget {
    User(UserId),
    Channel(ChannelId),
    Guild(GuildId),
}

enum SubCommand {
    View,
    Add,
    Remove,
    Clear,
}

#[command]
async fn todo_parse(ctx: &Context, msg: &Message, args: String) -> CommandResult {

    let mut split = args.split(" ");

    if let Some(target) = split.next() {
        target_opt = match target {
            "user" =>
                TodoTarget::User(msg.author.id),

            "channel" =>
                TodoTarget::Channel(msg.channel_id),

            "server" | "guild" => {
                if let Some(gid) = msg.guild_id {
                    TodoTarget::Guild(gid)
                }
                else {
                    None
                }
            },

            _ => {
                 None
            },
        };

        if let Some(target) = target_opt {

            let subcommand_opt = match split.next() {

                Some("add") => Some(SubCommand::Add),

                Some("remove") => Some(SubCommand::Remove),

                Some("clear") => Some(SubCommand::Clear),

                None => Some(SubCommand::View),

                Some(_unrecognised) => None,
            };

            if let Some(subcommand) = subcommand_opt {
                todo(ctx, target, subcommand).await;
            }
            else {
                let _ = msg.channel_id.say(&ctx, "Todo help").await;
            }

        }
        else {
            let _ = msg.channel_id.say(&ctx, "Todo help").await;
        }

    }
    else {
        let _ = msg.channel_id.say(&ctx, "Todo help").await;
    }

    Ok(())
}

async fn todo(ctx: &Context, target: TodoTarget, subcommand: SubCommand) {

}
