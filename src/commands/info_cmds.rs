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

use crate::THEME_COLOR;


#[command]
#[can_blacklist(false)]
async fn help(ctx: &Context, msg: &Message, _args: String) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| m
        .embed(|e| e
            .title("Help")
            .description("Help Description")
            .color(THEME_COLOR)
        )
    ).await?;

    Ok(())
}

#[command]
async fn info(ctx: &Context, msg: &Message, _args: String) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| m
        .embed(|e| e
            .title("Info")
            .description("Info Description")
            .color(THEME_COLOR)
        )
    ).await?;

    Ok(())
}

#[command]
async fn donate(ctx: &Context, msg: &Message, _args: String) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| m
        .embed(|e| e
            .title("Donate")
            .description("Donate Description")
            .color(THEME_COLOR)
        )
    ).await?;

    Ok(())
}
