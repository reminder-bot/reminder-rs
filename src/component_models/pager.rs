use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use serenity::{builder::CreateComponents, model::interactions::message_component::ButtonStyle};

use crate::{component_models::ComponentDataModel, models::reminder::look_flags::LookFlags};

pub trait Pager {
    fn next_page(&self, max_pages: usize) -> usize;

    fn create_button_row(&self, max_pages: usize, comp: &mut CreateComponents);
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

#[derive(Serialize, Deserialize)]
pub struct LookPager {
    pub flags: LookFlags,
    pub page: usize,
    action: PageAction,
    pub timezone: Tz,
}

impl Pager for LookPager {
    fn next_page(&self, max_pages: usize) -> usize {
        match self.action {
            PageAction::First => 0,
            PageAction::Previous => 0.max(self.page - 1),
            PageAction::Refresh => self.page,
            PageAction::Next => (max_pages - 1).min(self.page + 1),
            PageAction::Last => max_pages - 1,
        }
    }

    fn create_button_row(&self, max_pages: usize, comp: &mut CreateComponents) {
        let next_page = self.next_page(max_pages);

        let (page_first, page_prev, page_refresh, page_next, page_last) =
            LookPager::buttons(self.flags, next_page, self.timezone);

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

impl LookPager {
    pub fn new(flags: LookFlags, timezone: Tz) -> Self {
        Self { flags, page: 0, action: PageAction::First, timezone }
    }

    pub fn buttons(
        flags: LookFlags,
        page: usize,
        timezone: Tz,
    ) -> (
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
    ) {
        (
            ComponentDataModel::LookPager(LookPager {
                flags,
                page,
                action: PageAction::First,
                timezone,
            }),
            ComponentDataModel::LookPager(LookPager {
                flags,
                page,
                action: PageAction::Previous,
                timezone,
            }),
            ComponentDataModel::LookPager(LookPager {
                flags,
                page,
                action: PageAction::Refresh,
                timezone,
            }),
            ComponentDataModel::LookPager(LookPager {
                flags,
                page,
                action: PageAction::Next,
                timezone,
            }),
            ComponentDataModel::LookPager(LookPager {
                flags,
                page,
                action: PageAction::Last,
                timezone,
            }),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct DelPager {
    pub page: usize,
    action: PageAction,
    pub timezone: Tz,
}

impl Pager for DelPager {
    fn next_page(&self, max_pages: usize) -> usize {
        match self.action {
            PageAction::First => 0,
            PageAction::Previous => 0.max(self.page - 1),
            PageAction::Refresh => self.page,
            PageAction::Next => (max_pages - 1).min(self.page + 1),
            PageAction::Last => max_pages - 1,
        }
    }

    fn create_button_row(&self, max_pages: usize, comp: &mut CreateComponents) {
        let next_page = self.next_page(max_pages);

        let (page_first, page_prev, page_refresh, page_next, page_last) =
            DelPager::buttons(next_page, self.timezone);

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

impl DelPager {
    pub fn new(timezone: Tz) -> Self {
        Self { page: 0, action: PageAction::First, timezone }
    }

    pub fn buttons(
        page: usize,
        timezone: Tz,
    ) -> (
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
        ComponentDataModel,
    ) {
        (
            ComponentDataModel::DelPager(DelPager { page, action: PageAction::First, timezone }),
            ComponentDataModel::DelPager(DelPager { page, action: PageAction::Previous, timezone }),
            ComponentDataModel::DelPager(DelPager { page, action: PageAction::Refresh, timezone }),
            ComponentDataModel::DelPager(DelPager { page, action: PageAction::Next, timezone }),
            ComponentDataModel::DelPager(DelPager { page, action: PageAction::Last, timezone }),
        )
    }
}
