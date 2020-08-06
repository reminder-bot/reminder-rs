use async_trait::async_trait;

use serenity::{
    client::Context,
    framework::Framework,
    model::channel::Message,
};

use std::{
    collections::HashSet,
    hash::{
        Hash,
        Hasher
    },
};

use serenity::framework::standard::CommandFn;

pub enum PermissionLevel {
    Unrestricted,
    Managed,
    Restricted,
}

pub struct Command {
    name: String,
    required_perms: PermissionLevel,
    can_blacklist: bool,
    supports_dm: bool,
    func: CommandFn,
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Command {}

// create event handler for bot
pub struct RegexFramework {
    commands: HashSet<Command>,
    command_names: String,
    default_prefix: String,
    ignore_bots: bool,
}

impl Command {
    pub fn from(name: &str, required_perms: PermissionLevel, func: CommandFn) -> Self {
        Command {
            name: name.to_string(),
            required_perms,
            can_blacklist: true,
            supports_dm: false,
            func,
        }
    }

    pub fn can_blacklist(&mut self, can_blacklist: bool) -> &mut Self {
        self.can_blacklist = can_blacklist;

        self
    }

    pub fn supports_dm(&mut self, supports_dm: bool) -> &mut Self {
        self.supports_dm = supports_dm;

        self
    }
}

impl RegexFramework {
    pub fn new() -> Self {
        Self {
            commands: HashSet::new(),
            command_names: String::new(),
            default_prefix: String::from("$"),
            ignore_bots: true,
        }
    }

    pub fn default_prefix(mut self, new_prefix: &str) -> Self {
        self.default_prefix = new_prefix.to_string();

        self
    }

    pub fn ignore_bots(mut self, ignore_bots: bool) -> Self {
        self.ignore_bots = ignore_bots;

        self
    }

    pub fn add_command(mut self, command: Command) -> Self {
        self.commands.insert(command);

        self
    }

    pub fn build(mut self) -> Self {
        self.command_names = self.commands
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<String>>()
            .join("|");

        self
    }
}

#[async_trait]
impl Framework for RegexFramework {
    async fn dispatch(&self, ctx: Context, msg: Message) {
        println!("Message received! command_names=={}", self.command_names);
    }
}
