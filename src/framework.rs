// todo move framework to its own module, split out permission checks

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use log::{error, info, warn};
use regex::{Match, Regex, RegexBuilder};
use serenity::{
    async_trait,
    builder::{CreateApplicationCommands, CreateComponents, CreateEmbed},
    cache::Cache,
    client::Context,
    framework::Framework,
    futures::prelude::future::BoxFuture,
    http::Http,
    json::Value,
    model::{
        channel::{Channel, GuildChannel, Message},
        guild::{Guild, Member},
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOption,
                ApplicationCommandOptionType,
            },
            InteractionResponseType,
        },
        prelude::application_command::ApplicationCommandInteractionDataOption,
    },
    prelude::TypeMapKey,
    FutureExt, Result as SerenityResult,
};

use crate::{
    models::{channel_data::ChannelData, CtxData},
    LimitExecutors, SQLPool,
};

#[derive(Debug, PartialEq)]
pub enum PermissionLevel {
    Unrestricted,
    Managed,
    Restricted,
}

pub struct CreateGenericResponse {
    content: String,
    embed: Option<CreateEmbed>,
    components: Option<CreateComponents>,
}

impl CreateGenericResponse {
    pub fn new() -> Self {
        Self { content: "".to_string(), embed: None, components: None }
    }

    pub fn content<D: ToString>(mut self, content: D) -> Self {
        self.content = content.to_string();

        self
    }

    pub fn embed<F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed>(mut self, f: F) -> Self {
        let mut embed = CreateEmbed::default();
        f(&mut embed);

        self.embed = Some(embed);
        self
    }

    pub fn components<F: FnOnce(&mut CreateComponents) -> &mut CreateComponents>(
        mut self,
        f: F,
    ) -> Self {
        let mut components = CreateComponents::default();
        f(&mut components);

        self.components = Some(components);
        self
    }
}

#[async_trait]
pub trait CommandInvoke {
    fn channel_id(&self) -> ChannelId;
    fn guild_id(&self) -> Option<GuildId>;
    fn guild(&self, cache: Arc<Cache>) -> Option<Guild>;
    fn author_id(&self) -> UserId;
    async fn member(&self, context: &Context) -> SerenityResult<Member>;
    fn msg(&self) -> Option<Message>;
    fn interaction(&self) -> Option<ApplicationCommandInteraction>;
    async fn respond(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()>;
    async fn followup(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()>;
}

#[async_trait]
impl CommandInvoke for Message {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    fn guild_id(&self) -> Option<GuildId> {
        self.guild_id
    }

    fn guild(&self, cache: Arc<Cache>) -> Option<Guild> {
        self.guild(cache)
    }

    fn author_id(&self) -> UserId {
        self.author.id
    }

    async fn member(&self, context: &Context) -> SerenityResult<Member> {
        self.member(context).await
    }

    fn msg(&self) -> Option<Message> {
        Some(self.clone())
    }

    fn interaction(&self) -> Option<ApplicationCommandInteraction> {
        None
    }

    async fn respond(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        self.channel_id
            .send_message(http, |m| {
                m.content(generic_response.content);

                if let Some(embed) = generic_response.embed {
                    m.set_embed(embed.clone());
                }

                if let Some(components) = generic_response.components {
                    m.components(|c| {
                        *c = components;
                        c
                    });
                }

                m
            })
            .await
            .map(|_| ())
    }

    async fn followup(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        self.channel_id
            .send_message(http, |m| {
                m.content(generic_response.content);

                if let Some(embed) = generic_response.embed {
                    m.set_embed(embed.clone());
                }

                if let Some(components) = generic_response.components {
                    m.components(|c| {
                        *c = components;
                        c
                    });
                }

                m
            })
            .await
            .map(|_| ())
    }
}

#[async_trait]
impl CommandInvoke for ApplicationCommandInteraction {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    fn guild_id(&self) -> Option<GuildId> {
        self.guild_id
    }

    fn guild(&self, cache: Arc<Cache>) -> Option<Guild> {
        if let Some(guild_id) = self.guild_id {
            guild_id.to_guild_cached(cache)
        } else {
            None
        }
    }

    fn author_id(&self) -> UserId {
        self.member.as_ref().unwrap().user.id
    }

    async fn member(&self, _: &Context) -> SerenityResult<Member> {
        Ok(self.member.clone().unwrap())
    }

    fn msg(&self) -> Option<Message> {
        None
    }

    fn interaction(&self) -> Option<ApplicationCommandInteraction> {
        Some(self.clone())
    }

    async fn respond(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        self.create_interaction_response(http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(
                |d| {
                    d.content(generic_response.content);

                    if let Some(embed) = generic_response.embed {
                        d.add_embed(embed.clone());
                    }

                    if let Some(components) = generic_response.components {
                        d.components(|c| {
                            *c = components;
                            c
                        });
                    }

                    d
                },
            )
        })
        .await
        .map(|_| ())
    }

    async fn followup(
        &self,
        http: Arc<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        self.create_followup_message(http, |d| {
            d.content(generic_response.content);

            if let Some(embed) = generic_response.embed {
                d.add_embed(embed.clone());
            }

            if let Some(components) = generic_response.components {
                d.components(|c| {
                    *c = components;
                    c
                });
            }

            d
        })
        .await
        .map(|_| ())
    }
}

#[derive(Debug)]
pub struct Arg {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: ApplicationCommandOptionType,
    pub required: bool,
    pub options: &'static [&'static Self],
}

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
}

pub struct CommandOptions {
    pub command: String,
    pub subcommand: Option<String>,
    pub subcommand_group: Option<String>,
    pub options: HashMap<String, OptionValue>,
}

impl CommandOptions {
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.options.get(key)
    }
}

impl From<ApplicationCommandInteraction> for CommandOptions {
    fn from(interaction: ApplicationCommandInteraction) -> Self {
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
                        OptionValue::String(option.value.unwrap().to_string()),
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
                            option.value.map(|m| m.as_u64()).flatten().unwrap(),
                        )),
                    );
                }
                ApplicationCommandOptionType::Channel => {
                    cmd_opts.options.insert(
                        option.name,
                        OptionValue::Channel(ChannelId(
                            option.value.map(|m| m.as_u64()).flatten().unwrap(),
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

        let mut cmd_opts = Self {
            command: interaction.data.name,
            subcommand: None,
            subcommand_group: None,
            options: Default::default(),
        };

        for option in interaction.data.options {
            match_option(option, &mut cmd_opts)
        }

        cmd_opts
    }
}

type SlashCommandFn = for<'fut> fn(
    &'fut Context,
    &'fut (dyn CommandInvoke + Sync + Send),
    CommandOptions,
) -> BoxFuture<'fut, ()>;

type TextCommandFn = for<'fut> fn(
    &'fut Context,
    &'fut (dyn CommandInvoke + Sync + Send),
    String,
) -> BoxFuture<'fut, ()>;

type MultiCommandFn =
    for<'fut> fn(&'fut Context, &'fut (dyn CommandInvoke + Sync + Send)) -> BoxFuture<'fut, ()>;

pub enum CommandFnType {
    Slash(SlashCommandFn),
    Text(TextCommandFn),
    Multi(MultiCommandFn),
}

impl CommandFnType {
    pub fn is_slash(&self) -> bool {
        match self {
            CommandFnType::Text(_) => false,
            _ => true,
        }
    }
}

pub struct Command {
    pub fun: CommandFnType,

    pub names: &'static [&'static str],

    pub desc: &'static str,
    pub examples: &'static [&'static str],
    pub group: &'static str,

    pub required_permissions: PermissionLevel,
    pub args: &'static [&'static Arg],

    pub can_blacklist: bool,
    pub supports_dm: bool,
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.names[0].hash(state)
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.names[0] == other.names[0]
    }
}

impl Eq for Command {}

impl Command {
    async fn check_permissions(&self, ctx: &Context, guild: &Guild, member: &Member) -> bool {
        if self.required_permissions == PermissionLevel::Unrestricted {
            true
        } else {
            let permissions = guild.member_permissions(&ctx, &member.user).await.unwrap();

            if permissions.manage_guild()
                || (permissions.manage_messages()
                    && self.required_permissions == PermissionLevel::Managed)
            {
                return true;
            }

            if self.required_permissions == PermissionLevel::Managed {
                let pool = ctx
                    .data
                    .read()
                    .await
                    .get::<SQLPool>()
                    .cloned()
                    .expect("Could not get SQLPool from data");

                match sqlx::query!(
                    "
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
            guild = ?)
                    ",
                    self.names[0],
                    guild.id.as_u64()
                )
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => {
                        let role_ids =
                            member.roles.iter().map(|r| *r.as_u64()).collect::<Vec<u64>>();

                        for row in rows {
                            if role_ids.contains(&row.role) {
                                return true;
                            }
                        }

                        false
                    }

                    Err(sqlx::Error::RowNotFound) => false,

                    Err(e) => {
                        warn!("Unexpected error occurred querying command_restrictions: {:?}", e);

                        false
                    }
                }
            } else {
                false
            }
        }
    }
}

pub struct RegexFramework {
    pub commands_map: HashMap<String, &'static Command>,
    pub commands: HashSet<&'static Command>,
    command_matcher: Regex,
    dm_regex_matcher: Regex,
    default_prefix: String,
    client_id: u64,
    ignore_bots: bool,
    case_insensitive: bool,
    dm_enabled: bool,
    default_text_fun: TextCommandFn,
    debug_guild: Option<GuildId>,
}

impl TypeMapKey for RegexFramework {
    type Value = Arc<RegexFramework>;
}

fn drop_text<'fut>(
    _: &'fut Context,
    _: &'fut (dyn CommandInvoke + Sync + Send),
    _: String,
) -> std::pin::Pin<std::boxed::Box<(dyn std::future::Future<Output = ()> + std::marker::Send + 'fut)>>
{
    async move {}.boxed()
}

impl RegexFramework {
    pub fn new<T: Into<u64>>(client_id: T) -> Self {
        Self {
            commands_map: HashMap::new(),
            commands: HashSet::new(),
            command_matcher: Regex::new(r#"^$"#).unwrap(),
            dm_regex_matcher: Regex::new(r#"^$"#).unwrap(),
            default_prefix: "".to_string(),
            client_id: client_id.into(),
            ignore_bots: true,
            case_insensitive: true,
            dm_enabled: true,
            default_text_fun: drop_text,
            debug_guild: None,
        }
    }

    pub fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.case_insensitive = case_insensitive;

        self
    }

    pub fn default_prefix<T: ToString>(mut self, new_prefix: T) -> Self {
        self.default_prefix = new_prefix.to_string();

        self
    }

    pub fn ignore_bots(mut self, ignore_bots: bool) -> Self {
        self.ignore_bots = ignore_bots;

        self
    }

    pub fn dm_enabled(mut self, dm_enabled: bool) -> Self {
        self.dm_enabled = dm_enabled;

        self
    }

    pub fn add_command(mut self, command: &'static Command) -> Self {
        self.commands.insert(command);

        for name in command.names {
            self.commands_map.insert(name.to_string(), command);
        }

        self
    }

    pub fn debug_guild(mut self, guild_id: Option<GuildId>) -> Self {
        self.debug_guild = guild_id;

        self
    }

    pub fn build(mut self) -> Self {
        {
            let command_names;

            {
                let mut command_names_vec =
                    self.commands_map.keys().map(|k| &k[..]).collect::<Vec<&str>>();

                command_names_vec.sort_unstable_by_key(|a| a.len());

                command_names = command_names_vec.join("|");
            }

            info!("Command names: {}", command_names);

            {
                let match_string =
                    r#"^(?:(?:<@ID>\s*)|(?:<@!ID>\s*)|(?P<prefix>\S{1,5}?))(?P<cmd>COMMANDS)(?:$|\s+(?P<args>.*))$"#
                        .replace("COMMANDS", command_names.as_str())
                        .replace("ID", self.client_id.to_string().as_str());

                self.command_matcher = RegexBuilder::new(match_string.as_str())
                    .case_insensitive(self.case_insensitive)
                    .dot_matches_new_line(true)
                    .build()
                    .unwrap();
            }
        }

        {
            let dm_command_names;

            {
                let mut command_names_vec = self
                    .commands_map
                    .iter()
                    .filter_map(
                        |(key, command)| if command.supports_dm { Some(&key[..]) } else { None },
                    )
                    .collect::<Vec<&str>>();

                command_names_vec.sort_unstable_by_key(|a| a.len());

                dm_command_names = command_names_vec.join("|");
            }

            {
                let match_string = r#"^(?:(?:<@ID>\s+)|(?:<@!ID>\s+)|(\$)|())(?P<cmd>COMMANDS)(?:$|\s+(?P<args>.*))$"#
                    .replace("COMMANDS", dm_command_names.as_str())
                    .replace("ID", self.client_id.to_string().as_str());

                self.dm_regex_matcher = RegexBuilder::new(match_string.as_str())
                    .case_insensitive(self.case_insensitive)
                    .dot_matches_new_line(true)
                    .build()
                    .unwrap();
            }
        }

        self
    }

    fn _populate_commands<'a>(
        &self,
        commands: &'a mut CreateApplicationCommands,
    ) -> &'a mut CreateApplicationCommands {
        for command in &self.commands {
            if command.fun.is_slash() {
                commands.create_application_command(|c| {
                    c.name(command.names[0]).description(command.desc);

                    for arg in command.args {
                        c.create_option(|o| {
                            o.name(arg.name)
                                .description(arg.description)
                                .kind(arg.kind)
                                .required(arg.required);

                            for option in arg.options {
                                o.create_sub_option(|s| {
                                    s.name(option.name)
                                        .description(option.description)
                                        .kind(option.kind)
                                        .required(option.required)
                                });
                            }

                            o
                        });
                    }

                    c
                });
            }
        }

        commands
    }

    pub async fn build_slash(&self, http: impl AsRef<Http>) {
        info!("Building slash commands...");

        match self.debug_guild {
            None => {
                ApplicationCommand::set_global_application_commands(&http, |c| {
                    self._populate_commands(c)
                })
                .await;
            }
            Some(debug_guild) => {
                debug_guild
                    .set_application_commands(&http, |c| self._populate_commands(c))
                    .await
                    .unwrap();
            }
        }

        info!("Slash commands built!");
    }

    pub async fn execute(&self, ctx: Context, interaction: ApplicationCommandInteraction) {
        let command = {
            self.commands_map
                .get(&interaction.data.name)
                .expect(&format!("Received invalid command: {}", interaction.data.name))
        };

        let guild = interaction.guild(ctx.cache.clone()).unwrap();
        let member = interaction.clone().member.unwrap();

        if command.check_permissions(&ctx, &guild, &member).await {
            let args = CommandOptions::from(interaction.clone());

            if !ctx.check_executing(interaction.author_id()).await {
                ctx.set_executing(interaction.author_id()).await;

                match command.fun {
                    CommandFnType::Slash(t) => t(&ctx, &interaction, args).await,
                    CommandFnType::Multi(m) => m(&ctx, &interaction).await,
                    _ => (),
                }

                ctx.drop_executing(interaction.author_id()).await;
            }
        } else if command.required_permissions == PermissionLevel::Restricted {
            let _ = interaction
                .respond(
                    ctx.http.clone(),
                    CreateGenericResponse::new().content(
                        "You must have the `Manage Server` permission to use this command.",
                    ),
                )
                .await;
        } else if command.required_permissions == PermissionLevel::Managed {
            let _ = interaction
                .respond(
                    ctx.http.clone(),
                    CreateGenericResponse::new().content(
                        "You must have `Manage Messages` or have a role capable of sending reminders to that channel. \
                         Please talk to your server admin, and ask them to use the `/restrict` command to specify \
                         allowed roles.",
                    ),
                )
                .await;
        }
    }
}

enum PermissionCheck {
    None,              // No permissions
    Basic(bool, bool), // Send + Embed permissions (sufficient to reply)
    All,               // Above + Manage Webhooks (sufficient to operate)
}

#[async_trait]
impl Framework for RegexFramework {
    async fn dispatch(&self, ctx: Context, msg: Message) {
        async fn check_self_permissions(
            ctx: &Context,
            guild: &Guild,
            channel: &GuildChannel,
        ) -> SerenityResult<PermissionCheck> {
            let user_id = ctx.cache.current_user_id();

            let guild_perms = guild.member_permissions(&ctx, user_id).await?;
            let channel_perms = channel.permissions_for_user(ctx, user_id)?;

            let basic_perms = channel_perms.send_messages();

            Ok(if basic_perms && guild_perms.manage_webhooks() && channel_perms.embed_links() {
                PermissionCheck::All
            } else if basic_perms {
                PermissionCheck::Basic(guild_perms.manage_webhooks(), channel_perms.embed_links())
            } else {
                PermissionCheck::None
            })
        }

        async fn check_prefix(ctx: &Context, guild: &Guild, prefix_opt: Option<Match<'_>>) -> bool {
            if let Some(prefix) = prefix_opt {
                let guild_prefix = ctx.prefix(Some(guild.id)).await;

                guild_prefix.as_str() == prefix.as_str()
            } else {
                true
            }
        }

        // gate to prevent analysing messages unnecessarily
        if (msg.author.bot && self.ignore_bots) || msg.content.is_empty() {
        } else {
            // Guild Command
            if let (Some(guild), Ok(Channel::Guild(channel))) =
                (msg.guild(&ctx), msg.channel(&ctx).await)
            {
                let data = ctx.data.read().await;

                let pool = data.get::<SQLPool>().cloned().expect("Could not get SQLPool from data");

                if let Some(full_match) = self.command_matcher.captures(&msg.content) {
                    if check_prefix(&ctx, &guild, full_match.name("prefix")).await {
                        match check_self_permissions(&ctx, &guild, &channel).await {
                            Ok(perms) => match perms {
                                PermissionCheck::All => {
                                    let command = self
                                        .commands_map
                                        .get(
                                            &full_match
                                                .name("cmd")
                                                .unwrap()
                                                .as_str()
                                                .to_lowercase(),
                                        )
                                        .unwrap();

                                    let channel = msg.channel(&ctx).await.unwrap();

                                    let channel_data =
                                        ChannelData::from_channel(&channel, &pool).await.unwrap();

                                    if !command.can_blacklist || !channel_data.blacklisted {
                                        let args = full_match
                                            .name("args")
                                            .map(|m| m.as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        let member = guild.member(&ctx, &msg.author).await.unwrap();

                                        if command.check_permissions(&ctx, &guild, &member).await {
                                            if msg.id == MessageId(0)
                                                || !ctx.check_executing(msg.author.id).await
                                            {
                                                ctx.set_executing(msg.author.id).await;

                                                match command.fun {
                                                    CommandFnType::Text(t) => t(&ctx, &msg, args),
                                                    CommandFnType::Multi(m) => m(&ctx, &msg),
                                                    _ => (self.default_text_fun)(&ctx, &msg, args),
                                                }
                                                .await;

                                                ctx.drop_executing(msg.author.id).await;
                                            }
                                        } else if command.required_permissions
                                            == PermissionLevel::Restricted
                                        {
                                            let _ = msg
                                                .channel_id
                                                .say(
                                                    &ctx,
                                                    "You must have the `Manage Server` permission to use this command.",
                                                )
                                                .await;
                                        } else if command.required_permissions
                                            == PermissionLevel::Managed
                                        {
                                            let _ = msg
                                                .channel_id
                                                .say(
                                                    &ctx,
                                                    "You must have `Manage Messages` or have a role capable of \
                                                     sending reminders to that channel. Please talk to your server \
                                                     admin, and ask them to use the `/restrict` command to specify \
                                                     allowed roles.",
                                                )
                                                .await;
                                        }
                                    }
                                }

                                PermissionCheck::Basic(manage_webhooks, embed_links) => {
                                    let _ = msg
                                        .channel_id
                                        .say(
                                            &ctx,
                                            format!(
                                                "Please ensure the bot has the correct permissions:

✅     **Send Message**
{}     **Embed Links**
{}     **Manage Webhooks**",
                                                if manage_webhooks { "✅" } else { "❌" },
                                                if embed_links { "✅" } else { "❌" },
                                            ),
                                        )
                                        .await;
                                }

                                PermissionCheck::None => {
                                    warn!("Missing enough permissions for guild {}", guild.id);
                                }
                            },

                            Err(e) => {
                                error!(
                                    "Error occurred getting permissions in guild {}: {:?}",
                                    guild.id, e
                                );
                            }
                        }
                    }
                }
            }
            // DM Command
            else if self.dm_enabled {
                if let Some(full_match) = self.dm_regex_matcher.captures(&msg.content[..]) {
                    let command = self
                        .commands_map
                        .get(&full_match.name("cmd").unwrap().as_str().to_lowercase())
                        .unwrap();
                    let args =
                        full_match.name("args").map(|m| m.as_str()).unwrap_or("").to_string();

                    if msg.id == MessageId(0) || !ctx.check_executing(msg.author.id).await {
                        ctx.set_executing(msg.author.id).await;

                        match command.fun {
                            CommandFnType::Text(t) => t(&ctx, &msg, args),
                            CommandFnType::Multi(m) => m(&ctx, &msg),
                            _ => (self.default_text_fun)(&ctx, &msg, args),
                        }
                        .await;

                        ctx.drop_executing(msg.author.id).await;
                    }
                }
            }
        }
    }
}
