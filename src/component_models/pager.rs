use chrono_tz::Tz;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use serenity::{
    builder::CreateComponents,
    model::{id::UserId, interactions::message_component::ButtonStyle},
};

use crate::{component_models::ComponentDataModel, models::reminder::look_flags::LookFlags};

#[derive(Serialize, Deserialize)]
pub struct Pager<D> {
    pub page: usize,
    action: PageAction,
    pub data: D,
}

impl<D> Pager<D>
where
    D: Serialize + Clone,
{
    pub fn new(page: usize, data: D) -> Self {
        Self { page, action: PageAction::Refresh, data }
    }

    pub fn next_page(&self, max_pages: usize) -> usize {
        match self.action {
            PageAction::First => 0,
            PageAction::Previous => 0.max(self.page - 1),
            PageAction::Refresh => self.page,
            PageAction::Next => (max_pages - 1).min(self.page + 1),
            PageAction::Last => max_pages - 1,
        }
    }

    pub fn to_custom_id(&self) -> String {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        base64::encode(buf)
    }

    fn buttons(&self, page: usize) -> (Pager<D>, Pager<D>, Pager<D>, Pager<D>, Pager<D>) {
        (
            Pager { page, action: PageAction::First, data: self.data.clone() },
            Pager { page, action: PageAction::Previous, data: self.data.clone() },
            Pager { page, action: PageAction::Refresh, data: self.data.clone() },
            Pager { page, action: PageAction::Next, data: self.data.clone() },
            Pager { page, action: PageAction::Last, data: self.data.clone() },
        )
    }

    pub fn create_button_row(&self, max_pages: usize, comp: &mut CreateComponents) {
        let next_page = self.next_page(max_pages);

        let (page_first, page_prev, page_refresh, page_next, page_last) = self.buttons(next_page);

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
                b.label("🔁").style(ButtonStyle::Secondary).custom_id(page_refresh.to_custom_id())
            })
            .create_button(|b| {
                b.label("▶️")
                    .style(ButtonStyle::Secondary)
                    .custom_id(page_next.to_custom_id())
                    .disabled(next_page + 1 == max_pages)
            })
            .create_button(|b| {
                b.label("⏭️")
                    .style(ButtonStyle::Primary)
                    .custom_id(page_last.to_custom_id())
                    .disabled(next_page + 1 == max_pages)
            })
        });
    }
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum PageAction {
    First = 0,
    Previous = 1,
    Refresh = 2,
    Next = 3,
    Last = 4,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LookData {
    pub flags: LookFlags,
    pub timezone: Tz,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DelData {
    pub author_id: UserId,
    pub timezone: Tz,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TodoData {
    pub user_id: Option<u64>,
    pub channel_id: Option<u64>,
    pub guild_id: Option<u64>,
}
