use std::io::Cursor;

use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use serenity::model::{
    id::{ChannelId, RoleId},
    interactions::message_component::MessageComponentInteraction,
};

use crate::models::reminder::look_flags::LookFlags;

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

    pub async fn act(&self, component: MessageComponentInteraction) {
        match self {
            ComponentDataModel::Restrict(restrict) => {
                println!("{:?}", component.data.values);
            }
            ComponentDataModel::LookPager(pager) => {}
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Restrict {
    pub role_id: RoleId,
}

#[derive(Deserialize, Serialize)]
pub struct LookPager {
    pub flags: LookFlags,
    pub page_request: u16,
}
