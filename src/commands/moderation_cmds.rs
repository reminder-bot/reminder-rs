use chrono::offset::Utc;
use chrono_tz::{Tz, TZ_VARIANTS};
use levenshtein::levenshtein;

use super::autocomplete::timezone_autocomplete;
use crate::{consts::THEME_COLOR, models::CtxData, Context, Error};

/// Select your timezone
#[poise::command(slash_command, identifying_name = "timezone")]
pub async fn timezone(
    ctx: Context<'_>,
    #[description = "Timezone to use from this list: https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee"]
    #[autocomplete = "timezone_autocomplete"]
    timezone: Option<String>,
) -> Result<(), Error> {
    let mut user_data = ctx.author_data().await.unwrap();

    let footer_text = format!("Current timezone: {}", user_data.timezone);

    if let Some(timezone) = timezone {
        match timezone.parse::<Tz>() {
            Ok(tz) => {
                user_data.timezone = timezone.clone();
                user_data.commit_changes(&ctx.data().database).await;

                let now = Utc::now().with_timezone(&tz);

                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Timezone Set")
                            .description(format!(
                                "Timezone has been set to **{}**. Your current time should be `{}`",
                                timezone,
                                now.format("%H:%M")
                            ))
                            .color(*THEME_COLOR)
                    })
                })
                .await?;
            }

            Err(_) => {
                let filtered_tz = TZ_VARIANTS
                    .iter()
                    .filter(|tz| {
                        timezone.contains(&tz.to_string())
                            || tz.to_string().contains(&timezone)
                            || levenshtein(&tz.to_string(), &timezone) < 4
                    })
                    .take(25)
                    .map(|t| t.to_owned())
                    .collect::<Vec<Tz>>();

                let fields = filtered_tz.iter().map(|tz| {
                    (
                        tz.to_string(),
                        format!("🕗 `{}`", Utc::now().with_timezone(tz).format("%H:%M")),
                        true,
                    )
                });

                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Timezone Not Recognized")
                            .description("Possibly you meant one of the following timezones, otherwise click [here](https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee):")
                            .color(*THEME_COLOR)
                            .fields(fields)
                            .footer(|f| f.text(footer_text))
                            .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
                    })
                })
                .await?;
            }
        }
    } else {
        let popular_timezones_iter = ctx.data().popular_timezones.iter().map(|t| {
            (t.to_string(), format!("🕗 `{}`", Utc::now().with_timezone(t).format("%H:%M")), true)
        });

        ctx.send(|m| {
            m.embed(|e| {
                e.title("Timezone Usage")
                    .description(
                        "**Usage:**
`/timezone Name`

**Example:**
`/timezone Europe/London`

You may want to use one of the popular timezones below, otherwise click [here](https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee):",
                    )
                    .color(*THEME_COLOR)
                    .fields(popular_timezones_iter)
                    .footer(|f| f.text(footer_text))
                    .url("https://gist.github.com/JellyWX/913dfc8b63d45192ad6cb54c829324ee")
            })
        })
        .await?;
    }

    Ok(())
}

/// Configure whether other users can set reminders to your direct messages
#[poise::command(slash_command, rename = "dm", identifying_name = "allowed_dm")]
pub async fn allowed_dm(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Allow other users to set reminders in your direct messages
#[poise::command(slash_command, rename = "allow", identifying_name = "allowed_dm")]
pub async fn set_allowed_dm(ctx: Context<'_>) -> Result<(), Error> {
    let mut user_data = ctx.author_data().await?;
    user_data.allowed_dm = true;
    user_data.commit_changes(&ctx.data().database).await;

    ctx.send(|r| {
        r.ephemeral(true).embed(|e| {
            e.title("DMs permitted")
                .description("You will receive a message if a user sets a DM reminder for you.")
                .color(*THEME_COLOR)
        })
    })
    .await?;

    Ok(())
}

/// Block other users from setting reminders in your direct messages
#[poise::command(slash_command, rename = "block", identifying_name = "allowed_dm")]
pub async fn unset_allowed_dm(ctx: Context<'_>) -> Result<(), Error> {
    let mut user_data = ctx.author_data().await?;
    user_data.allowed_dm = false;
    user_data.commit_changes(&ctx.data().database).await;

    ctx.send(|r| {
        r.ephemeral(true).embed(|e| {
            e.title("DMs blocked")
                .description(
                    "You can still set DM reminders for yourself or for users with DMs enabled.",
                )
                .color(*THEME_COLOR)
        })
    })
    .await?;

    Ok(())
}

/// View the webhook being used to send reminders to this channel
#[poise::command(
    slash_command,
    identifying_name = "webhook_url",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn webhook(ctx: Context<'_>) -> Result<(), Error> {
    match ctx.channel_data().await {
        Ok(data) => {
            if let (Some(id), Some(token)) = (data.webhook_id, data.webhook_token) {
                let _ = ctx
                    .send(|b| {
                        b.ephemeral(true).content(format!(
                            "**Warning!**
This link can be used by users to anonymously send messages, with or without permissions.
Do not share it!
|| https://discord.com/api/webhooks/{}/{} ||",
                            id, token,
                        ))
                    })
                    .await;
            } else {
                let _ = ctx.say("No webhook configured on this channel.").await;
            }
        }
        Err(_) => {
            let _ = ctx.say("No webhook configured on this channel.").await;
        }
    }

    Ok(())
}
