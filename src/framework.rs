// todo move framework to its own module, split out permission checks

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use log::info;
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    builder::{CreateApplicationCommands, CreateComponents, CreateEmbed},
    cache::Cache,
    client::Context,
    framework::Framework,
    futures::prelude::future::BoxFuture,
    http::Http,
    model::{
        channel::Message,
        guild::{Guild, Member},
        id::{ChannelId, GuildId, RoleId, UserId},
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
            },
            message_component::MessageComponentInteraction,
            InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
        },
        prelude::application_command::ApplicationCommandInteractionDataOption,
    },
    prelude::TypeMapKey,
    Result as SerenityResult,
};

use crate::SQLPool;

pub struct CreateGenericResponse {
    content: String,
    embed: Option<CreateEmbed>,
    components: Option<CreateComponents>,
    flags: InteractionApplicationCommandCallbackDataFlags,
}

impl CreateGenericResponse {
    pub fn new() -> Self {
        Self {
            content: "".to_string(),
            embed: None,
            components: None,
            flags: InteractionApplicationCommandCallbackDataFlags::empty(),
        }
    }

    pub fn ephemeral(mut self) -> Self {
        self.flags.insert(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);

        self
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

#[derive(Clone)]
enum InvokeModel {
    Slash(ApplicationCommandInteraction),
    Component(MessageComponentInteraction),
}

#[derive(Clone)]
pub struct CommandInvoke {
    model: InvokeModel,
    already_responded: bool,
    deferred: bool,
}

impl CommandInvoke {
    pub fn component(component: MessageComponentInteraction) -> Self {
        Self { model: InvokeModel::Component(component), already_responded: false, deferred: false }
    }

    fn slash(interaction: ApplicationCommandInteraction) -> Self {
        Self { model: InvokeModel::Slash(interaction), already_responded: false, deferred: false }
    }

    pub async fn defer(&mut self, http: impl AsRef<Http>) {
        if !self.deferred {
            match &self.model {
                InvokeModel::Slash(i) => {
                    i.create_interaction_response(http, |r| {
                        r.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                    })
                    .await
                    .unwrap();

                    self.deferred = true;
                }
                InvokeModel::Component(i) => {
                    i.create_interaction_response(http, |r| {
                        r.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                    })
                    .await
                    .unwrap();

                    self.deferred = true;
                }
            }
        }
    }

    pub fn channel_id(&self) -> ChannelId {
        match &self.model {
            InvokeModel::Slash(i) => i.channel_id,
            InvokeModel::Component(i) => i.channel_id,
        }
    }

    pub fn guild_id(&self) -> Option<GuildId> {
        match &self.model {
            InvokeModel::Slash(i) => i.guild_id,
            InvokeModel::Component(i) => i.guild_id,
        }
    }

    pub fn guild(&self, cache: impl AsRef<Cache>) -> Option<Guild> {
        self.guild_id().map(|id| id.to_guild_cached(cache)).flatten()
    }

    pub fn author_id(&self) -> UserId {
        match &self.model {
            InvokeModel::Slash(i) => i.user.id,
            InvokeModel::Component(i) => i.user.id,
        }
    }

    pub fn member(&self) -> Option<Member> {
        match &self.model {
            InvokeModel::Slash(i) => i.member.clone(),
            InvokeModel::Component(i) => i.member.clone(),
        }
    }

    pub async fn respond(
        &mut self,
        http: impl AsRef<Http>,
        generic_response: CreateGenericResponse,
    ) -> SerenityResult<()> {
        match &self.model {
            InvokeModel::Slash(i) => {
                if self.already_responded {
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
                } else if self.deferred {
                    i.edit_original_interaction_response(http, |d| {
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
                } else {
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
                }
            }
            InvokeModel::Component(i) => i
                .create_interaction_response(http, |r| {
                    r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|d| {
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
                .map(|_| ()),
        }?;

        self.already_responded = true;

        Ok(())
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

impl CommandOptions {
    fn new(command: &'static Command) -> Self {
        Self {
            command: command.names[0].to_string(),
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
    for<'fut> fn(&'fut Context, &'fut mut CommandInvoke, CommandOptions) -> BoxFuture<'fut, ()>;

type MultiCommandFn = for<'fut> fn(&'fut Context, &'fut mut CommandInvoke) -> BoxFuture<'fut, ()>;

pub type HookFn = for<'fut> fn(
    &'fut Context,
    &'fut mut CommandInvoke,
    &'fut CommandOptions,
) -> BoxFuture<'fut, HookResult>;

pub enum CommandFnType {
    Slash(SlashCommandFn),
    Multi(MultiCommandFn),
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
    ignore_bots: bool,
    dm_enabled: bool,
    debug_guild: Option<GuildId>,
    hooks: Vec<&'static Hook>,
}

impl TypeMapKey for RegexFramework {
    type Value = Arc<RegexFramework>;
}

impl RegexFramework {
    pub fn new() -> Self {
        Self {
            commands_map: HashMap::new(),
            commands: HashSet::new(),
            ignore_bots: true,
            dm_enabled: true,
            debug_guild: None,
            hooks: vec![],
        }
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

    fn _populate_commands<'a>(
        &self,
        commands: &'a mut CreateApplicationCommands,
    ) -> &'a mut CreateApplicationCommands {
        for command in &self.commands {
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
        {
            if let Some(guild_id) = interaction.guild_id {
                let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
                let _ = sqlx::query!("INSERT IGNORE INTO guilds (guild) VALUES (?)", guild_id.0)
                    .execute(&pool)
                    .await;
            }
        }

        let command = {
            self.commands_map
                .get(&interaction.data.name)
                .expect(&format!("Received invalid command: {}", interaction.data.name))
        };

        let args = CommandOptions::new(command).populate(&interaction);
        let mut command_invoke = CommandInvoke::slash(interaction);

        for hook in command.hooks {
            match (hook.fun)(&ctx, &mut command_invoke, &args).await {
                HookResult::Continue => {}
                HookResult::Halt => {
                    return;
                }
            }
        }

        for hook in &self.hooks {
            match (hook.fun)(&ctx, &mut command_invoke, &args).await {
                HookResult::Continue => {}
                HookResult::Halt => {
                    return;
                }
            }
        }

        match command.fun {
            CommandFnType::Slash(t) => t(&ctx, &mut command_invoke, args).await,
            CommandFnType::Multi(m) => m(&ctx, &mut command_invoke).await,
        }
    }

    pub async fn run_command_from_options(
        &self,
        ctx: &Context,
        command_invoke: &mut CommandInvoke,
        command_options: CommandOptions,
    ) {
        let command = {
            self.commands_map
                .get(&command_options.command)
                .expect(&format!("Received invalid command: {}", command_options.command))
        };

        match command.fun {
            CommandFnType::Slash(t) => t(&ctx, command_invoke, command_options).await,
            CommandFnType::Multi(m) => m(&ctx, command_invoke).await,
        }
    }
}

#[async_trait]
impl Framework for RegexFramework {
    async fn dispatch(&self, _ctx: Context, _msg: Message) {}
}
