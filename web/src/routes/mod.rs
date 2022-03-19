pub mod dashboard;
pub mod login;

use std::collections::HashMap;

use rocket::request::FlashMessage;
use rocket_dyn_templates::Template;

#[get("/")]
pub async fn index(flash: Option<FlashMessage<'_>>) -> Template {
    let mut map: HashMap<&str, String> = HashMap::new();

    if let Some(message) = flash {
        map.insert("flashed_message", message.message().to_string());
        map.insert("flashed_grade", message.kind().to_string());
    }

    Template::render("index", &map)
}

#[get("/ret?<to>")]
pub async fn return_to_same_site(to: &str) -> Template {
    let mut map: HashMap<&str, String> = HashMap::new();

    map.insert("to", to.to_string());

    Template::render("return", &map)
}

#[get("/cookies")]
pub async fn cookies() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("cookies", &map)
}

#[get("/privacy")]
pub async fn privacy() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("privacy", &map)
}

#[get("/terms")]
pub async fn terms() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("terms", &map)
}

#[get("/help")]
pub async fn help() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("help", &map)
}

#[get("/help/timezone")]
pub async fn help_timezone() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("help_timezone", &map)
}
