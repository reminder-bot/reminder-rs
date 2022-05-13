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

#[get("/")]
pub async fn help() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("help", &map)
}

#[get("/timezone")]
pub async fn help_timezone() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/timezone", &map)
}

#[get("/create_reminder")]
pub async fn help_create_reminder() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/create_reminder", &map)
}

#[get("/delete_reminder")]
pub async fn help_delete_reminder() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/delete_reminder", &map)
}

#[get("/timers")]
pub async fn help_timers() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/timers", &map)
}

#[get("/todo_lists")]
pub async fn help_todo_lists() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/todo_lists", &map)
}

#[get("/macros")]
pub async fn help_macros() -> Template {
    let map: HashMap<&str, String> = HashMap::new();
    Template::render("support/macros", &map)
}
