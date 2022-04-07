pub const DISCORD_OAUTH_TOKEN: &'static str = "https://discord.com/api/oauth2/token";
pub const DISCORD_OAUTH_AUTHORIZE: &'static str = "https://discord.com/api/oauth2/authorize";
pub const DISCORD_API: &'static str = "https://discord.com/api";
pub const DISCORD_CDN: &'static str = "https://cdn.discordapp.com/avatars";

pub const MAX_CONTENT_LENGTH: usize = 2000;
pub const MAX_EMBED_DESCRIPTION_LENGTH: usize = 4096;
pub const MAX_EMBED_TITLE_LENGTH: usize = 256;
pub const MAX_EMBED_AUTHOR_LENGTH: usize = 256;
pub const MAX_EMBED_FOOTER_LENGTH: usize = 2048;
pub const MAX_URL_LENGTH: usize = 512;
pub const MAX_USERNAME_LENGTH: usize = 100;
pub const MAX_EMBED_FIELDS: usize = 25;
pub const MAX_EMBED_FIELD_TITLE_LENGTH: usize = 256;
pub const MAX_EMBED_FIELD_VALUE_LENGTH: usize = 1024;

pub const MINUTE: usize = 60;
pub const HOUR: usize = 60 * MINUTE;
pub const DAY: usize = 24 * HOUR;

pub const CHARACTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

use std::{collections::HashSet, env, iter::FromIterator};

use lazy_static::lazy_static;
use serenity::model::prelude::AttachmentType;

lazy_static! {
    pub static ref DEFAULT_AVATAR: AttachmentType<'static> = (
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/",
            env!("WEBHOOK_AVATAR", "WEBHOOK_AVATAR not provided for compilation")
        )) as &[u8],
        env!("WEBHOOK_AVATAR"),
    )
        .into();
    pub static ref SUBSCRIPTION_ROLES: HashSet<u64> = HashSet::from_iter(
        env::var("SUBSCRIPTION_ROLES")
            .map(|var| var
                .split(',')
                .filter_map(|item| { item.parse::<u64>().ok() })
                .collect::<Vec<u64>>())
            .unwrap_or_else(|_| Vec::new())
    );
    pub static ref CNC_GUILD: Option<u64> =
        env::var("CNC_GUILD").map(|var| var.parse::<u64>().ok()).ok().flatten();
    pub static ref MIN_INTERVAL: u32 = env::var("MIN_INTERVAL")
        .ok()
        .map(|inner| inner.parse::<u32>().ok())
        .flatten()
        .unwrap_or(600);
}
