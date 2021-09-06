use regex::Captures;
use serenity::model::{channel::Message, guild::Guild, misc::Mentionable};

use crate::{consts::REGEX_CONTENT_SUBSTITUTION, models::reminder::errors::ContentError};

pub struct Content {
    pub content: String,
    pub tts: bool,
    pub attachment: Option<Vec<u8>>,
    pub attachment_name: Option<String>,
}

impl Content {
    pub fn new() -> Self {
        Self {
            content: "".to_string(),
            tts: false,
            attachment: None,
            attachment_name: None,
        }
    }

    pub async fn build<S: ToString>(content: S, message: &Message) -> Result<Self, ContentError> {
        if message.attachments.len() > 1 {
            Err(ContentError::TooManyAttachments)
        } else if let Some(attachment) = message.attachments.get(0) {
            if attachment.size > 8_000_000 {
                Err(ContentError::AttachmentTooLarge)
            } else if let Ok(attachment_bytes) = attachment.download().await {
                Ok(Self {
                    content: content.to_string(),
                    tts: false,
                    attachment: Some(attachment_bytes),
                    attachment_name: Some(attachment.filename.clone()),
                })
            } else {
                Err(ContentError::AttachmentDownloadFailed)
            }
        } else {
            Ok(Self {
                content: content.to_string(),
                tts: false,
                attachment: None,
                attachment_name: None,
            })
        }
    }

    pub fn substitute(&mut self, guild: Guild) {
        if self.content.starts_with("/tts ") {
            self.tts = true;
            self.content = self.content.split_off(5);
        }

        self.content = REGEX_CONTENT_SUBSTITUTION
            .replace(&self.content, |caps: &Captures| {
                if let Some(user) = caps.name("user") {
                    format!("<@{}>", user.as_str())
                } else if let Some(role_name) = caps.name("role") {
                    if let Some(role) = guild.role_by_name(role_name.as_str()) {
                        role.mention().to_string()
                    } else {
                        format!("<<{}>>", role_name.as_str().to_string())
                    }
                } else {
                    String::new()
                }
            })
            .to_string()
            .replace("<<everyone>>", "@everyone")
            .replace("<<here>>", "@here");
    }
}
