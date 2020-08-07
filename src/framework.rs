use async_trait::async_trait;

use serenity::{
    client::Context,
    framework::Framework,
    model::{
        guild::Guild,
        channel::{
            Channel, GuildChannel, Message,
        }
    },
};

use log::{
    warn,
    error,
    debug,
    info,
};

use regex::{
    Regex, Match
};

use std::{
    collections::HashMap,
    fmt,
};

use serenity::framework::standard::CommandFn;
use crate::SQLPool;

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
    regex_matcher: Regex,
    default_prefix: String,
    client_id: u64,
    ignore_bots: bool,
}

impl RegexFramework {
    pub fn new(client_id: u64) -> Self {
        Self {
            commands: HashMap::new(),
            regex_matcher: Regex::new(r#"^$"#).unwrap(),
            default_prefix: String::from("$"),
            client_id,
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
        let command_names;

        {
            let mut command_names_vec = self.commands
                .keys()
                .map(|k| &k[..])
                .collect::<Vec<&str>>();

            command_names_vec.sort_unstable_by(|a, b| b.len().cmp(&a.len()));

            command_names = command_names_vec.join("|");
        }

        info!("Command names: {}", command_names);

        let match_string = r#"^(?:(?:<@ID>\s+)|(?:<@!ID>\s+)|(?P<prefix>\S{1,5}?))(?P<cmd>COMMANDS)(?:$|\s+(?P<args>.*))$"#
            .replace("COMMANDS", command_names.as_str())
            .replace("ID", self.client_id.to_string().as_str());

        self.regex_matcher = Regex::new(match_string.as_str()).unwrap();

        self
    }
}

enum PermissionCheck {
    None,  // No permissions
    Basic, // Send + Embed permissions (sufficient to reply)
    All,   // Above + Manage Webhooks (sufficient to operate)
}

#[async_trait]
impl Framework for RegexFramework {
    async fn dispatch(&self, ctx: Context, msg: Message) {

        async fn check_self_permissions(ctx: &Context, guild: &Guild, channel: &GuildChannel) -> Result<PermissionCheck, Box<dyn std::error::Error>> {
            let user_id = ctx.cache.current_user_id().await;

            let guild_perms = guild.member_permissions(user_id);
            let perms = channel.permissions_for_user(ctx, user_id).await?;

            let basic_perms = perms.send_messages() && perms.embed_links();

            Ok(if basic_perms && guild_perms.manage_webhooks() {
                PermissionCheck::All
            }
            else if basic_perms {
                PermissionCheck::Basic
            }
            else {
                PermissionCheck::None
            })
        }

        async fn check_prefix(ctx: &Context, guild_id: u64, prefix_opt: Option<Match<'_>>) -> bool {
            if let Some(prefix) = prefix_opt {
                let pool = ctx.data.read().await
                    .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

                match sqlx::query!("SELECT prefix FROM guilds WHERE id = ?", guild_id)
                    .fetch_one(&pool)
                    .await {
                    Ok(row) => {
                        prefix.as_str() == row.prefix
                    }

                    Err(sqlx::Error::RowNotFound) => {
                        prefix.as_str() == "$"
                    }

                    Err(e) => {
                        warn!("Unexpected error in prefix query: {:?}", e);

                        false
                    }
                }
            }
            else {
                true
            }
        }

        // gate to prevent analysing messages unnecessarily
        if (msg.author.bot && self.ignore_bots) ||
            msg.tts                             ||
            msg.content.len() == 0              ||
            msg.attachments.len() > 0
        {
            return
        }

        // Guild Command
        else if let (Some(guild), Some(Channel::Guild(channel))) = (msg.guild(&ctx).await, msg.channel(&ctx).await) {

            if let Some(full_match) = self.regex_matcher.captures(&msg.content[..]) {

                if check_prefix(&ctx, *guild.id.as_u64(), full_match.name("prefix")).await {

                    debug!("Prefix matched on {}", msg.content);

                    match check_self_permissions(&ctx, &guild, &channel).await {
                        Ok(perms) => match perms {
                            PermissionCheck::All => {}

                            PermissionCheck::Basic => {}

                            PermissionCheck::None => {
                                warn!("Missing enough permissions for guild {}", guild.id);
                            }
                        }

                        Err(e) => {
                            error!("Error occurred getting permissions in guild {}: {:?}", guild.id, e);
                        }
                    }
                }
            }
        }

        // DM Command
        else {

        }
    }
}
