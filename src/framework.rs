use async_trait::async_trait;

use serenity::{
    client::Context,
    framework::{
        Framework,
        standard::CommandResult,
    },
    model::{
        guild::{
            Guild,
            Member,
        },
        channel::{
            Channel, GuildChannel, Message,
        }
    },
    futures::prelude::future::BoxFuture,
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
    env,
};

use crate::{
    models::ChannelData,
    SQLPool,
    consts::PREFIX,
};

type CommandFn = for<'fut> fn(&'fut Context, &'fut Message, String) -> BoxFuture<'fut, CommandResult>;

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

impl Command {
    async fn check_permissions(&self, ctx: &Context, guild: &Guild, member: &Member) -> bool {

        guild.member_permissions(&member.user).manage_guild() || match self.required_perms {
            PermissionLevel::Unrestricted => true,

            PermissionLevel::Managed => {
                let pool = ctx.data.read().await
                    .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

                match sqlx::query!("
SELECT
    role
FROM
    roles
INNER JOIN
    command_restrictions ON roles.id = command_restrictions.role_id
WHERE
    command_restrictions.command = ? AND
    roles.guild_id = (
        SELECT
            id
        FROM
            guilds
        WHERE
            guild = ?
                )", self.name, guild.id.as_u64())
                    .fetch_all(&pool)
                    .await {

                    Ok(rows) => {

                        let role_ids = member.roles.iter().map(|r| *r.as_u64()).collect::<Vec<u64>>();

                        for row in rows {
                            if role_ids.contains(&row.role) {
                                return true
                            }
                        }

                        false
                    }

                    Err(sqlx::Error::RowNotFound) => {
                        false
                    }

                    Err(e) => {
                        warn!("Unexpected error occurred querying command_restrictions: {:?}", e);

                        false
                    }

                }
            }

            PermissionLevel::Restricted => {
                false
            }
        }

    }
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
    command_matcher: Regex,
    dm_regex_matcher: Regex,
    default_prefix: String,
    client_id: u64,
    ignore_bots: bool,
}

impl RegexFramework {
    pub fn new(client_id: u64) -> Self {
        Self {
            commands: HashMap::new(),
            command_matcher: Regex::new(r#"^$"#).unwrap(),
            dm_regex_matcher: Regex::new(r#"^$"#).unwrap(),
            default_prefix: env::var("DEFAULT_PREFIX").unwrap_or_else(|_| PREFIX.to_string()),
            client_id,
            ignore_bots: true,
        }
    }

    pub fn default_prefix<T: ToString>(mut self, new_prefix: T) -> Self {
        self.default_prefix = new_prefix.to_string();

        self
    }

    pub fn ignore_bots(mut self, ignore_bots: bool) -> Self {
        self.ignore_bots = ignore_bots;

        self
    }

    pub fn add_command(mut self, name: &str, command: &'static Command) -> Self {
        self.commands.insert(name.to_string(), command);

        self
    }

    pub fn build(mut self) -> Self {
        {
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

            {
                let match_string = r#"^(?:(?:<@ID>\s+)|(?:<@!ID>\s+)|(?P<prefix>\S{1,5}?))(?P<cmd>COMMANDS)(?:$|\s+(?P<args>.*))$"#
                    .replace("COMMANDS", command_names.as_str())
                    .replace("ID", self.client_id.to_string().as_str());

                self.command_matcher = Regex::new(match_string.as_str()).unwrap();
            }
        }

        {
            let dm_command_names;

            {
                let mut command_names_vec = self.commands
                    .iter()
                    .filter_map(|(key, command)| {
                        if command.supports_dm {
                            Some(&key[..])
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&str>>();

                command_names_vec.sort_unstable_by(|a, b| b.len().cmp(&a.len()));

                dm_command_names = command_names_vec.join("|");
            }

            {
                let match_string = r#"^(?:(?:<@ID>\s+)|(?:<@!ID>\s+)|(\$)|())(?P<cmd>COMMANDS)(?:$|\s+(?P<args>.*))$"#
                    .replace("COMMANDS", dm_command_names.as_str())
                    .replace("ID", self.client_id.to_string().as_str());

                self.dm_regex_matcher = Regex::new(match_string.as_str()).unwrap();
            }
        }

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

        async fn check_self_permissions(ctx: &Context, guild: &Guild, channel: &GuildChannel) -> Result<PermissionCheck, Box<dyn std::error::Error + Sync + Send>> {
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

        async fn check_prefix(ctx: &Context, guild: &Guild, prefix_opt: Option<Match<'_>>) -> bool {
            if let Some(prefix) = prefix_opt {
                let pool = ctx.data.read().await
                    .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

                match sqlx::query!("SELECT prefix FROM guilds WHERE guild = ?", guild.id.as_u64())
                    .fetch_one(&pool)
                    .await {
                    Ok(row) => {
                        prefix.as_str() == row.prefix
                    }

                    Err(sqlx::Error::RowNotFound) => {
                        let _ = sqlx::query!("INSERT INTO guilds (guild, name) VALUES (?, ?)", guild.id.as_u64(), guild.name)
                            .execute(&pool)
                            .await;

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
            msg.content.is_empty()              ||
            !msg.attachments.is_empty()         {}

        // Guild Command
        else if let (Some(guild), Some(Channel::Guild(channel))) = (msg.guild(&ctx).await, msg.channel(&ctx).await) {

            let member = guild.member(&ctx, &msg.author).await.unwrap();

            if let Some(full_match) = self.command_matcher.captures(&msg.content[..]) {

                if check_prefix(&ctx, &guild, full_match.name("prefix")).await {

                    debug!("Prefix matched on {}", msg.content);

                    match check_self_permissions(&ctx, &guild, &channel).await {
                        Ok(perms) => match perms {
                            PermissionCheck::All => {
                                let pool = ctx.data.read().await
                                    .get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

                                let command = self.commands.get(full_match.name("cmd").unwrap().as_str()).unwrap();
                                let channel_data = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool).await;

                                if !command.can_blacklist || !channel_data.map(|c| c.blacklisted).unwrap_or(false) {
                                    let args = full_match.name("args")
                                        .map(|m| m.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    if command.check_permissions(&ctx, &guild, &member).await {
                                        (command.func)(&ctx, &msg, args).await.unwrap();
                                    }
                                }
                            }

                            PermissionCheck::Basic => {
                                let _ = msg.channel_id.say(&ctx, "Not enough perms").await;
                            }

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
        else if let Some(full_match) = self.dm_regex_matcher.captures(&msg.content[..]) {
            let command = self.commands.get(full_match.name("cmd").unwrap().as_str()).unwrap();
            let args = full_match.name("args")
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();

            (command.func)(&ctx, &msg, args).await.unwrap();
        }
    }
}
