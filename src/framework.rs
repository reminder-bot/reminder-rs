use async_trait::async_trait;

use serenity::{
    client::Context,
    framework::Framework,
    model::channel::Message,
};

use std::{
    collections::HashMap,
    fmt,
};

use serenity::framework::standard::CommandFn;

#[derive(Debug)]
pub enum PermissionLevel {
    Unrestricted,
    Managed,
    Restricted,
}

pub struct Command {
    pub name: &'static str,
    pub required_perms: PermissionLevel,
    pub supports_dm: bool,
    pub can_blacklist: bool,
    pub func: CommandFn,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
         .field("name", &self.name)
         .field("required_perms", &self.required_perms)
         .field("supports_dm", &self.supports_dm)
         .field("can_blacklist", &self.can_blacklist)
         .finish()
    }
}

// create event handler for bot
pub struct RegexFramework {
    commands: HashMap<String, &'static Command>,
    command_names: String,
    default_prefix: String,
    ignore_bots: bool,
}

impl RegexFramework {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
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

    pub fn add_command(mut self, name: String, command: &'static Command) -> Self {
        self.commands.insert(name, command);

        self
    }

    pub fn build(mut self) -> Self {
        self.command_names = self.commands
            .keys()
            .map(|k| &k[..])
            .collect::<Vec<&str>>()
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
