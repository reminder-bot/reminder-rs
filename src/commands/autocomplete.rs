use chrono_tz::TZ_VARIANTS;

use crate::Context;

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
