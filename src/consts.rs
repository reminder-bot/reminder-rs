pub const DAY: u64 = 86_400;
pub const HOUR: u64 = 3_600;
pub const MINUTE: u64 = 60;

pub const CHARACTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

const THEME_COLOR_FALLBACK: u32 = 0x8fb677;

use std::{collections::HashSet, env, iter::FromIterator};

use regex::Regex;

lazy_static! {
    pub static ref SUBSCRIPTION_ROLES: HashSet<u64> = HashSet::from_iter(
        env::var("SUBSCRIPTION_ROLES")
            .map(|var| var
                .split(',')
                .filter_map(|item| { item.parse::<u64>().ok() })
                .collect::<Vec<u64>>())
            .unwrap_or_else(|_| vec![])
    );
    pub static ref CNC_GUILD: Option<u64> = env::var("CNC_GUILD")
        .map(|var| var.parse::<u64>().ok())
        .ok()
        .flatten();
    pub static ref REGEX_CHANNEL: Regex = Regex::new(r#"^\s*<#(\d+)>\s*$"#).unwrap();
    pub static ref REGEX_ROLE: Regex = Regex::new(r#"<@&([0-9]+)>"#).unwrap();
    pub static ref REGEX_COMMANDS: Regex = Regex::new(r#"([a-z]+)"#).unwrap();
    pub static ref REGEX_ALIAS: Regex =
        Regex::new(r#"(?P<name>[\S]{1,12})(?:(?: (?P<cmd>.*)$)|$)"#).unwrap();
    pub static ref REGEX_CHANNEL_USER: Regex = Regex::new(r#"^\s*<(#|@)(?:!)?(\d+)>\s*$"#).unwrap();
    pub static ref MIN_INTERVAL: i64 = env::var("MIN_INTERVAL")
        .ok()
        .map(|inner| inner.parse::<i64>().ok())
        .flatten()
        .unwrap_or(600);
    pub static ref MAX_TIME: i64 = env::var("MAX_TIME")
        .ok()
        .map(|inner| inner.parse::<i64>().ok())
        .flatten()
        .unwrap_or(60 * 60 * 24 * 365 * 50);
    pub static ref LOCAL_TIMEZONE: String =
        env::var("LOCAL_TIMEZONE").unwrap_or_else(|_| "UTC".to_string());
    pub static ref LOCAL_LANGUAGE: String =
        env::var("LOCAL_LANGUAGE").unwrap_or_else(|_| "EN".to_string());
    pub static ref PYTHON_LOCATION: String =
        env::var("PYTHON_LOCATION").unwrap_or_else(|_| "venv/bin/python3".to_string());
    pub static ref STRINGS_TABLE: String =
        env::var("STRINGS_TABLE").unwrap_or_else(|_| "strings".to_string());
    pub static ref DEFAULT_PREFIX: String =
        env::var("DEFAULT_PREFIX").unwrap_or_else(|_| "$".to_string());
    pub static ref THEME_COLOR: u32 = env::var("THEME_COLOR").map_or(
        THEME_COLOR_FALLBACK,
        |inner| u32::from_str_radix(&inner, 16).unwrap_or(THEME_COLOR_FALLBACK)
    );
    pub static ref WEBHOOK_AVATAR: Option<String> = env::var("WEBHOOK_AVATAR").ok();
}
