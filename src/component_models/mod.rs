pub(crate) mod pager;

use std::io::Cursor;

use chrono_tz::Tz;
use log::warn;
use poise::{
    serenity_prelude as serenity,
    serenity_prelude::{
        builder::CreateEmbed,
        model::{
            application::interaction::{
                message_component::MessageComponentInteraction, InteractionResponseType,
                MessageFlags,
            },
            channel::Channel,
        },
        Context,
    },
};
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

use crate::{
    commands::{
        command_macro::list::{max_macro_page, show_macro_page},
        reminder_cmds::{max_delete_page, show_delete_page},
        todo_cmds::{max_todo_page, show_todo_page},
    },
    component_models::pager::{DelPager, LookPager, MacroPager, Pager, TodoPager},
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    models::reminder::Reminder,
    utils::send_as_initial_response,
    Data,
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[repr(u8)]
pub enum ComponentDataModel {
    LookPager(LookPager),
    DelPager(DelPager),
    TodoPager(TodoPager),
    DelSelector(DelSelector),
    TodoSelector(TodoSelector),
    MacroPager(MacroPager),
    UndoReminder(UndoReminder),
}

impl ComponentDataModel {
    pub fn to_custom_id(&self) -> String {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        base64::encode(buf)
    }

    pub fn from_custom_id(data: &String) -> Self {
        let buf = base64::decode(data)
            .map_err(|e| format!("Could not decode `custom_id' {}: {:?}", data, e))
            .unwrap();
        let cur = Cursor::new(buf);
        rmp_serde::from_read(cur).unwrap()
    }

    pub async fn act(&self, ctx: &Context, data: &Data, component: &MessageComponentInteraction) {
        match self {
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

                let reminders = Reminder::from_channel(&data.database, channel_id, &flags).await;

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
                    .join("");

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
                                response.set_embeds(vec![embed]).components(|comp| {
                                    pager.create_button_row(pages, comp);

                                    comp
                                })
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::DelPager(pager) => {
                let reminders = Reminder::from_guild(
                    &ctx,
                    &data.database,
                    component.guild_id,
                    component.user.id,
                )
                .await;

                let max_pages = max_delete_page(&reminders, &pager.timezone);

                let resp = show_delete_page(&reminders, pager.next_page(max_pages), pager.timezone);

                let _ = component
                    .create_interaction_response(&ctx, |f| {
                        f.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |d| {
                                send_as_initial_response(resp, d);
                                d
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::DelSelector(selector) => {
                let selected_id = component.data.values.join(",");

                sqlx::query!("DELETE FROM reminders WHERE FIND_IN_SET(id, ?)", selected_id)
                    .execute(&data.database)
                    .await
                    .unwrap();

                let reminders = Reminder::from_guild(
                    &ctx,
                    &data.database,
                    component.guild_id,
                    component.user.id,
                )
                .await;

                let resp = show_delete_page(&reminders, selector.page, selector.timezone);

                let _ = component
                    .create_interaction_response(&ctx, |f| {
                        f.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |d| {
                                send_as_initial_response(resp, d);
                                d
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::TodoPager(pager) => {
                if Some(component.user.id.0) == pager.user_id || pager.user_id.is_none() {
                    let values = if let Some(uid) = pager.user_id {
                        sqlx::query!(
                            "SELECT todos.id, value FROM todos
INNER JOIN users ON todos.user_id = users.id
WHERE users.user = ?",
                            uid,
                        )
                        .fetch_all(&data.database)
                        .await
                        .unwrap()
                        .iter()
                        .map(|row| (row.id as usize, row.value.clone()))
                        .collect::<Vec<(usize, String)>>()
                    } else if let Some(cid) = pager.channel_id {
                        sqlx::query!(
                            "SELECT todos.id, value FROM todos
INNER JOIN channels ON todos.channel_id = channels.id
WHERE channels.channel = ?",
                            cid,
                        )
                        .fetch_all(&data.database)
                        .await
                        .unwrap()
                        .iter()
                        .map(|row| (row.id as usize, row.value.clone()))
                        .collect::<Vec<(usize, String)>>()
                    } else {
                        sqlx::query!(
                            "SELECT todos.id, value FROM todos
INNER JOIN guilds ON todos.guild_id = guilds.id
WHERE guilds.guild = ?",
                            pager.guild_id,
                        )
                        .fetch_all(&data.database)
                        .await
                        .unwrap()
                        .iter()
                        .map(|row| (row.id as usize, row.value.clone()))
                        .collect::<Vec<(usize, String)>>()
                    };

                    let max_pages = max_todo_page(&values);

                    let resp = show_todo_page(
                        &values,
                        pager.next_page(max_pages),
                        pager.user_id,
                        pager.channel_id,
                        pager.guild_id,
                    );

                    let _ = component
                        .create_interaction_response(&ctx, |f| {
                            f.kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|d| {
                                    send_as_initial_response(resp, d);
                                    d
                                })
                        })
                        .await;
                } else {
                    let _ = component
                        .create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d| {
                                    d.flags(
                                        MessageFlags::EPHEMERAL,
                                    )
                                    .content("Only the user who performed the command can use these components")
                                })
                        })
                        .await;
                }
            }
            ComponentDataModel::TodoSelector(selector) => {
                if Some(component.user.id.0) == selector.user_id || selector.user_id.is_none() {
                    let selected_id = component.data.values.join(",");

                    sqlx::query!("DELETE FROM todos WHERE FIND_IN_SET(id, ?)", selected_id)
                        .execute(&data.database)
                        .await
                        .unwrap();

                    let values = sqlx::query!(
                    // fucking braindead mysql use <=> instead of = for null comparison
                    "SELECT id, value FROM todos WHERE user_id <=> ? AND channel_id <=> ? AND guild_id <=> ?",
                    selector.user_id,
                    selector.channel_id,
                    selector.guild_id,
                )
                .fetch_all(&data.database)
                .await
                .unwrap()
                .iter()
                .map(|row| (row.id as usize, row.value.clone()))
                .collect::<Vec<(usize, String)>>();

                    let resp = show_todo_page(
                        &values,
                        selector.page,
                        selector.user_id,
                        selector.channel_id,
                        selector.guild_id,
                    );

                    let _ = component
                        .create_interaction_response(&ctx, |f| {
                            f.kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|d| {
                                    send_as_initial_response(resp, d);
                                    d
                                })
                        })
                        .await;
                } else {
                    let _ = component
                        .create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d| {
                                    d.flags(
                                        MessageFlags::EPHEMERAL,
                                    )
                                    .content("Only the user who performed the command can use these components")
                                })
                        })
                        .await;
                }
            }
            ComponentDataModel::MacroPager(pager) => {
                let macros = data.command_macros(component.guild_id.unwrap()).await.unwrap();

                let max_page = max_macro_page(&macros);
                let page = pager.next_page(max_page);

                let resp = show_macro_page(&macros, page);

                let _ = component
                    .create_interaction_response(&ctx, |f| {
                        f.kind(InteractionResponseType::UpdateMessage).interaction_response_data(
                            |d| {
                                send_as_initial_response(resp, d);
                                d
                            },
                        )
                    })
                    .await;
            }
            ComponentDataModel::UndoReminder(undo_reminder) => {
                if component.user.id == undo_reminder.user_id {
                    let reminder =
                        Reminder::from_id(&data.database, undo_reminder.reminder_id).await;

                    if let Some(reminder) = reminder {
                        match reminder.delete(&data.database).await {
                            Ok(()) => {
                                let _ = component
                                    .create_interaction_response(&ctx, |f| {
                                        f.kind(InteractionResponseType::UpdateMessage)
                                            .interaction_response_data(|d| {
                                                d.embed(|e| {
                                                    e.title("Reminder Canceled")
                                                        .description(
                                                            "This reminder has been canceled.",
                                                        )
                                                        .color(*THEME_COLOR)
                                                })
                                                .components(|c| c)
                                            })
                                    })
                                    .await;
                            }
                            Err(e) => {
                                warn!("Error canceling reminder: {:?}", e);

                                let _ = component
                                    .create_interaction_response(&ctx, |f| {
                                        f.kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|d| {
                                                d.content(
                                                    "The reminder could not be canceled: it may have already been deleted. Check `/del`!")
                                                    .ephemeral(true)
                                            })
                                    })
                                    .await;
                            }
                        }
                    } else {
                        let _ = component
                            .create_interaction_response(&ctx, |f| {
                                f.kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|d| {
                                        d.content(
                                            "The reminder could not be canceled: it may have already been deleted. Check `/del`!")
                                            .ephemeral(true)
                                    })
                            })
                            .await;
                    }
                } else {
                    let _ = component
                        .create_interaction_response(&ctx, |f| {
                            f.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d| {
                                    d.content(
                                        "Only the user who performed the command can use this button.")
                                        .ephemeral(true)
                                })
                        })
                        .await;
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DelSelector {
    pub page: usize,
    pub timezone: Tz,
}

#[derive(Serialize, Deserialize)]
pub struct TodoSelector {
    pub page: usize,
    pub user_id: Option<u64>,
    pub channel_id: Option<u64>,
    pub guild_id: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct UndoReminder {
    pub user_id: serenity::UserId,
    pub reminder_id: u32,
}
