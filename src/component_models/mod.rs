pub(crate) mod pager;

use std::io::Cursor;

use chrono_tz::Tz;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::Channel,
        id::{GuildId, RoleId, UserId},
        interactions::{message_component::MessageComponentInteraction, InteractionResponseType},
        prelude::InteractionApplicationCommandCallbackDataFlags,
    },
};

use crate::{
    component_models::pager::{DelPager, LookPager, Pager},
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    models::reminder::Reminder,
    SQLPool,
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[repr(u8)]
pub enum ComponentDataModel {
    Restrict(Restrict),
    LookPager(LookPager),
    DelPager(DelPager),
    DelSelector(DelSelector),
}

impl ComponentDataModel {
    pub fn to_custom_id(&self) -> String {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        base64::encode(buf)
    }

    pub fn from_custom_id(data: &String) -> Self {
        let buf = base64::decode(data).unwrap();
        let cur = Cursor::new(buf);
        rmp_serde::from_read(cur).unwrap()
    }

    pub async fn act(&self, ctx: &Context, component: MessageComponentInteraction) {
        match self {
            ComponentDataModel::Restrict(restrict) => {
                if restrict.author_id == component.user.id {
                    let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

                    let _ = sqlx::query!(
                        "
INSERT IGNORE INTO roles (role, name, guild_id) VALUES (?, \"Role\", (SELECT id FROM guilds WHERE guild = ?))
                        ",
                        restrict.role_id.0,
                        restrict.guild_id.0
                    )
                    .execute(&pool)
                    .await;

                    for command in &component.data.values {
                        let _ = sqlx::query!(
                            "INSERT INTO command_restrictions (role_id, command) VALUES ((SELECT id FROM roles WHERE role = ?), ?)",
                            restrict.role_id.0,
                            command
                        )
                        .execute(&pool)
                        .await;
                    }

                    component
                        .create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|response| response
                                    .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                    .content("Role permissions updated")
                                )
                        })
                        .await
                        .unwrap();
                } else {
                    // tell them they cant do this
                }
            }
            ComponentDataModel::LookPager(pager) => {
                let flags = pager.flags;

                let channel_opt = component.channel_id.to_channel_cached(&ctx);

                let channel_id = if let Some(Channel::Guild(channel)) = channel_opt {
                    if Some(channel.guild_id) == component.guild_id {
                        flags.channel_id.unwrap_or(component.channel_id)
                    } else {
                        component.channel_id
                    }
                } else {
                    component.channel_id
                };

                let reminders = Reminder::from_channel(ctx, channel_id, &flags).await;

                let pages = reminders
                    .iter()
                    .map(|reminder| reminder.display(&flags, &pager.timezone))
                    .fold(0, |t, r| t + r.len())
                    .div_ceil(EMBED_DESCRIPTION_MAX_LENGTH);

                let channel_name =
                    if let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx) {
                        Some(channel.name)
                    } else {
                        None
                    };

                let next_page = pager.next_page(pages);

                let mut char_count = 0;
                let mut skip_char_count = 0;

                let display = reminders
                    .iter()
                    .map(|reminder| reminder.display(&flags, &pager.timezone))
                    .skip_while(|p| {
                        skip_char_count += p.len();

                        skip_char_count < EMBED_DESCRIPTION_MAX_LENGTH * next_page as usize
                    })
                    .take_while(|p| {
                        char_count += p.len();

                        char_count < EMBED_DESCRIPTION_MAX_LENGTH
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let mut embed = CreateEmbed::default();
                embed
                    .title(format!(
                        "Reminders{}",
                        channel_name.map_or(String::new(), |n| format!(" on #{}", n))
                    ))
                    .description(display)
                    .footer(|f| f.text(format!("Page {} of {}", next_page + 1, pages)))
                    .color(*THEME_COLOR);

                let _ = component
                    .create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |response| {
                                response.embeds(vec![embed]).components(|comp| {
                                    pager.create_button_row(pages, comp);

                                    comp
                                })
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::DelPager(pager) => {
                let reminders =
                    Reminder::from_guild(ctx, component.guild_id, component.user.id).await;

                let pages = reminders
                    .iter()
                    .enumerate()
                    .map(|(count, reminder)| reminder.display_del(count, &pager.timezone))
                    .fold(0, |t, r| t + r.len())
                    .div_ceil(EMBED_DESCRIPTION_MAX_LENGTH);

                let next_page = pager.next_page(pages);

                let mut char_count = 0;
                let mut skip_char_count = 0;
                let mut first_num = 0;

                let (shown_reminders, display_vec): (Vec<&Reminder>, Vec<String>) = reminders
                    .iter()
                    .enumerate()
                    .map(|(count, reminder)| {
                        (reminder, reminder.display_del(count, &pager.timezone))
                    })
                    .skip_while(|(_, p)| {
                        first_num += 1;
                        skip_char_count += p.len();

                        skip_char_count < EMBED_DESCRIPTION_MAX_LENGTH * next_page
                    })
                    .take_while(|(_, p)| {
                        char_count += p.len();

                        char_count < EMBED_DESCRIPTION_MAX_LENGTH
                    })
                    .unzip();

                let display = display_vec.join("\n");

                let del_selector = ComponentDataModel::DelSelector(DelSelector {
                    page: next_page,
                    timezone: pager.timezone,
                });

                let mut embed = CreateEmbed::default();
                embed
                    .title("Delete Reminders")
                    .description(display)
                    .footer(|f| f.text(format!("Page {} of {}", next_page + 1, pages)))
                    .color(*THEME_COLOR);

                component
                    .create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |response| {
                                response.embeds(vec![embed]).components(|comp| {
                                    pager.create_button_row(pages, comp);

                                    comp.create_action_row(|row| {
                                        row.create_select_menu(|menu| {
                                            menu.custom_id(del_selector.to_custom_id()).options(
                                                |opt| {
                                                    for (count, reminder) in
                                                        shown_reminders.iter().enumerate()
                                                    {
                                                        opt.create_option(|o| {
                                                            o.label(count + first_num)
                                                                .value(reminder.id)
                                                                .description({
                                                                    let c =
                                                                        reminder.display_content();

                                                                    if c.len() > 100 {
                                                                        format!(
                                                                            "{}...",
                                                                            reminder
                                                                                .display_content()
                                                                                .chars()
                                                                                .take(97)
                                                                                .collect::<String>(
                                                                                )
                                                                        )
                                                                    } else {
                                                                        c.to_string()
                                                                    }
                                                                })
                                                        });
                                                    }

                                                    opt
                                                },
                                            )
                                        })
                                    })
                                })
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::DelSelector(selector) => {
                let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();
                let selected_id = component.data.values.join(",");

                sqlx::query!("DELETE FROM reminders WHERE FIND_IN_SET(id, ?)", selected_id)
                    .execute(&pool)
                    .await
                    .unwrap();

                let reminders =
                    Reminder::from_guild(ctx, component.guild_id, component.user.id).await;

                let mut char_count = 0;
                let mut skip_char_count = 0;
                let mut first_num = 0;

                let (shown_reminders, display_vec): (Vec<&Reminder>, Vec<String>) = reminders
                    .iter()
                    .enumerate()
                    .map(|(count, reminder)| {
                        (reminder, reminder.display_del(count, &selector.timezone))
                    })
                    .skip_while(|(_, p)| {
                        first_num += 1;
                        skip_char_count += p.len();

                        skip_char_count < EMBED_DESCRIPTION_MAX_LENGTH * selector.page
                    })
                    .take_while(|(_, p)| {
                        char_count += p.len();

                        char_count < EMBED_DESCRIPTION_MAX_LENGTH
                    })
                    .unzip();

                let display = display_vec.join("\n");

                let pages = reminders
                    .iter()
                    .enumerate()
                    .map(|(count, reminder)| reminder.display_del(count, &selector.timezone))
                    .fold(0, |t, r| t + r.len())
                    .div_ceil(EMBED_DESCRIPTION_MAX_LENGTH);

                let pager = DelPager::new(selector.timezone);

                let del_selector = ComponentDataModel::DelSelector(DelSelector {
                    page: selector.page,
                    timezone: selector.timezone,
                });

                let mut embed = CreateEmbed::default();
                embed
                    .title("Delete Reminders")
                    .description(display)
                    .footer(|f| f.text(format!("Page {} of {}", selector.page + 1, pages)))
                    .color(*THEME_COLOR);

                component
                    .create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |response| {
                                response.embeds(vec![embed]).components(|comp| {
                                    pager.create_button_row(pages, comp);

                                    comp.create_action_row(|row| {
                                        row.create_select_menu(|menu| {
                                            menu.custom_id(del_selector.to_custom_id()).options(
                                                |opt| {
                                                    for (count, reminder) in
                                                        shown_reminders.iter().enumerate()
                                                    {
                                                        opt.create_option(|o| {
                                                            o.label(count + 1)
                                                                .value(reminder.id)
                                                                .description({
                                                                    let c =
                                                                        reminder.display_content();

                                                                    if c.len() > 100 {
                                                                        format!(
                                                                            "{}...",
                                                                            reminder
                                                                                .display_content()
                                                                                .chars()
                                                                                .take(97)
                                                                                .collect::<String>(
                                                                                )
                                                                        )
                                                                    } else {
                                                                        c.to_string()
                                                                    }
                                                                })
                                                        });
                                                    }

                                                    opt
                                                },
                                            )
                                        })
                                    })
                                })
                            },
                        )
                    })
                    .await;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Restrict {
    pub role_id: RoleId,
    pub author_id: UserId,
    pub guild_id: GuildId,
}

#[derive(Serialize, Deserialize)]
pub struct DelSelector {
    pub page: usize,
    pub timezone: Tz,
}
