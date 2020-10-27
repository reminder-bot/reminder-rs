use regex_command_attr::command;

use serenity::{
    client::Context,
    constants::MESSAGE_CODE_LIMIT,
    model::{
        channel::Message,
        id::{ChannelId, GuildId, UserId},
    },
};

use std::fmt;

use crate::{
    models::{GuildData, UserData},
    SQLPool,
};
use sqlx::MySqlPool;
use std::convert::TryFrom;

use async_trait::async_trait;

#[derive(Debug)]
struct TodoNotFound;

impl std::error::Error for TodoNotFound {}
impl fmt::Display for TodoNotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Todo not found")
    }
}

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
    pub fn command(&self, subcommand_opt: Option<SubCommand>) -> String {
        let context = if self.channel.is_some() {
            "channel"
        } else if self.guild.is_some() {
            "guild"
        } else {
            "user"
        };

        if let Some(subcommand) = subcommand_opt {
            format!("todo {} {}", context, subcommand.to_string())
        } else {
            format!("todo {}", context)
        }
    }

    pub fn name(&self) -> String {
        if self.channel.is_some() {
            "Channel"
        } else if self.guild.is_some() {
            "Guild"
        } else {
            "User"
        }
        .to_string()
    }

    pub async fn view(
        &self,
        pool: MySqlPool,
    ) -> Result<Vec<Todo>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(if let Some(cid) = self.channel {
            sqlx::query_as!(
                Todo,
                "
SELECT id, user_id, guild_id, channel_id, value FROM todos WHERE channel_id = (SELECT id FROM channels WHERE channel = ?)
                ",
                cid.as_u64()
            )
            .fetch_all(&pool)
            .await?
        } else if let Some(gid) = self.guild {
            sqlx::query_as!(
                Todo,
                "
SELECT id, user_id, guild_id, channel_id, value FROM todos WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND channel_id IS NULL
                ",
                gid.as_u64()
            )
            .fetch_all(&pool)
            .await?
        } else {
            sqlx::query_as!(
                Todo,
                "
SELECT id, user_id, guild_id, channel_id, value FROM todos WHERE user_id = (SELECT id FROM users WHERE user = ?) AND guild_id IS NULL
                ",
                self.user.as_u64()
            )
            .fetch_all(&pool)
            .await?
        })
    }

    pub async fn add(
        &self,
        value: String,
        pool: MySqlPool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let (Some(cid), Some(gid)) = (self.channel, self.guild) {
            sqlx::query!(
                "
INSERT INTO todos (user_id, guild_id, channel_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    (SELECT id FROM guilds WHERE guild = ?),
    (SELECT id FROM channels WHERE channel = ?),
    ?
)
                ",
                self.user.as_u64(),
                gid.as_u64(),
                cid.as_u64(),
                value
            )
            .execute(&pool)
            .await?;
        } else if let Some(gid) = self.guild {
            sqlx::query!(
                "
INSERT INTO todos (user_id, guild_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    (SELECT id FROM guilds WHERE guild = ?),
    ?
)
                ",
                self.user.as_u64(),
                gid.as_u64(),
                value
            )
            .execute(&pool)
            .await?;
        } else {
            sqlx::query!(
                "
INSERT INTO todos (user_id, value) VALUES (
    (SELECT id FROM users WHERE user = ?),
    ?
)
                ",
                self.user.as_u64(),
                value
            )
            .execute(&pool)
            .await?;
        }

        Ok(())
    }

    pub async fn remove(
        &self,
        num: usize,
        pool: &MySqlPool,
    ) -> Result<Todo, Box<dyn std::error::Error + Sync + Send>> {
        let todos = self.view(pool.clone()).await?;

        if let Some(removal_todo) = todos.get(num) {
            let deleting = sqlx::query_as!(
                Todo,
                "
SELECT id, user_id, guild_id, channel_id, value FROM todos WHERE id = ?
                ",
                removal_todo.id
            )
            .fetch_one(&pool.clone())
            .await?;

            sqlx::query!(
                "
DELETE FROM todos WHERE id = ?
                ",
                removal_todo.id
            )
            .execute(pool)
            .await?;

            Ok(deleting)
        } else {
            Err(Box::new(TodoNotFound))
        }
    }

    pub async fn clear(
        &self,
        pool: &MySqlPool,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        if let Some(cid) = self.channel {
            sqlx::query!(
                "
DELETE FROM todos WHERE channel_id = (SELECT id FROM channels WHERE channel = ?)
                ",
                cid.as_u64()
            )
            .execute(pool)
            .await?;
        } else if let Some(gid) = self.guild {
            sqlx::query!(
                "
DELETE FROM todos WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?) AND channel_id IS NULL
                ",
                gid.as_u64()
            )
            .execute(pool)
            .await?;
        } else {
            sqlx::query!(
                "
DELETE FROM todos WHERE user_id = (SELECT id FROM users WHERE user = ?) AND guild_id IS NULL
                ",
                self.user.as_u64()
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    async fn execute(&self, ctx: &Context, msg: &Message, subcommand: SubCommand, extra: String) {
        let pool = ctx
            .data
            .read()
            .await
            .get::<SQLPool>()
            .cloned()
            .expect("Could not get SQLPool from data");

        let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();
        let prefix = GuildData::prefix_from_id(msg.guild_id, &pool).await;

        match subcommand {
            SubCommand::View => {
                let todo_items = self.view(pool).await.unwrap();
                let mut todo_groups = vec!["".to_string()];
                let mut char_count = 0;

                todo_items.iter().enumerate().for_each(|(count, todo)| {
                    let display = format!("{}: {}\n", count + 1, todo.value);

                    if char_count + display.len() > MESSAGE_CODE_LIMIT as usize {
                        char_count = display.len();

                        todo_groups.push(display);
                    } else {
                        char_count += display.len();

                        let last_group = todo_groups.pop().unwrap();

                        todo_groups.push(format!("{}{}", last_group, display));
                    }
                });

                for group in todo_groups {
                    let _ = msg
                        .channel_id
                        .send_message(&ctx, |m| {
                            m.embed(|e| e.title(format!("{} Todo", self.name())).description(group))
                        })
                        .await;
                }
            }

            SubCommand::Add => {
                let content = user_data
                    .response(&pool, "todo/added")
                    .await
                    .replacen("{name}", &extra, 1);

                self.add(extra, pool).await.unwrap();

                let _ = msg.channel_id.say(&ctx, content).await;
            }

            SubCommand::Remove => {
                let _ = if let Ok(num) = extra.parse::<usize>() {
                    if let Ok(todo) = self.remove(num - 1, &pool).await {
                        let content = user_data.response(&pool, "todo/removed").await.replacen(
                            "{}",
                            &todo.value,
                            1,
                        );

                        msg.channel_id.say(&ctx, content)
                    } else {
                        msg.channel_id
                            .say(&ctx, user_data.response(&pool, "todo/error_index").await)
                    }
                } else {
                    let content = user_data
                        .response(&pool, "todo/error_value")
                        .await
                        .replacen("{prefix}", &prefix, 1)
                        .replacen("{command}", &self.command(Some(subcommand)), 1);

                    msg.channel_id.say(&ctx, content)
                }
                .await;
            }

            SubCommand::Clear => {
                self.clear(&pool).await.unwrap();

                let content = user_data.response(&pool, "todo/cleared").await;

                let _ = msg.channel_id.say(&ctx, content).await;
            }
        }
    }
}

enum SubCommand {
    View,
    Add,
    Remove,
    Clear,
}

impl TryFrom<Option<&str>> for SubCommand {
    type Error = ();

    fn try_from(value: Option<&str>) -> Result<Self, Self::Error> {
        match value {
            Some("add") => Ok(SubCommand::Add),

            Some("remove") => Ok(SubCommand::Remove),

            Some("clear") => Ok(SubCommand::Clear),

            None | Some("") => Ok(SubCommand::View),

            Some(_unrecognised) => Err(()),
        }
    }
}

impl ToString for SubCommand {
    fn to_string(&self) -> String {
        match self {
            SubCommand::View => "",
            SubCommand::Add => "add",
            SubCommand::Remove => "remove",
            SubCommand::Clear => "clear",
        }
        .to_string()
    }
}

#[async_trait]
trait Execute {
    async fn execute(self, ctx: &Context, msg: &Message, extra: String, target: TodoTarget);
}

#[async_trait]
impl Execute for Result<SubCommand, ()> {
    async fn execute(self, ctx: &Context, msg: &Message, extra: String, target: TodoTarget) {
        if let Ok(subcommand) = self {
            target.execute(ctx, msg, subcommand, extra).await;
        } else {
            show_help(&ctx, msg, Some(target)).await;
        }
    }
}

#[command]
#[permission_level(Managed)]
async fn todo_user(ctx: &Context, msg: &Message, args: String) {
    let mut split = args.split(' ');

    let target = TodoTarget {
        user: msg.author.id,
        guild: None,
        channel: None,
    };

    let subcommand_opt = SubCommand::try_from(split.next());

    subcommand_opt
        .execute(ctx, msg, split.collect::<Vec<&str>>().join(" "), target)
        .await;
}

#[command("todos")]
#[supports_dm(false)]
#[permission_level(Managed)]
async fn todo_channel(ctx: &Context, msg: &Message, args: String) {
    let mut split = args.split(' ');

    let target = TodoTarget {
        user: msg.author.id,
        guild: msg.guild_id,
        channel: Some(msg.channel_id),
    };

    let subcommand_opt = SubCommand::try_from(split.next());

    subcommand_opt
        .execute(ctx, msg, split.collect::<Vec<&str>>().join(" "), target)
        .await;
}

#[command("todos")]
#[supports_dm(false)]
#[permission_level(Managed)]
async fn todo_guild(ctx: &Context, msg: &Message, args: String) {
    let mut split = args.split(' ');

    let target = TodoTarget {
        user: msg.author.id,
        guild: msg.guild_id,
        channel: None,
    };

    let subcommand_opt = SubCommand::try_from(split.next());

    subcommand_opt
        .execute(ctx, msg, split.collect::<Vec<&str>>().join(" "), target)
        .await;
}

async fn show_help(ctx: &Context, msg: &Message, target: Option<TodoTarget>) {
    let pool = ctx
        .data
        .read()
        .await
        .get::<SQLPool>()
        .cloned()
        .expect("Could not get SQLPool from data");

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();
    let prefix = GuildData::prefix_from_id(msg.guild_id, &pool).await;

    let content = user_data
        .response(&pool, "todo/help")
        .await
        .replace("{prefix}", &prefix)
        .replace(
            "{command}",
            target
                .map_or_else(|| "todo user".to_string(), |t| t.command(None))
                .as_str(),
        );

    let _ = msg.channel_id.say(&ctx, content).await;
}
