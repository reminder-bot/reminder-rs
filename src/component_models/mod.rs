use std::io::Cursor;

use chrono_tz::Tz;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::Channel,
        id::{ChannelId, RoleId},
        interactions::{
            message_component::{ButtonStyle, MessageComponentInteraction},
            InteractionResponseType,
        },
    },
};

use crate::{
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    models::{
        reminder::{look_flags::LookFlags, Reminder},
        user_data::UserData,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ComponentDataModel {
    Restrict(Restrict),
    LookPager(LookPager),
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
                println!("{:?}", component.data.values);
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
                    .div_ceil(EMBED_DESCRIPTION_MAX_LENGTH) as u16;

                let channel_name =
                    if let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx) {
                        Some(channel.name)
                    } else {
                        None
                    };

                let next_page = match pager.action {
                    PageAction::First => 0,
                    PageAction::Previous => 0.max(pager.page - 1),
                    PageAction::Next => (pages - 1).min(pager.page + 1),
                    PageAction::Last => pages - 1,
                };

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

                let page_first = ComponentDataModel::LookPager(LookPager {
                    flags: flags.clone(),
                    page: next_page,
                    action: PageAction::First,
                    timezone: pager.timezone,
                });
                let page_prev = ComponentDataModel::LookPager(LookPager {
                    flags: flags.clone(),
                    page: next_page,
                    action: PageAction::Previous,
                    timezone: pager.timezone,
                });
                let page_next = ComponentDataModel::LookPager(LookPager {
                    flags: flags.clone(),
                    page: next_page,
                    action: PageAction::Next,
                    timezone: pager.timezone,
                });
                let page_last = ComponentDataModel::LookPager(LookPager {
                    flags: flags.clone(),
                    page: next_page,
                    action: PageAction::Last,
                    timezone: pager.timezone,
                });

                let mut embed = CreateEmbed::default();
                embed
                    .title(format!(
                        "Reminders{}",
                        channel_name.map_or(String::new(), |n| format!(" on #{}", n))
                    ))
                    .description(display)
                    .footer(|f| f.text(format!("Page {} of {}", next_page + 1, pages)))
                    .color(*THEME_COLOR);

                let _ =
                    component
                        .create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|response| {
                                    response.embeds(vec![embed]).components(|comp| {
                                        comp.create_action_row(|row| {
                                            row.create_button(|b| {
                                                b.label("⏮️")
                                                    .style(ButtonStyle::Primary)
                                                    .custom_id(page_first.to_custom_id())
                                                    .disabled(next_page == 0)
                                            })
                                            .create_button(|b| {
                                                b.label("◀️")
                                                    .style(ButtonStyle::Secondary)
                                                    .custom_id(page_prev.to_custom_id())
                                                    .disabled(next_page == 0)
                                            })
                                            .create_button(|b| {
                                                b.label("▶️")
                                                    .style(ButtonStyle::Secondary)
                                                    .custom_id(page_next.to_custom_id())
                                                    .disabled(next_page + 1 == pages)
                                            })
                                            .create_button(|b| {
                                                b.label("⏭️")
                                                    .style(ButtonStyle::Primary)
                                                    .custom_id(page_last.to_custom_id())
                                                    .disabled(next_page + 1 == pages)
                                            })
                                        })
                                    })
                                })
                        })
                        .await;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Restrict {
    pub role_id: RoleId,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PageAction {
    First = 0,
    Previous = 1,
    Next = 2,
    Last = 3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LookPager {
    pub flags: LookFlags,
    pub page: u16,
    pub action: PageAction,
    pub timezone: Tz,
}
