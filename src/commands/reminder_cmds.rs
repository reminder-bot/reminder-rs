use regex_command_attr::command;

use serenity::{
    client::Context,
    http::CacheHttp,
    model::{
        channel::Message,
        channel::{Channel, GuildChannel},
        guild::Guild,
        id::{ChannelId, GuildId, UserId},
        webhook::Webhook,
    },
    prelude::Mentionable,
    Result as SerenityResult,
};

use crate::{
    check_subscription_on_message, command_help,
    consts::{
        CHARACTERS, MAX_TIME, MIN_INTERVAL, REGEX_CHANNEL_USER, REGEX_CONTENT_SUBSTITUTION,
        REGEX_NATURAL_COMMAND_1, REGEX_NATURAL_COMMAND_2, REGEX_REMIND_COMMAND, THEME_COLOR,
    },
    framework::SendIterator,
    get_ctx_data,
    models::{
        channel_data::ChannelData,
        guild_data::GuildData,
        reminder::{LookFlags, Reminder},
        timer::Timer,
        user_data::UserData,
        CtxGuildData,
    },
    time_parser::{natural_parser, TimeParser},
};

use chrono::NaiveDateTime;

use rand::{rngs::OsRng, seq::IteratorRandom};

use sqlx::MySqlPool;

use num_integer::Integer;

use std::{
    collections::HashSet,
    convert::TryInto,
    default::Default,
    env,
    fmt::Display,
    string::ToString,
    time::{SystemTime, UNIX_EPOCH},
};

use regex::Captures;

async fn create_webhook(
    ctx: impl CacheHttp,
    channel: GuildChannel,
    name: impl Display,
) -> SerenityResult<Webhook> {
    channel
        .create_webhook_with_avatar(
            ctx.http(),
            name,
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/",
                    env!(
                        "WEBHOOK_AVATAR",
                        "WEBHOOK_AVATAR not provided for compilation"
                    )
                )) as &[u8],
                env!("WEBHOOK_AVATAR"),
            ),
        )
        .await
}

#[command]
#[supports_dm(false)]
#[permission_level(Restricted)]
async fn pause(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;

    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();

    if args.is_empty() {
        channel.paused = !channel.paused;
        channel.paused_until = None;

        channel.commit_changes(&pool).await;

        if channel.paused {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "pause/paused_indefinite"))
                .await;
        } else {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&language, "pause/unpaused"))
                .await;
        }
    } else {
        let parser = TimeParser::new(&args, timezone);
        let pause_until = parser.timestamp();

        match pause_until {
            Ok(timestamp) => {
                let dt = NaiveDateTime::from_timestamp(timestamp, 0);

                channel.paused = true;
                channel.paused_until = Some(dt);

                channel.commit_changes(&pool).await;

                let content = lm
                    .get(&language, "pause/paused_until")
                    .replace("{}", &format!("<t:{}:D>", timestamp));

                let _ = msg.channel_id.say(&ctx, content).await;
            }

            Err(_) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "pause/invalid_time"))
                    .await;
            }
        }
    }
}

#[command]
#[permission_level(Restricted)]
async fn offset(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    if args.is_empty() {
        let prefix = ctx.prefix(msg.guild_id).await;

        command_help(ctx, msg, lm, &prefix, &user_data.language, "offset").await;
    } else {
        let parser = TimeParser::new(&args, user_data.timezone());

        if let Ok(displacement) = parser.displacement() {
            if let Some(guild) = msg.guild(&ctx) {
                let guild_data = GuildData::from_guild(guild, &pool).await.unwrap();

                sqlx::query!(
                    "
UPDATE reminders
    INNER JOIN `channels`
        ON `channels`.id = reminders.channel_id
    SET
        reminders.`utc_time` = DATE_ADD(reminders.`utc_time`, INTERVAL ? SECOND)
    WHERE channels.guild_id = ?
                    ",
                    displacement,
                    guild_data.id
                )
                .execute(&pool)
                .await
                .unwrap();
            } else {
                sqlx::query!(
                    "
UPDATE reminders SET `utc_time` = DATE_ADD(`utc_time`, INTERVAL ? SECOND) WHERE reminders.channel_id = ?
                    ",
                    displacement,
                    user_data.dm_channel
                )
                .execute(&pool)
                .await
                .unwrap();
            }

            let response = lm.get(&user_data.language, "offset/success").replacen(
                "{}",
                &displacement.to_string(),
                1,
            );

            let _ = msg.channel_id.say(&ctx, response).await;
        } else {
            let _ = msg
                .channel_id
                .say(&ctx, lm.get(&user_data.language, "offset/invalid_time"))
                .await;
        }
    }
}

#[command]
#[permission_level(Restricted)]
async fn nudge(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;
    let timezone = UserData::timezone_of(&msg.author, &pool).await;

    let mut channel = ChannelData::from_channel(msg.channel(&ctx).await.unwrap(), &pool)
        .await
        .unwrap();

    if args.is_empty() {
        let content = lm
            .get(&language, "nudge/no_argument")
            .replace("{nudge}", &format!("{}s", &channel.nudge.to_string()));

        let _ = msg.channel_id.say(&ctx, content).await;
    } else {
        let parser = TimeParser::new(&args, timezone);
        let nudge_time = parser.displacement();

        match nudge_time {
            Ok(displacement) => {
                if displacement < i16::MIN as i64 || displacement > i16::MAX as i64 {
                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "nudge/invalid_time"))
                        .await;
                } else {
                    channel.nudge = displacement as i16;

                    channel.commit_changes(&pool).await;

                    let response = lm.get(&language, "nudge/success").replacen(
                        "{}",
                        &displacement.to_string(),
                        1,
                    );

                    let _ = msg.channel_id.say(&ctx, response).await;
                }
            }

            Err(_) => {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "nudge/invalid_time"))
                    .await;
            }
        }
    }
}

#[command("look")]
#[permission_level(Managed)]
async fn look(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let flags = LookFlags::from_string(&args);

    let channel_opt = msg.channel_id.to_channel_cached(&ctx);

    let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
        if Some(channel.guild_id) == msg.guild_id {
            flags.channel_id.unwrap_or(msg.channel_id)
        } else {
            msg.channel_id
        }
    } else {
        msg.channel_id
    };

    let reminders = Reminder::from_channel(ctx, channel_id, &flags).await;

    if reminders.is_empty() {
        let _ = msg
            .channel_id
            .say(&ctx, lm.get(&language, "look/no_reminders"))
            .await;
    } else {
        let inter = lm.get(&language, "look/inter");

        let display = reminders
            .iter()
            .map(|reminder| reminder.display(&flags, &inter));

        let _ = msg.channel_id.say_lines(&ctx, display).await;
    }
}

#[command("del")]
#[permission_level(Managed)]
async fn delete(ctx: &Context, msg: &Message, _args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    let _ = msg
        .channel_id
        .say(&ctx, lm.get(&user_data.language, "del/listing"))
        .await;

    let mut reminder_ids: Vec<u32> = vec![];

    let reminders = Reminder::from_guild(ctx, msg.guild_id, msg.author.id).await;

    let enumerated_reminders = reminders.iter().enumerate().map(|(count, reminder)| {
        reminder_ids.push(reminder.id);

        format!(
            "**{}**: '{}' *<#{}>* at <t:{}>",
            count + 1,
            reminder.display_content(),
            reminder.channel,
            reminder.utc_time.timestamp()
        )
    });

    let _ = msg.channel_id.say_lines(&ctx, enumerated_reminders).await;
    let _ = msg
        .channel_id
        .say(&ctx, lm.get(&user_data.language, "del/listed"))
        .await;

    let reply = msg
        .channel_id
        .await_reply(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .await;

    if let Some(content) = reply.map(|m| m.content.replace(",", " ")) {
        let parts = content
            .split(' ')
            .filter(|i| !i.is_empty())
            .collect::<Vec<&str>>();

        let valid_parts = parts
            .iter()
            .filter_map(|i| {
                i.parse::<usize>()
                    .ok()
                    .filter(|val| val > &0)
                    .map(|val| reminder_ids.get(val - 1))
                    .flatten()
            })
            .map(|item| item.to_string())
            .collect::<Vec<String>>();

        if parts.len() == valid_parts.len() {
            let joined = valid_parts.join(",");

            let count_row = sqlx::query!(
                "
SELECT COUNT(1) AS count FROM reminders WHERE FIND_IN_SET(id, ?)
                ",
                joined
            )
            .fetch_one(&pool)
            .await
            .unwrap();

            sqlx::query!(
                "
DELETE FROM reminders WHERE FIND_IN_SET(id, ?)
                ",
                joined
            )
            .execute(&pool)
            .await
            .unwrap();

            if let Some(guild_id) = msg.guild_id {
                let _ = sqlx::query!(
                    "
INSERT INTO events (event_name, bulk_count, guild_id, user_id) VALUES ('delete', ?, ?, ?)
                    ",
                    count_row.count,
                    guild_id.as_u64(),
                    user_data.id
                )
                .execute(&pool)
                .await;
            }

            let content = lm.get(&user_data.language, "del/count").replacen(
                "{}",
                &count_row.count.to_string(),
                1,
            );

            let _ = msg.channel_id.say(&ctx, content).await;
        } else {
            let content = lm
                .get(&user_data.language, "del/count")
                .replacen("{}", "0", 1);

            let _ = msg.channel_id.say(&ctx, content).await;
        }
    }
}

#[command("timer")]
#[permission_level(Managed)]
async fn timer(ctx: &Context, msg: &Message, args: String) {
    fn time_difference(start_time: NaiveDateTime) -> String {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let now = NaiveDateTime::from_timestamp(unix_time, 0);

        let delta = (now - start_time).num_seconds();

        let (minutes, seconds) = delta.div_rem(&60);
        let (hours, minutes) = minutes.div_rem(&60);
        let (days, hours) = hours.div_rem(&24);

        format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    }

    let (pool, lm) = get_ctx_data(&ctx).await;

    let language = UserData::language_of(&msg.author, &pool).await;

    let mut args_iter = args.splitn(2, ' ');

    let owner = msg
        .guild_id
        .map(|g| g.as_u64().to_owned())
        .unwrap_or_else(|| msg.author.id.as_u64().to_owned());

    match args_iter.next() {
        Some("list") => {
            let timers = Timer::from_owner(owner, &pool).await;

            let _ = msg
                .channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.fields(timers.iter().map(|timer| {
                            (
                                &timer.name,
                                format!("⏳ `{}`", time_difference(timer.start_time)),
                                false,
                            )
                        }))
                    })
                })
                .await;
        }

        Some("start") => {
            let count = Timer::count_from_owner(owner, &pool).await;

            if count >= 25 {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "timer/limit"))
                    .await;
            } else {
                let name = args_iter
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or(format!("New timer #{}", count + 1));

                if name.len() <= 32 {
                    Timer::create(&name, owner, &pool).await;

                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/success"))
                        .await;
                } else {
                    let _ = msg
                        .channel_id
                        .say(
                            &ctx,
                            lm.get(&language, "timer/name_length")
                                .replace("{}", &name.len().to_string()),
                        )
                        .await;
                }
            }
        }

        Some("delete") => {
            if let Some(name) = args_iter.next() {
                let exists = sqlx::query!(
                    "
SELECT 1 as _r FROM timers WHERE owner = ? AND name = ?
                    ",
                    owner,
                    name
                )
                .fetch_one(&pool)
                .await;

                if exists.is_ok() {
                    sqlx::query!(
                        "
DELETE FROM timers WHERE owner = ? AND name = ?
                        ",
                        owner,
                        name
                    )
                    .execute(&pool)
                    .await
                    .unwrap();

                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/deleted"))
                        .await;
                } else {
                    let _ = msg
                        .channel_id
                        .say(&ctx, lm.get(&language, "timer/not_found"))
                        .await;
                }
            } else {
                let _ = msg
                    .channel_id
                    .say(&ctx, lm.get(&language, "timer/help"))
                    .await;
            }
        }

        _ => {
            let prefix = ctx.prefix(msg.guild_id).await;

            command_help(ctx, msg, lm, &prefix, &language, "timer").await;
        }
    }
}

#[derive(PartialEq)]
enum RemindCommand {
    Remind,
    Interval,
}

enum ReminderScope {
    User(u64),
    Channel(u64),
}

impl ReminderScope {
    fn mention(&self) -> String {
        match self {
            Self::User(id) => format!("<@{}>", id),
            Self::Channel(id) => format!("<#{}>", id),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
enum ReminderError {
    LongInterval,
    PastTime,
    ShortInterval,
    InvalidTag,
    InvalidTime,
    InvalidExpiration,
    DiscordError(String),
}

impl std::fmt::Display for ReminderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_response())
    }
}

impl std::error::Error for ReminderError {}

trait ToResponse {
    fn to_response(&self) -> &'static str;

    fn to_response_natural(&self) -> &'static str;
}

impl ToResponse for ReminderError {
    fn to_response(&self) -> &'static str {
        match self {
            Self::LongInterval => "interval/long_interval",
            Self::PastTime => "remind/past_time",
            Self::ShortInterval => "interval/short_interval",
            Self::InvalidTag => "remind/invalid_tag",
            Self::InvalidTime => "remind/invalid_time",
            Self::InvalidExpiration => "interval/invalid_expiration",
            Self::DiscordError(_) => "remind/generic_error",
        }
    }

    fn to_response_natural(&self) -> &'static str {
        match self {
            Self::InvalidTime => "natural/invalid_time",
            _ => self.to_response(),
        }
    }
}

impl<T> ToResponse for Result<T, ReminderError> {
    fn to_response(&self) -> &'static str {
        match self {
            Ok(_) => "remind/success",

            Err(reminder_error) => reminder_error.to_response(),
        }
    }

    fn to_response_natural(&self) -> &'static str {
        match self {
            Ok(_) => "remind/success",

            Err(reminder_error) => reminder_error.to_response_natural(),
        }
    }
}

fn generate_uid() -> String {
    let mut generator: OsRng = Default::default();

    (0..64)
        .map(|_| {
            CHARACTERS
                .chars()
                .choose(&mut generator)
                .unwrap()
                .to_owned()
                .to_string()
        })
        .collect::<Vec<String>>()
        .join("")
}

#[derive(Debug)]
enum ContentError {
    TooManyAttachments,
    AttachmentTooLarge,
    AttachmentDownloadFailed,
}

impl ContentError {
    fn to_response(&self) -> &'static str {
        match self {
            ContentError::TooManyAttachments => "remind/too_many_attachments",
            ContentError::AttachmentTooLarge => "remind/attachment_too_large",
            ContentError::AttachmentDownloadFailed => "remind/attachment_download_failed",
        }
    }
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ContentError {}

struct Content {
    content: String,
    tts: bool,
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
}

impl Content {
    async fn build<S: ToString>(content: S, message: &Message) -> Result<Self, ContentError> {
        if message.attachments.len() > 1 {
            Err(ContentError::TooManyAttachments)
        } else if let Some(attachment) = message.attachments.get(0) {
            if attachment.size > 8_000_000 {
                Err(ContentError::AttachmentTooLarge)
            } else if let Ok(attachment_bytes) = attachment.download().await {
                Ok(Self {
                    content: content.to_string(),
                    tts: false,
                    attachment: Some(attachment_bytes),
                    attachment_name: Some(attachment.filename.clone()),
                })
            } else {
                Err(ContentError::AttachmentDownloadFailed)
            }
        } else {
            Ok(Self {
                content: content.to_string(),
                tts: false,
                attachment: None,
                attachment_name: None,
            })
        }
    }

    fn substitute(&mut self, guild: Guild) {
        if self.content.starts_with("/tts ") {
            self.tts = true;
            self.content = self.content.split_off(5);
        }

        self.content = REGEX_CONTENT_SUBSTITUTION
            .replace(&self.content, |caps: &Captures| {
                if let Some(user) = caps.name("user") {
                    format!("<@{}>", user.as_str())
                } else if let Some(role_name) = caps.name("role") {
                    if let Some(role) = guild.role_by_name(role_name.as_str()) {
                        role.mention().to_string()
                    } else {
                        format!("<<{}>>", role_name.as_str().to_string())
                    }
                } else {
                    String::new()
                }
            })
            .to_string()
            .replace("<<everyone>>", "@everyone")
            .replace("<<here>>", "@here");
    }
}

#[command("countdown")]
#[permission_level(Managed)]
async fn countdown(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;
    let language = UserData::language_of(&msg.author, &pool).await;

    if check_subscription_on_message(&ctx, &msg).await {
        let timezone = UserData::timezone_of(&msg.author, &pool).await;

        let split_args = args.splitn(3, ' ').collect::<Vec<&str>>();

        if split_args.len() == 3 {
            let time = split_args.get(0).unwrap();
            let interval = split_args.get(1).unwrap();
            let event_name = split_args.get(2).unwrap();

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let time_parser = TimeParser::new(*time, timezone);
            let interval_parser = TimeParser::new(*interval, timezone);

            if let Ok(target_ts) = time_parser.timestamp() {
                if let Ok(interval) = interval_parser.displacement() {
                    let mut first_time = target_ts;

                    while first_time - interval > now as i64 {
                        first_time -= interval;
                    }

                    let description = format!(
                        "**{}** occurs in **<<timefrom:{}:%d days, %h:%m>>**",
                        event_name, target_ts
                    );

                    sqlx::query!(
                        "
INSERT INTO reminders (
    `uid`,
    `name`,
    `embed_title`,
    `embed_description`,
    `embed_color`,
    `channel_id`,
    `utc_time`,
    `interval_seconds`,
    `set_by`,
    `expires`
) VALUES (
    ?,
    'Countdown',
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    (SELECT id FROM users WHERE user = ?),
    FROM_UNIXTIME(?)
)
                    ",
                        generate_uid(),
                        event_name,
                        description,
                        *THEME_COLOR,
                        msg.channel_id.as_u64(),
                        first_time,
                        interval,
                        msg.author.id.as_u64(),
                        target_ts
                    )
                    .execute(&pool)
                    .await
                    .unwrap();

                    let _ = msg.channel_id.send_message(&ctx, |m| {
                        m.embed(|e| {
                            e.title(lm.get(&language, "remind/success")).description(
                                "A new countdown reminder has been created on this channel",
                            )
                        })
                    });
                } else {
                    let _ = msg.channel_id.send_message(&ctx, |m| {
                        m.embed(|e| {
                            e.title(lm.get(&language, "remind/issue"))
                                .description(lm.get(&language, "interval/invalid_interval"))
                        })
                    });
                }
            } else {
                let _ = msg.channel_id.send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.title(lm.get(&language, "remind/issue"))
                            .description(lm.get(&language, "remind/invalid_time"))
                    })
                });
            }
        } else {
            command_help(
                ctx,
                msg,
                lm,
                &ctx.prefix(msg.guild_id).await,
                &language,
                "countdown",
            )
            .await;
        }
    } else {
        let _ = msg
            .channel_id
            .say(
                &ctx,
                lm.get(&language, "interval/donor")
                    .replace("{prefix}", &ctx.prefix(msg.guild_id).await),
            )
            .await;
    }
}

#[command("remind")]
#[permission_level(Managed)]
async fn remind(ctx: &Context, msg: &Message, args: String) {
    remind_command(ctx, msg, args, RemindCommand::Remind).await;
}

#[command("interval")]
#[permission_level(Managed)]
async fn interval(ctx: &Context, msg: &Message, args: String) {
    remind_command(ctx, msg, args, RemindCommand::Interval).await;
}

fn parse_mention_list(mentions: &str) -> Vec<ReminderScope> {
    REGEX_CHANNEL_USER
        .captures_iter(mentions)
        .map(|i| {
            let pref = i.get(1).unwrap().as_str();
            let id = i.get(2).unwrap().as_str().parse::<u64>().unwrap();

            if pref == "#" {
                ReminderScope::Channel(id)
            } else {
                ReminderScope::User(id)
            }
        })
        .collect::<Vec<ReminderScope>>()
}

async fn remind_command(ctx: &Context, msg: &Message, args: String, command: RemindCommand) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let timezone = UserData::timezone_of(&msg.author, &pool).await;
    let language = UserData::language_of(&msg.author, &pool).await;

    match REGEX_REMIND_COMMAND.captures(&args) {
        Some(captures) => {
            let parsed = parse_mention_list(captures.name("mentions").unwrap().as_str());

            let scopes = if parsed.is_empty() {
                vec![ReminderScope::Channel(msg.channel_id.into())]
            } else {
                parsed
            };

            let time_parser = TimeParser::new(captures.name("time").unwrap().as_str(), timezone);

            let expires_parser = captures
                .name("expires")
                .map(|mat| TimeParser::new(mat.as_str(), timezone));

            let interval_parser = captures
                .name("interval")
                .map(|mat| TimeParser::new(mat.as_str(), timezone))
                .map(|parser| parser.displacement())
                .transpose();

            if let Ok(interval) = interval_parser {
                if interval.is_some() && !check_subscription_on_message(&ctx, msg).await {
                    // no patreon
                    let _ = msg
                        .channel_id
                        .say(
                            &ctx,
                            lm.get(&language, "interval/donor")
                                .replace("{prefix}", &ctx.prefix(msg.guild_id).await),
                        )
                        .await;
                } else {
                    let content_res = Content::build(
                        captures.name("content").map(|mat| mat.as_str()).unwrap(),
                        msg,
                    )
                    .await;

                    match content_res {
                        Ok(mut content) => {
                            let mut ok_locations = vec![];
                            let mut ok_reminders = vec![];
                            let mut err_locations = vec![];
                            let mut err_types = HashSet::new();

                            for scope in scopes {
                                let res = create_reminder(
                                    &ctx,
                                    &pool,
                                    msg.author.id,
                                    msg.guild_id,
                                    &scope,
                                    &time_parser,
                                    expires_parser.as_ref(),
                                    interval,
                                    &mut content,
                                )
                                .await;

                                match res {
                                    Err(e) => {
                                        err_locations.push(scope);
                                        err_types.insert(e);
                                    }

                                    Ok(id) => {
                                        ok_locations.push(scope);
                                        ok_reminders.push(id);
                                    }
                                }
                            }

                            let success_part = match ok_locations.len() {
                                0 => "".to_string(),
                                1 => lm
                                    .get(&language, "remind/success")
                                    .replace("{location}", &ok_locations[0].mention())
                                    .replace(
                                        "{offset}",
                                        &format!("<t:{}:R>", time_parser.timestamp().unwrap()),
                                    ),
                                n => lm
                                    .get(&language, "remind/success_bulk")
                                    .replace("{number}", &n.to_string())
                                    .replace(
                                        "{location}",
                                        &ok_locations
                                            .iter()
                                            .map(|l| l.mention())
                                            .collect::<Vec<String>>()
                                            .join(", "),
                                    )
                                    .replace(
                                        "{offset}",
                                        &format!("<t:{}:R>", time_parser.timestamp().unwrap()),
                                    ),
                            };

                            let error_part = format!(
                                "{}\n{}",
                                match err_locations.len() {
                                    0 => "".to_string(),
                                    1 => lm
                                        .get(&language, "remind/issue")
                                        .replace("{location}", &err_locations[0].mention()),
                                    n => lm
                                        .get(&language, "remind/issue_bulk")
                                        .replace("{number}", &n.to_string())
                                        .replace(
                                            "{location}",
                                            &err_locations
                                                .iter()
                                                .map(|l| l.mention())
                                                .collect::<Vec<String>>()
                                                .join(", "),
                                        ),
                                },
                                err_types
                                    .iter()
                                    .map(|err| match err {
                                        ReminderError::DiscordError(s) => lm
                                            .get(&language, err.to_response())
                                            .replace("{error}", &s),

                                        _ => lm
                                            .get(&language, err.to_response())
                                            .replace("{min_interval}", &*MIN_INTERVAL.to_string()),
                                    })
                                    .collect::<Vec<String>>()
                                    .join("\n")
                            );

                            let _ = msg
                                .channel_id
                                .send_message(&ctx, |m| {
                                    m.embed(|e| {
                                        e.title(
                                            lm.get(&language, "remind/title").replace(
                                                "{number}",
                                                &ok_locations.len().to_string(),
                                            ),
                                        )
                                        .description(format!("{}\n\n{}", success_part, error_part))
                                        .color(*THEME_COLOR)
                                    })
                                })
                                .await;
                        }

                        Err(content_error) => {
                            let _ = msg
                                .channel_id
                                .send_message(ctx, |m| {
                                    m.embed(move |e| {
                                        e.title(
                                            lm.get(&language, "remind/title")
                                                .replace("{number}", "0"),
                                        )
                                        .description(lm.get(&language, content_error.to_response()))
                                        .color(*THEME_COLOR)
                                    })
                                })
                                .await;
                        }
                    }
                }
            } else {
                let _ = msg
                    .channel_id
                    .send_message(ctx, |m| {
                        m.embed(move |e| {
                            e.title(lm.get(&language, "remind/title").replace("{number}", "0"))
                                .description(lm.get(&language, "interval/invalid_interval"))
                                .color(*THEME_COLOR)
                        })
                    })
                    .await;
            }
        }

        None => {
            let prefix = ctx.prefix(msg.guild_id).await;

            match command {
                RemindCommand::Remind => {
                    command_help(ctx, msg, lm, &prefix, &language, "remind").await
                }

                RemindCommand::Interval => {
                    command_help(ctx, msg, lm, &prefix, &language, "interval").await
                }
            }
        }
    }
}

#[command("natural")]
#[permission_level(Managed)]
async fn natural(ctx: &Context, msg: &Message, args: String) {
    let (pool, lm) = get_ctx_data(&ctx).await;

    let user_data = UserData::from_user(&msg.author, &ctx, &pool).await.unwrap();

    match REGEX_NATURAL_COMMAND_1.captures(&args) {
        Some(captures) => {
            let (expires, interval, string_content) =
                if check_subscription_on_message(&ctx, msg).await {
                    let rest_content = captures.name("msg").unwrap().as_str();

                    match REGEX_NATURAL_COMMAND_2.captures(rest_content) {
                        Some(secondary_captures) => {
                            let expires =
                                if let Some(expires_crop) = secondary_captures.name("expires") {
                                    natural_parser(expires_crop.as_str(), &user_data.timezone).await
                                } else {
                                    None
                                };

                            let interval =
                                if let Some(interval_crop) = secondary_captures.name("interval") {
                                    humantime::parse_duration(interval_crop.as_str())
                                        .or_else(|_| {
                                            humantime::parse_duration(&format!(
                                                "1 {}",
                                                interval_crop.as_str()
                                            ))
                                        })
                                        .map(|duration| duration.as_secs() as i64)
                                        .ok()
                                } else {
                                    None
                                };

                            (
                                expires,
                                interval,
                                if interval.is_some() {
                                    secondary_captures.name("msg").unwrap().as_str()
                                } else {
                                    rest_content
                                },
                            )
                        }

                        None => (None, None, rest_content),
                    }
                } else {
                    (None, None, captures.name("msg").unwrap().as_str())
                };

            let location_ids = if let Some(mentions) = captures.name("mentions").map(|m| m.as_str())
            {
                parse_mention_list(mentions)
            } else {
                vec![ReminderScope::Channel(msg.channel_id.into())]
            };

            if let Some(timestamp) =
                natural_parser(captures.name("time").unwrap().as_str(), &user_data.timezone).await
            {
                let content_res = Content::build(string_content, msg).await;

                match content_res {
                    Ok(mut content) => {
                        let mut ok_locations = vec![];
                        let mut err_locations = vec![];
                        let mut err_types = HashSet::new();

                        for scope in location_ids {
                            let res = create_reminder(
                                &ctx,
                                &pool,
                                msg.author.id,
                                msg.guild_id,
                                &scope,
                                timestamp,
                                expires,
                                interval,
                                &mut content,
                            )
                            .await;

                            if let Err(e) = res {
                                err_locations.push(scope);
                                err_types.insert(e);
                            } else {
                                ok_locations.push(scope);
                            }
                        }

                        let success_part = match ok_locations.len() {
                            0 => "".to_string(),
                            1 => lm
                                .get(&user_data.language, "remind/success")
                                .replace("{location}", &ok_locations[0].mention())
                                .replace("{offset}", &format!("<t:{}:R>", timestamp)),
                            n => lm
                                .get(&user_data.language, "remind/success_bulk")
                                .replace("{number}", &n.to_string())
                                .replace(
                                    "{location}",
                                    &ok_locations
                                        .iter()
                                        .map(|l| l.mention())
                                        .collect::<Vec<String>>()
                                        .join(", "),
                                )
                                .replace("{offset}", &format!("<t:{}:R>", timestamp)),
                        };

                        let error_part = format!(
                            "{}\n{}",
                            match err_locations.len() {
                                0 => "".to_string(),
                                1 => lm
                                    .get(&user_data.language, "remind/issue")
                                    .replace("{location}", &err_locations[0].mention()),
                                n => lm
                                    .get(&user_data.language, "remind/issue_bulk")
                                    .replace("{number}", &n.to_string())
                                    .replace(
                                        "{location}",
                                        &err_locations
                                            .iter()
                                            .map(|l| l.mention())
                                            .collect::<Vec<String>>()
                                            .join(", "),
                                    ),
                            },
                            err_types
                                .iter()
                                .map(|err| match err {
                                    ReminderError::DiscordError(s) => lm
                                        .get(&user_data.language, err.to_response_natural())
                                        .replace("{error}", &s),

                                    _ => lm
                                        .get(&user_data.language, err.to_response_natural())
                                        .to_string(),
                                })
                                .collect::<Vec<String>>()
                                .join("\n")
                        );

                        let _ = msg
                            .channel_id
                            .send_message(&ctx, |m| {
                                m.embed(|e| {
                                    e.title(
                                        lm.get(&user_data.language, "remind/title")
                                            .replace("{number}", &ok_locations.len().to_string()),
                                    )
                                    .description(format!("{}\n\n{}", success_part, error_part))
                                    .color(*THEME_COLOR)
                                })
                            })
                            .await;
                    }

                    Err(content_error) => {
                        let _ = msg
                            .channel_id
                            .send_message(ctx, |m| {
                                m.embed(move |e| {
                                    e.title(
                                        lm.get(&user_data.language, "remind/title")
                                            .replace("{number}", "0"),
                                    )
                                    .description(
                                        lm.get(&user_data.language, content_error.to_response()),
                                    )
                                    .color(*THEME_COLOR)
                                })
                            })
                            .await;
                    }
                }
            } else {
                let _ = msg
                    .channel_id
                    .send_message(ctx, |m| {
                        m.embed(move |e| {
                            e.title(
                                lm.get(&user_data.language, "remind/title")
                                    .replace("{number}", "0"),
                            )
                            .description(lm.get(&user_data.language, "natural/invalid_time"))
                            .color(*THEME_COLOR)
                        })
                    })
                    .await;
            }
        }

        None => {
            command_help(
                ctx,
                msg,
                lm,
                &ctx.prefix(msg.guild_id).await,
                &user_data.language,
                "natural",
            )
            .await;
        }
    }
}

async fn create_reminder<'a, U: Into<u64>, T: TryInto<i64>>(
    ctx: &Context,
    pool: &MySqlPool,
    user_id: U,
    guild_id: Option<GuildId>,
    scope_id: &ReminderScope,
    time_parser: T,
    expires_parser: Option<T>,
    interval: Option<i64>,
    content: &mut Content,
) -> Result<Reminder, ReminderError> {
    let user_id = user_id.into();

    if let Some(g_id) = guild_id {
        if let Some(guild) = g_id.to_guild_cached(&ctx) {
            content.substitute(guild);
        }
    }

    let mut nudge = 0;

    let db_channel_id = match scope_id {
        ReminderScope::User(user_id) => {
            if let Ok(user) = UserId(*user_id).to_user(&ctx).await {
                let user_data = UserData::from_user(&user, &ctx, &pool).await.unwrap();

                if let Some(guild_id) = guild_id {
                    if guild_id.member(&ctx, user).await.is_err() {
                        return Err(ReminderError::InvalidTag);
                    }
                }

                user_data.dm_channel
            } else {
                return Err(ReminderError::InvalidTag);
            }
        }

        ReminderScope::Channel(channel_id) => {
            let channel = ChannelId(*channel_id).to_channel(&ctx).await.unwrap();

            if channel.clone().guild().map(|gc| gc.guild_id) != guild_id {
                return Err(ReminderError::InvalidTag);
            }

            let mut channel_data = ChannelData::from_channel(channel.clone(), &pool)
                .await
                .unwrap();

            nudge = channel_data.nudge;

            if let Some(guild_channel) = channel.guild() {
                if channel_data.webhook_token.is_none() || channel_data.webhook_id.is_none() {
                    match create_webhook(&ctx, guild_channel, "Reminder").await {
                        Ok(webhook) => {
                            channel_data.webhook_id = Some(webhook.id.as_u64().to_owned());
                            channel_data.webhook_token = webhook.token;

                            channel_data.commit_changes(&pool).await;
                        }

                        Err(e) => {
                            return Err(ReminderError::DiscordError(e.to_string()));
                        }
                    }
                }
            }

            channel_data.id
        }
    };

    // validate time, channel
    if interval.map_or(false, |inner| inner < *MIN_INTERVAL) {
        Err(ReminderError::ShortInterval)
    } else if interval.map_or(false, |inner| inner > *MAX_TIME) {
        Err(ReminderError::LongInterval)
    } else {
        match time_parser.try_into() {
            Ok(time_pre) => {
                match expires_parser.map(|t| t.try_into()).transpose() {
                    Ok(expires) => {
                        let time = time_pre + nudge as i64;

                        let unix_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;

                        if time >= unix_time - 10 {
                            let uid = generate_uid();

                            sqlx::query!(
                                "
INSERT INTO reminders (
    uid,
    content,
    tts,
    attachment,
    attachment_name,
    channel_id,
    `utc_time`,
    expires,
    `interval_seconds`,
    set_by
) VALUES (
    ?,
    ?,
    ?,
    ?,
    ?,
    ?,
    DATE_ADD(FROM_UNIXTIME(0), INTERVAL ? SECOND),
    DATE_ADD(FROM_UNIXTIME(0), INTERVAL ? SECOND),
    ?,
    (SELECT id FROM users WHERE user = ? LIMIT 1)
)
                            ",
                                uid,
                                content.content,
                                content.tts,
                                content.attachment,
                                content.attachment_name,
                                db_channel_id,
                                time,
                                expires,
                                interval,
                                user_id
                            )
                            .execute(pool)
                            .await
                            .unwrap();

                            let reminder = Reminder::from_uid(ctx, uid).await.unwrap();

                            Ok(reminder)
                        } else if time < 0 {
                            // case required for if python returns -1
                            Err(ReminderError::InvalidTime)
                        } else {
                            Err(ReminderError::PastTime)
                        }
                    }

                    Err(_) => Err(ReminderError::InvalidExpiration),
                }
            }

            Err(_) => Err(ReminderError::InvalidTime),
        }
    }
}
