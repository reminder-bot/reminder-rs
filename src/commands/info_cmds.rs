use regex_command_attr::command;

use serenity::{
    builder::CreateEmbedFooter,
    client::Context,
    model::{
        channel::Message,
        interactions::{Interaction, InteractionResponseType},
    },
};

use chrono::offset::Utc;

use crate::{
    command_help, consts::DEFAULT_PREFIX, get_ctx_data, language_manager::LanguageManager,
    models::CtxGuildData, models::UserData, FrameworkCtx, THEME_COLOR,
};

use inflector::Inflector;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[command]
#[can_blacklist(false)]
async fn ping(ctx: &Context, msg: &Message, _args: String) {
    let now = SystemTime::now();
    let since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Time calculated as going backwards. Very bad");

    let delta = since_epoch.as_millis() as i64 - msg.timestamp.timestamp_millis();

    let _ = msg
        .channel_id
        .say(&ctx, format!("Time taken to receive message: {}ms", delta))
        .await;
}

async fn footer(ctx: &Context) -> impl FnOnce(&mut CreateEmbedFooter) -> &mut CreateEmbedFooter {
    let shard_count = ctx.cache.shard_count().await;
    let shard = ctx.shard_id;

    move |f| {
        f.text(format!(
            "{}\nshard {} of {}",
            concat!(env!("CARGO_PKG_NAME"), " ver ", env!("CARGO_PKG_VERSION")),
            shard,
            shard_count,
        ))
    }
}

#[command]
#[can_blacklist(false)]
async fn help(ctx: &Context, msg: &Message, args: String) {
    async fn default_help(
        ctx: &Context,
        msg: &Message,
        lm: Arc<LanguageManager>,
        prefix: &str,
        language: &str,
    ) {
        let desc = lm.get(language, "help/desc").replace("{prefix}", prefix);
        let footer = footer(ctx).await;

        let _ = msg
            .channel_id
            .send_message(ctx, |m| {
                m.embed(move |e| {
                    e.title("Help Menu")
                        .description(desc)
                        .field(
                            lm.get(language, "help/setup_title"),
                            "`lang` `timezone` `meridian`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/mod_title"),
                            "`prefix` `blacklist` `restrict` `alias`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/reminder_title"),
                            "`remind` `interval` `natural` `look` `countdown`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/reminder_mod_title"),
                            "`del` `offset` `pause` `nudge`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/info_title"),
                            "`help` `info` `donate` `clock`",
                            true,
                        )
                        .field(
                            lm.get(language, "help/todo_title"),
                            "`todo` `todos` `todoc`",
                            true,
                        )
                        .field(lm.get(language, "help/other_title"), "`timer`", true)
                        .footer(footer)
                        .color(*THEME_COLOR)
                })
            })
            .await;
    }

    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool);
    let prefix = ctx.prefix(msg.guild_id);

    if !args.is_empty() {
        let framework = ctx
            .data
            .read()
            .await
            .get::<FrameworkCtx>()
            .cloned()
            .expect("Could not get FrameworkCtx from data");

        let matched = framework
            .commands
            .get(args.as_str())
            .map(|inner| inner.name);

        if let Some(command_name) = matched {
            command_help(ctx, msg, lm, &prefix.await, &language.await, command_name).await
        } else {
            default_help(ctx, msg, lm, &prefix.await, &language.await).await;
        }
    } else {
        default_help(ctx, msg, lm, &prefix.await, &language.await).await;
    }
}

pub async fn help_interaction(ctx: &Context, interaction: Interaction) {
    async fn default_help(
        ctx: &Context,
        interaction: Interaction,
        lm: Arc<LanguageManager>,
        language: &str,
    ) {
        let desc = lm.get(language, "help/desc").replace("{prefix}", "/");
        let footer = footer(ctx).await;

        interaction
            .create_interaction_response(ctx, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|data| {
                        data.embed(move |e| {
                            e.title("Help Menu")
                                .description(desc)
                                .field(
                                    lm.get(language, "help/setup_title"),
                                    "`lang` `timezone` `meridian`",
                                    true,
                                )
                                .field(
                                    lm.get(language, "help/mod_title"),
                                    "`prefix` `blacklist` `restrict` `alias`",
                                    true,
                                )
                                .field(
                                    lm.get(language, "help/reminder_title"),
                                    "`remind` `interval` `natural` `look` `countdown`",
                                    true,
                                )
                                .field(
                                    lm.get(language, "help/reminder_mod_title"),
                                    "`del` `offset` `pause` `nudge`",
                                    true,
                                )
                                .field(
                                    lm.get(language, "help/info_title"),
                                    "`help` `info` `donate` `clock`",
                                    true,
                                )
                                .field(
                                    lm.get(language, "help/todo_title"),
                                    "`todo` `todos` `todoc`",
                                    true,
                                )
                                .field(lm.get(language, "help/other_title"), "`timer`", true)
                                .footer(footer)
                                .color(*THEME_COLOR)
                        })
                    })
            })
            .await
            .unwrap();
    }

    async fn command_help(
        ctx: &Context,
        interaction: Interaction,
        lm: Arc<LanguageManager>,
        language: &str,
        command_name: &str,
    ) {
        interaction
            .create_interaction_response(ctx, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|data| {
                        data.embed(move |e| {
                            e.title(format!("{} Help", command_name.to_title_case()))
                                .description(
                                    lm.get(&language, &format!("help/{}", command_name))
                                        .replace("{prefix}", "/"),
                                )
                                .footer(|f| {
                                    f.text(concat!(
                                        env!("CARGO_PKG_NAME"),
                                        " ver ",
                                        env!("CARGO_PKG_VERSION")
                                    ))
                                })
                                .color(*THEME_COLOR)
                        })
                    })
            })
            .await
            .unwrap();
    }

    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(interaction.member.user.id, &pool);

    if let Some(data) = &interaction.data {
        if let Some(command_name) = data
            .options
            .first()
            .map(|opt| {
                opt.value
                    .clone()
                    .map(|inner| inner.as_str().unwrap().to_string())
            })
            .flatten()
        {
            let framework = ctx
                .data
                .read()
                .await
                .get::<FrameworkCtx>()
                .cloned()
                .expect("Could not get FrameworkCtx from data");

            let matched = framework
                .commands
                .get(&command_name)
                .map(|inner| inner.name);

            if let Some(command_name) = matched {
                command_help(ctx, interaction, lm, &language.await, command_name).await
            } else {
                default_help(ctx, interaction, lm, &language.await).await;
            }
        } else {
            default_help(ctx, interaction, lm, &language.await).await;
        }
    } else {
        default_help(ctx, interaction, lm, &language.await).await;
    }
}

#[command]
async fn info(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool);
    let prefix = ctx.prefix(msg.guild_id);
    let current_user = ctx.cache.current_user();
    let footer = footer(ctx).await;

    let desc = lm
        .get(&language.await, "info")
        .replacen("{user}", &current_user.await.name, 1)
        .replace("{default_prefix}", &*DEFAULT_PREFIX)
        .replace("{prefix}", &prefix.await);

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Info")
                    .description(desc)
                    .footer(footer)
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

pub async fn info_interaction(ctx: &Context, interaction: Interaction) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&interaction.member, &pool);
    let current_user = ctx.cache.current_user();
    let footer = footer(ctx).await;

    let desc = lm
        .get(&language.await, "info")
        .replacen("{user}", &current_user.await.name, 1)
        .replace("{default_prefix}", &*DEFAULT_PREFIX)
        .replace("{prefix}", "/");

    interaction
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| {
                    data.embed(move |e| {
                        e.title("Info")
                            .description(desc)
                            .footer(footer)
                            .color(*THEME_COLOR)
                    })
                })
        })
        .await
        .unwrap();
}

#[command]
async fn donate(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let desc = lm.get(&language, "donate");
    let footer = footer(ctx).await;

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Donate")
                    .description(desc)
                    .footer(footer)
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

pub async fn donate_interaction(ctx: &Context, interaction: Interaction) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&interaction.member, &pool).await;
    let desc = lm.get(&language, "donate");
    let footer = footer(ctx).await;

    interaction
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| {
                    data.embed(move |e| {
                        e.title("Donate")
                            .description(desc)
                            .footer(footer)
                            .color(*THEME_COLOR)
                    })
                })
        })
        .await
        .unwrap();
}

#[command]
async fn dashboard(ctx: &Context, msg: &Message, _args: String) {
    let footer = footer(ctx).await;

    let _ = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(move |e| {
                e.title("Dashboard")
                    .description("https://reminder-bot.com/dashboard")
                    .footer(footer)
                    .color(*THEME_COLOR)
            })
        })
        .await;
}

#[command]
async fn clock(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;
    let meridian = UserData::meridian_of(&msg.author, &pool).await;

    let now = Utc::now().with_timezone(&timezone);

    let clock_display = lm.get(&language, "clock/time");

    let _ = msg
        .channel_id
        .say(
            &ctx,
            clock_display.replacen("{}", &now.format(meridian.fmt_str()).to_string(), 1),
        )
        .await;
}

pub async fn clock_interaction(ctx: &Context, interaction: Interaction) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&interaction.member, &pool).await;
    let timezone = UserData::timezone_of(&interaction.member, &pool).await;
    let meridian = UserData::meridian_of(&interaction.member, &pool).await;

    let now = Utc::now().with_timezone(&timezone);

    let clock_display = lm.get(&language, "clock/time");

    interaction
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| {
                    data.content(clock_display.replacen(
                        "{}",
                        &now.format(meridian.fmt_str()).to_string(),
                        1,
                    ))
                })
        })
        .await
        .unwrap();
}
