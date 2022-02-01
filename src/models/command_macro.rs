use std::collections::HashMap;

use poise::{
    serenity::{
        json::Value,
        model::{
            id::{ChannelId, GuildId, RoleId, UserId},
            interactions::application_command::{
                ApplicationCommandInteraction, ApplicationCommandInteractionData,
                ApplicationCommandInteractionDataOption, ApplicationCommandOptionType,
                ApplicationCommandType,
            },
        },
    },
    ApplicationCommandOrAutocompleteInteraction,
};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use sqlx::Executor;

use crate::Database;

pub struct CommandMacro {
    pub guild_id: GuildId,
    pub name: String,
    pub description: Option<String>,
    pub commands: Vec<CommandOptions>,
}

impl CommandMacro {
    pub async fn from_guild(
        db_pool: impl Executor<'_, Database = Database>,
        guild_id: impl Into<GuildId>,
    ) -> Vec<Self> {
        let guild_id = guild_id.into();

        sqlx::query!(
            "SELECT * FROM macro WHERE guild_id = (SELECT id FROM guilds WHERE guild = ?)",
            guild_id.0
        )
        .fetch_all(db_pool)
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

#[derive(Serialize, Deserialize, Clone)]
pub enum OptionValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    User(UserId),
    Channel(ChannelId),
    Role(RoleId),
    Mentionable(u64),
    Number(f64),
}

impl OptionValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            OptionValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OptionValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_channel_id(&self) -> Option<ChannelId> {
        match self {
            OptionValue::Channel(c) => Some(*c),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            OptionValue::String(s) => s.to_string(),
            OptionValue::Integer(i) => i.to_string(),
            OptionValue::Boolean(b) => b.to_string(),
            OptionValue::User(u) => u.to_string(),
            OptionValue::Channel(c) => c.to_string(),
            OptionValue::Role(r) => r.to_string(),
            OptionValue::Mentionable(m) => m.to_string(),
            OptionValue::Number(n) => n.to_string(),
        }
    }

    fn as_value(&self) -> Value {
        match self {
            OptionValue::String(s) => Value::String(s.to_string()),
            OptionValue::Integer(i) => Value::Number(i.to_owned().into()),
            OptionValue::Boolean(b) => Value::Bool(b.to_owned()),
            OptionValue::User(u) => Value::String(u.to_string()),
            OptionValue::Channel(c) => Value::String(c.to_string()),
            OptionValue::Role(r) => Value::String(r.to_string()),
            OptionValue::Mentionable(m) => Value::String(m.to_string()),
            OptionValue::Number(n) => Value::Number(Number::from_f64(n.to_owned()).unwrap()),
        }
    }

    fn kind(&self) -> ApplicationCommandOptionType {
        match self {
            OptionValue::String(_) => ApplicationCommandOptionType::String,
            OptionValue::Integer(_) => ApplicationCommandOptionType::Integer,
            OptionValue::Boolean(_) => ApplicationCommandOptionType::Boolean,
            OptionValue::User(_) => ApplicationCommandOptionType::User,
            OptionValue::Channel(_) => ApplicationCommandOptionType::Channel,
            OptionValue::Role(_) => ApplicationCommandOptionType::Role,
            OptionValue::Mentionable(_) => ApplicationCommandOptionType::Mentionable,
            OptionValue::Number(_) => ApplicationCommandOptionType::Number,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandOptions {
    pub command: String,
    pub subcommand: Option<String>,
    pub subcommand_group: Option<String>,
    pub options: HashMap<String, OptionValue>,
}

impl Into<ApplicationCommandInteractionData> for CommandOptions {
    fn into(self) -> ApplicationCommandInteractionData {
        ApplicationCommandInteractionData {
            name: self.command,
            kind: ApplicationCommandType::ChatInput,
            options: self
                .options
                .iter()
                .map(|(name, value)| ApplicationCommandInteractionDataOption {
                    name: name.to_string(),
                    value: Some(value.as_value()),
                    kind: value.kind(),
                    options: vec![],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }
}

impl CommandOptions {
    pub fn new(command: impl ToString) -> Self {
        Self {
            command: command.to_string(),
            subcommand: None,
            subcommand_group: None,
            options: Default::default(),
        }
    }

    pub fn populate(&mut self, interaction: &ApplicationCommandInteraction) {
        fn match_option(
            option: ApplicationCommandInteractionDataOption,
            cmd_opts: &mut CommandOptions,
        ) {
            match option.kind {
                ApplicationCommandOptionType::SubCommand => {
                    cmd_opts.subcommand = Some(option.name);

                    for opt in option.options {
                        match_option(opt, cmd_opts);
                    }
                }
                ApplicationCommandOptionType::SubCommandGroup => {
                    cmd_opts.subcommand_group = Some(option.name);

                    for opt in option.options {
                        match_option(opt, cmd_opts);
                    }
                }
                ApplicationCommandOptionType::String => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::String(option.value.unwrap().as_str().unwrap().to_string()),
                    );
                }
                ApplicationCommandOptionType::Integer => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Integer(option.value.map(|m| m.as_i64()).flatten().unwrap()),
                    );
                }
                ApplicationCommandOptionType::Boolean => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Boolean(option.value.map(|m| m.as_bool()).flatten().unwrap()),
                    );
                }
                ApplicationCommandOptionType::User => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::User(UserId(
                            option
                                .value
                                .map(|m| m.as_str().map(|s| s.parse::<u64>().ok()))
                                .flatten()
                                .flatten()
                                .unwrap(),
                        )),
                    );
                }
                ApplicationCommandOptionType::Channel => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Channel(ChannelId(
                            option
                                .value
                                .map(|m| m.as_str().map(|s| s.parse::<u64>().ok()))
                                .flatten()
                                .flatten()
                                .unwrap(),
                        )),
                    );
                }
                ApplicationCommandOptionType::Role => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Role(RoleId(
                            option
                                .value
                                .map(|m| m.as_str().map(|s| s.parse::<u64>().ok()))
                                .flatten()
                                .flatten()
                                .unwrap(),
                        )),
                    );
                }
                ApplicationCommandOptionType::Mentionable => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Mentionable(
                            option.value.map(|m| m.as_u64()).flatten().unwrap(),
                        ),
                    );
                }
                ApplicationCommandOptionType::Number => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Number(option.value.map(|m| m.as_f64()).flatten().unwrap()),
                    );
                }
                _ => {}
            }
        }

        for option in &interaction.data.options {
            match_option(option.clone(), self)
        }
    }
}
