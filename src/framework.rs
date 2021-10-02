// todo move framework to its own module, split out permission checks

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use log::info;
use regex::{Match, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    builder::{CreateApplicationCommands, CreateComponents, CreateEmbed},
    cache::Cache,
    client::Context,
    framework::Framework,
    futures::prelude::future::BoxFuture,
    http::{CacheHttp, Http},
    model::{
        channel::Message,
        guild::{Guild, Member},
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
            },
            InteractionResponseType,
        },
        prelude::application_command::ApplicationCommandInteractionDataOption,
    },
    prelude::TypeMapKey,
    Result as SerenityResult,
};

use crate::{models::CtxData, LimitExecutors};

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

enum InvokeModel {
    Slash(ApplicationCommandInteraction),
    Text(Message),
}

pub struct CommandInvoke {
    model: InvokeModel,
    already_responded: bool,
}

impl CommandInvoke {
    fn slash(interaction: ApplicationCommandInteraction) -> Self {
        Self { model: InvokeModel::Slash(interaction), already_responded: false }
    }

    fn msg(msg: Message) -> Self {
        Self { model: InvokeModel::Text(msg), already_responded: false }
    }

    pub fn interaction(self) -> Option<ApplicationCommandInteraction> {
        match self.model {
            InvokeModel::Slash(i) => Some(i),
            InvokeModel::Text(_) => None,
        }
    }

    pub fn channel_id(&self) -> ChannelId {
        match &self.model {
            InvokeModel::Slash(i) => i.channel_id,
            InvokeModel::Text(m) => m.channel_id,
        }
    }

    pub fn guild_id(&self) -> Option<GuildId> {
        match &self.model {
            InvokeModel::Slash(i) => i.guild_id,
            InvokeModel::Text(m) => m.guild_id,
        }
    }

    pub fn guild(&self, cache: impl AsRef<Cache>) -> Option<Guild> {
        self.guild_id().map(|id| id.to_guild_cached(cache)).flatten()
    }

    pub fn author_id(&self) -> UserId {
        match &self.model {
            InvokeModel::Slash(i) => i.user.id,
            InvokeModel::Text(m) => m.author.id,
        }
    }

    pub async fn member(&self, cache_http: impl CacheHttp) -> Option<Member> {
        match &self.model {
            InvokeModel::Slash(i) => i.member.clone(),
            InvokeModel::Text(m) => m.member(cache_http).await.ok(),
        }
    }

    pub async fn respond(
        &self,
        http: impl AsRef<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        match &self.model {
            InvokeModel::Slash(i) => {
                if !self.already_responded {
                    i.create_interaction_response(http, |r| {
                        r.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                d.content(generic_response.content);

                                if let Some(embed) = generic_response.embed {
                                    d.add_embed(embed);
                                }

                                if let Some(components) = generic_response.components {
                                    d.components(|c| {
                                        *c = components;
                                        c
                                    });
                                }

                                d
                            })
                    })
                    .await
                    .map(|_| ())
                } else {
                    i.create_followup_message(http, |d| {
                        d.content(generic_response.content);

                        if let Some(embed) = generic_response.embed {
                            d.add_embed(embed);
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
            InvokeModel::Text(m) => m
                .channel_id
                .send_message(http, |m| {
                    m.content(generic_response.content);

                    if let Some(embed) = generic_response.embed {
                        m.set_embed(embed);
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
                .map(|_| ()),
        }
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandOptions {
    pub command: &'static str,
    pub subcommand: Option<String>,
    pub subcommand_group: Option<String>,
    pub options: HashMap<String, OptionValue>,
}

impl CommandOptions {
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.options.get(key)
    }
}

impl CommandOptions {
    fn new(command: &'static Command) -> Self {
        Self {
            command: command.names[0],
            subcommand: None,
            subcommand_group: None,
            options: Default::default(),
        }
    }

    fn populate(mut self, interaction: &ApplicationCommandInteraction) -> Self {
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

        for option in &interaction.data.options {
            match_option(option.clone(), &mut self)
        }

        self
    }
}

pub enum HookResult {
    Continue,
    Halt,
}

type SlashCommandFn =
    for<'fut> fn(&'fut Context, CommandInvoke, CommandOptions) -> BoxFuture<'fut, ()>;

type TextCommandFn = for<'fut> fn(&'fut Context, CommandInvoke, String) -> BoxFuture<'fut, ()>;

type MultiCommandFn = for<'fut> fn(&'fut Context, CommandInvoke) -> BoxFuture<'fut, ()>;

pub type HookFn = for<'fut> fn(
    &'fut Context,
    &'fut CommandInvoke,
    &'fut CommandOptions,
) -> BoxFuture<'fut, HookResult>;

pub enum CommandFnType {
    Slash(SlashCommandFn),
    Text(TextCommandFn),
    Multi(MultiCommandFn),
}

impl CommandFnType {
    pub fn is_slash(&self) -> bool {
        !matches!(self, CommandFnType::Text(_))
    }
}

pub struct Hook {
    pub fun: HookFn,
    pub uuid: u128,
}

impl PartialEq for Hook {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

pub struct Command {
    pub fun: CommandFnType,

    pub names: &'static [&'static str],

    pub desc: &'static str,
    pub examples: &'static [&'static str],
    pub group: &'static str,

    pub args: &'static [&'static Arg],

    pub can_blacklist: bool,
    pub supports_dm: bool,

    pub hooks: &'static [&'static Hook],
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
    debug_guild: Option<GuildId>,
    hooks: Vec<&'static Hook>,
}

impl TypeMapKey for RegexFramework {
    type Value = Arc<RegexFramework>;
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
            debug_guild: None,
            hooks: vec![],
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

    pub fn add_hook(mut self, fun: &'static Hook) -> Self {
        self.hooks.push(fun);

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
                                        .required(option.required);

                                    for sub_option in option.options {
                                        s.create_sub_option(|ss| {
                                            ss.name(sub_option.name)
                                                .description(sub_option.description)
                                                .kind(sub_option.kind)
                                                .required(sub_option.required)
                                        });
                                    }

                                    s
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
                .await
                .unwrap();
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

        let args = CommandOptions::new(command).populate(&interaction);
        let command_invoke = CommandInvoke::slash(interaction);

        for hook in command.hooks {
            match (hook.fun)(&ctx, &command_invoke, &args).await {
                HookResult::Continue => {}
                HookResult::Halt => {
                    return;
                }
            }
        }

        for hook in &self.hooks {
            match (hook.fun)(&ctx, &command_invoke, &args).await {
                HookResult::Continue => {}
                HookResult::Halt => {
                    return;
                }
            }
        }

        let user_id = command_invoke.author_id();

        if !ctx.check_executing(user_id).await {
            ctx.set_executing(user_id).await;

            match command.fun {
                CommandFnType::Slash(t) => t(&ctx, command_invoke, args).await,
                CommandFnType::Multi(m) => m(&ctx, command_invoke).await,
                _ => (),
            }

            ctx.drop_executing(user_id).await;
        }
    }
}

#[async_trait]
impl Framework for RegexFramework {
    async fn dispatch(&self, ctx: Context, msg: Message) {
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
            return;
        }

        let user_id = msg.author.id;
        let invoke = CommandInvoke::msg(msg.clone());

        // Guild Command
        if let Some(guild) = msg.guild(&ctx) {
            if let Some(full_match) = self.command_matcher.captures(&msg.content) {
                if check_prefix(&ctx, &guild, full_match.name("prefix")).await {
                    let command = self
                        .commands_map
                        .get(&full_match.name("cmd").unwrap().as_str().to_lowercase())
                        .unwrap();

                    let channel_data = ctx.channel_data(invoke.channel_id()).await.unwrap();

                    if !command.can_blacklist || !channel_data.blacklisted {
                        let args =
                            full_match.name("args").map(|m| m.as_str()).unwrap_or("").to_string();

                        if msg.id == MessageId(0) || !ctx.check_executing(user_id).await {
                            ctx.set_executing(user_id).await;

                            match command.fun {
                                CommandFnType::Text(t) => t(&ctx, invoke, args).await,
                                CommandFnType::Multi(m) => m(&ctx, invoke).await,
                                _ => {}
                            };

                            ctx.drop_executing(user_id).await;
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
                let args = full_match.name("args").map(|m| m.as_str()).unwrap_or("").to_string();

                let user_id = invoke.author_id();

                if msg.id == MessageId(0) || !ctx.check_executing(user_id).await {
                    ctx.set_executing(user_id).await;

                    match command.fun {
                        CommandFnType::Text(t) => t(&ctx, invoke, args).await,
                        CommandFnType::Multi(m) => m(&ctx, invoke).await,
                        _ => {}
                    };

                    ctx.drop_executing(user_id).await;
                }
            }
        }
    }
}
