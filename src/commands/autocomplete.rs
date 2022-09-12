use std::time::{SystemTime, UNIX_EPOCH};

use chrono_tz::TZ_VARIANTS;
use poise::AutocompleteChoice;

use crate::{models::CtxData, time_parser::natural_parser, Context};

pub async fn timezone_autocomplete(ctx: Context<'_>, partial: &str) -> Vec<String> {
    if partial.is_empty() {
        ctx.data().popular_timezones.iter().map(|t| t.to_string()).collect::<Vec<String>>()
    } else {
        TZ_VARIANTS
            .iter()
            .filter(|tz| tz.to_string().contains(&partial))
            .take(25)
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
    }
}

pub async fn macro_name_autocomplete(ctx: Context<'_>, partial: &str) -> Vec<String> {
    sqlx::query!(
        "
SELECT name
FROM macro
WHERE
    guild_id = (SELECT id FROM guilds WHERE guild = ?)
    AND name LIKE CONCAT(?, '%')",
        ctx.guild_id().unwrap().0,
        partial,
    )
    .fetch_all(&ctx.data().database)
    .await
    .unwrap_or_default()
    .iter()
    .map(|s| s.name.clone())
    .collect()
}

pub async fn multiline_autocomplete(
    _ctx: Context<'_>,
    partial: &str,
) -> Vec<AutocompleteChoice<String>> {
    if partial.is_empty() {
        vec![AutocompleteChoice { name: "Multiline content...".to_string(), value: "".to_string() }]
    } else {
        vec![
            AutocompleteChoice { name: partial.to_string(), value: partial.to_string() },
            AutocompleteChoice { name: "Multiline content...".to_string(), value: "".to_string() },
        ]
    }
}

pub async fn time_hint_autocomplete(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<AutocompleteChoice<String>> {
    if partial.is_empty() {
        vec![AutocompleteChoice {
            name: "Start typing a time...".to_string(),
            value: "now".to_string(),
        }]
    } else {
        match natural_parser(partial, &ctx.timezone().await.to_string()).await {
            Some(timestamp) => match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(now) => {
                    let diff = timestamp - now.as_secs() as i64;

                    if diff < 0 {
                        vec![AutocompleteChoice {
                            name: "Time is in the past".to_string(),
                            value: "now".to_string(),
                        }]
                    } else {
                        if diff > 86400 {
                            vec![
                                AutocompleteChoice {
                                    name: partial.to_string(),
                                    value: partial.to_string(),
                                },
                                AutocompleteChoice {
                                    name: format!(
                                        "In approximately {} days, {} hours",
                                        diff / 86400,
                                        (diff % 86400) / 3600
                                    ),
                                    value: partial.to_string(),
                                },
                            ]
                        } else if diff > 3600 {
                            vec![
                                AutocompleteChoice {
                                    name: partial.to_string(),
                                    value: partial.to_string(),
                                },
                                AutocompleteChoice {
                                    name: format!("In approximately {} hours", diff / 3600),
                                    value: partial.to_string(),
                                },
                            ]
                        } else {
                            vec![
                                AutocompleteChoice {
                                    name: partial.to_string(),
                                    value: partial.to_string(),
                                },
                                AutocompleteChoice {
                                    name: format!("In approximately {} minutes", diff / 60),
                                    value: partial.to_string(),
                                },
                            ]
                        }
                    }
                }
                Err(_) => {
                    vec![AutocompleteChoice {
                        name: partial.to_string(),
                        value: partial.to_string(),
                    }]
                }
            },

            None => {
                vec![AutocompleteChoice {
                    name: "Time not recognised".to_string(),
                    value: "now".to_string(),
                }]
            }
        }
    }
}
