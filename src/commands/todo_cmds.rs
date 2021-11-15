use regex_command_attr::command;
use serenity::client::Context;

use crate::{
    component_models::{
        pager::{Pager, TodoPager},
        ComponentDataModel, TodoSelector,
    },
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, SELECT_MAX_ENTRIES, THEME_COLOR},
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse},
    hooks::CHECK_GUILD_PERMISSIONS_HOOK,
    SQLPool,
};

#[command]
#[description("Manage todo lists")]
#[subcommandgroup("server")]
#[description("Manage the server todo list")]
#[subcommand("add")]
#[description("Add an item to the server todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from the server todo list")]
#[subcommandgroup("channel")]
#[description("Manage the channel todo list")]
#[subcommand("add")]
#[description("Add to the channel todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from the channel todo list")]
#[subcommandgroup("user")]
#[description("Manage your personal todo list")]
#[subcommand("add")]
#[description("Add to your personal todo list")]
#[arg(
    name = "task",
    description = "The task to add to the todo list",
    kind = "String",
    required = true
)]
#[subcommand("view")]
#[description("View and remove from your personal todo list")]
#[hook(CHECK_GUILD_PERMISSIONS_HOOK)]
async fn todo(ctx: &Context, invoke: &mut CommandInvoke, args: CommandOptions) {
    if invoke.guild_id().is_none() && args.subcommand_group != Some("user".to_string()) {
        let _ = invoke
            .respond(
                &ctx,
                CreateGenericResponse::new().content("Please use `/todo user` in direct messages"),
            )
            .await;
    } else {
        let pool = ctx.data.read().await.get::<SQLPool>().cloned().unwrap();

        let keys = match args.subcommand_group.as_ref().unwrap().as_str() {
            "server" => (None, None, invoke.guild_id().map(|g| g.0)),
            "channel" => (None, Some(invoke.channel_id().0), invoke.guild_id().map(|g| g.0)),
            _ => (Some(invoke.author_id().0), None, None),
        };

        match args.get("task") {
            Some(task) => {
                let task = task.to_string();

                sqlx::query!(
                    "INSERT INTO todos (user_id, channel_id, guild_id, value) VALUES ((SELECT id FROM users WHERE user = ?), (SELECT id FROM channels WHERE channel = ?), (SELECT id FROM guilds WHERE guild = ?), ?)",
                    keys.0,
                    keys.1,
                    keys.2,
                    task
                )
                .execute(&pool)
                .await
                .unwrap();

                let _ = invoke
                    .respond(&ctx, CreateGenericResponse::new().content("Item added to todo list"))
                    .await;
            }
            None => {
                let values = if let Some(uid) = keys.0 {
                    sqlx::query!(
                        "SELECT todos.id, value FROM todos
INNER JOIN users ON todos.user_id = users.id
WHERE users.user = ?",
                        uid,
                    )
                    .fetch_all(&pool)
                    .await
                    .unwrap()
                    .iter()
                    .map(|row| (row.id as usize, row.value.clone()))
                    .collect::<Vec<(usize, String)>>()
                } else if let Some(cid) = keys.1 {
                    sqlx::query!(
                        "SELECT todos.id, value FROM todos
INNER JOIN channels ON todos.channel_id = channels.id
WHERE channels.channel = ?",
                        cid,
                    )
                    .fetch_all(&pool)
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
                        keys.2,
                    )
                    .fetch_all(&pool)
                    .await
                    .unwrap()
                    .iter()
                    .map(|row| (row.id as usize, row.value.clone()))
                    .collect::<Vec<(usize, String)>>()
                };

                let resp = show_todo_page(&values, 0, keys.0, keys.1, keys.2);

                invoke.respond(&ctx, resp).await.unwrap();
            }
        }
    }
}

pub fn max_todo_page(todo_values: &[(usize, String)]) -> usize {
    let mut rows = 0;
    let mut char_count = 0;

    todo_values.iter().enumerate().map(|(c, (_, v))| format!("{}: {}", c, v)).fold(
        1,
        |mut pages, text| {
            rows += 1;
            char_count += text.len();

            if char_count > EMBED_DESCRIPTION_MAX_LENGTH || rows > SELECT_MAX_ENTRIES {
                rows = 1;
                char_count = text.len();
                pages += 1;
            }

            pages
        },
    )
}

pub fn show_todo_page(
    todo_values: &[(usize, String)],
    page: usize,
    user_id: Option<u64>,
    channel_id: Option<u64>,
    guild_id: Option<u64>,
) -> CreateGenericResponse {
    let pager = TodoPager::new(page, user_id, channel_id, guild_id);

    let pages = max_todo_page(todo_values);
    let mut page = page;
    if page >= pages {
        page = pages - 1;
    }

    let mut char_count = 0;
    let mut rows = 0;
    let mut skipped_rows = 0;
    let mut skipped_char_count = 0;
    let mut first_num = 0;

    let mut skipped_pages = 0;

    let (todo_ids, display_vec): (Vec<usize>, Vec<String>) = todo_values
        .iter()
        .enumerate()
        .map(|(c, (i, v))| (i, format!("`{}`: {}", c + 1, v)))
        .skip_while(|(_, p)| {
            first_num += 1;
            skipped_rows += 1;
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH
                || skipped_rows > SELECT_MAX_ENTRIES
            {
                skipped_rows = 1;
                skipped_char_count = p.len();
                skipped_pages += 1;
            }

            skipped_pages < page
        })
        .take_while(|(_, p)| {
            rows += 1;
            char_count += p.len();

            char_count < EMBED_DESCRIPTION_MAX_LENGTH && rows <= SELECT_MAX_ENTRIES
        })
        .unzip();

    let display = display_vec.join("\n");

    let title = if user_id.is_some() {
        "Your"
    } else if channel_id.is_some() {
        "Channel"
    } else {
        "Server"
    };

    if todo_ids.is_empty() {
        CreateGenericResponse::new().embed(|e| {
            e.title(format!("{} Todo List", title))
                .description("Todo List Empty!")
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        })
    } else {
        let todo_selector =
            ComponentDataModel::TodoSelector(TodoSelector { page, user_id, channel_id, guild_id });

        CreateGenericResponse::new()
            .embed(|e| {
                e.title(format!("{} Todo List", title))
                    .description(display)
                    .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                    .color(*THEME_COLOR)
            })
            .components(|comp| {
                pager.create_button_row(pages, comp);

                comp.create_action_row(|row| {
                    row.create_select_menu(|menu| {
                        menu.custom_id(todo_selector.to_custom_id()).options(|opt| {
                            for (count, (id, disp)) in todo_ids.iter().zip(&display_vec).enumerate()
                            {
                                opt.create_option(|o| {
                                    o.label(format!("Mark {} complete", count + first_num))
                                        .value(id)
                                        .description(disp.split_once(" ").unwrap_or(("", "")).1)
                                });
                            }

                            opt
                        })
                    })
                })
            })
    }
}
