use chrono::naive::NaiveDateTime;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod guild;
pub mod user;

fn name_default() -> String {
    "Reminder".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    attachment: Option<Vec<u8>>,
    attachment_name: Option<String>,
    avatar: Option<String>,
    #[serde(with = "string")]
    channel: u64,
    content: String,
    embed_author: String,
    embed_author_url: Option<String>,
    embed_color: u32,
    embed_description: String,
    embed_footer: String,
    embed_footer_url: Option<String>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_title: String,
    enabled: i8,
    expires: Option<NaiveDateTime>,
    interval_seconds: Option<u32>,
    interval_months: Option<u32>,
    #[serde(default = "name_default")]
    name: String,
    pin: i8,
    restartable: i8,
    tts: i8,
    #[serde(default)]
    uid: String,
    username: Option<String>,
    utc_time: NaiveDateTime,
}

// https://github.com/serde-rs/json/issues/329#issuecomment-305608405
mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
pub struct DeleteReminder {
    uid: String,
}

#[get("/")]
pub async fn dashboard_home(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    if cookies.get_private("userid").is_some() {
        let map: HashMap<&str, String> = HashMap::new();
        Ok(Template::render("dashboard", &map))
    } else {
        Err(Redirect::to("/login/discord"))
    }
}
