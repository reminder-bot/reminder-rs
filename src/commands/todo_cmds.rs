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

use crate::SQLPool;
use sqlx::MySqlPool;


#[derive(Debug)]
struct Todo {
    id: u32,
    user_id: Option<u32>,
    guild_id: Option<u32>,
    channel_id: Option<u32>,
    value: String,
}

struct TodoTarget {
    user: UserId,
    guild: Option<GuildId>,
    channel: Option<ChannelId>,
}

impl TodoTarget {
    pub async fn view(&self, pool: MySqlPool) -> Result<Vec<Todo>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(if let Some(cid) = self.channel {
            sqlx::query_as!(Todo,
                "
SELECT * FROM todos WHERE channel_id = (SELECT id FROM channels WHERE channel = ?)
                ", cid.as_u64())
                .fetch_all(&pool)
                .await?
        }
        else if let Some(gid) = self.guild {
            sqlx::query_as!(Todo,
                "
SELECT * FROM todos WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND channel_id IS NULL
                ", gid.as_u64())
                .fetch_all(&pool)
                .await?
        }
        else {
            sqlx::query_as!(Todo,
                "
SELECT * FROM todos WHERE user_id = (SELECT id FROM users WHERE user = ?)
                ", self.user.as_u64())
                .fetch_all(&pool)
                .await?
        })
    }

    pub async fn add(&self, value: String, pool: MySqlPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let (Some(cid), Some(gid)) = (self.channel, self.guild) {
            sqlx::query!(
                "
INSERT INTO todos (user_id, guild_id, channel_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    (SELECT id FROM guilds WHERE guild = ?),
    (SELECT id FROM channels WHERE channel = ?),
    ?
)
                ", self.user.as_u64(), gid.as_u64(), cid.as_u64(), value)
                .execute(&pool)
                .await?;
        }
        else if let Some(gid) = self.guild {
            sqlx::query!(
                "
INSERT INTO todos (user_id, guild_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    (SELECT id FROM guilds WHERE guild = ?),
    ?
)
                ", self.user.as_u64(), gid.as_u64(), value)
                .execute(&pool)
                .await?;
        }
        else {
            sqlx::query!(
                "
INSERT INTO todos (user_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    ?
)
                ", self.user.as_u64(), value)
                .execute(&pool)
                .await?;
        }

        Ok(())
    }

    pub async fn remove(&self, num: u32, pool: MySqlPool) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        sqlx::query!(
            "
DELETE FROM todos WHERE id = (SELECT id FROM (SELECT id FROM todos LIMIT ?,1) AS t)
            ", num)
            .execute(&pool)
            .await?;

        Ok(())
    }

    pub async fn clear(&self, pool: MySqlPool) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        if let Some(cid) = self.channel {
            sqlx::query!(
                "
DELETE FROM todos WHERE channel_id = (SELECT id FROM channels WHERE channel = ?)
                ", cid.as_u64())
                .execute(&pool)
                .await?;
        }
        else if let Some(gid) = self.guild {
            sqlx::query!(
                "
DELETE FROM todos WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND channel_id IS NULL
                ", gid.as_u64())
                .execute(&pool)
                .await?;
        }
        else {
            sqlx::query!(
                "
DELETE FROM todos WHERE user_id = (SELECT id FROM users WHERE user = ?)
                ", self.user.as_u64())
                .execute(&pool)
                .await?;
        }

        Ok(())
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

    let mut split = args.split(" ");

    if let Some(target) = split.next() {
        let target_opt = match target {
            "user" =>
                Some(TodoTarget {user: msg.author.id, guild: None, channel: None}),

            "channel" =>
                if let Some(gid) = msg.guild_id {
                    Some(TodoTarget {user: msg.author.id, guild: Some(gid), channel: Some(msg.channel_id)})
                }
                else {
                    None
                },

            "server" | "guild" => {
                if let Some(gid) = msg.guild_id {
                    Some(TodoTarget {user: msg.author.id, guild: Some(gid), channel: None})
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
                todo(ctx, target, subcommand, "".to_string()).await;
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

async fn todo(ctx: &Context, target: TodoTarget, subcommand: SubCommand, extra: String) {
    let pool = ctx.data.read().await
        .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

    match subcommand {
        SubCommand::View => {
            println!("{:?}", target.view(pool).await.unwrap());
        },

        SubCommand::Add => {

        },

        SubCommand::Remove => {

        },

        SubCommand::Clear => {

        },
    }
}
