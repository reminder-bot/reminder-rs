use regex_command_attr::command;
use serenity::{
    builder::{CreateEmbed, CreateInteractionResponse},
    client::Context,
    model::interactions::InteractionResponseType,
};

use crate::{
    component_models::pager::TodoPager,
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, SELECT_MAX_ENTRIES, THEME_COLOR},
    framework::{CommandInvoke, CommandOptions, CreateGenericResponse},
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
async fn todo(ctx: &Context, invoke: CommandInvoke, args: CommandOptions) {
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

        println!("{:?}", keys);

        match args.get("task") {
            Some(task) => {
                let task = task.to_string();

                sqlx::query!(
                    "INSERT INTO todos (user_id, channel_id, guild_id, value) VALUES (?, ?, ?, ?)",
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
                let values = sqlx::query!(
                    // fucking braindead mysql use <=> instead of = for null comparison
                    "SELECT value FROM todos WHERE user_id <=> ? AND channel_id <=> ? AND guild_id <=> ?",
                    keys.0,
                    keys.1,
                    keys.2,
                )
                .fetch_all(&pool)
                .await
                .unwrap()
                .iter()
                .map(|row| row.value.clone())
                .collect::<Vec<String>>();

                let resp = show_todo_page(&values, 0, keys.0, keys.1, keys.2);

                let interaction = invoke.interaction().unwrap();

                let _ = interaction
                    .create_interaction_response(&ctx, |r| {
                        *r = resp;
                        r.kind(InteractionResponseType::ChannelMessageWithSource)
                    })
                    .await
                    .unwrap();
            }
        }
    }
}

pub fn max_todo_page(todo_values: &[String]) -> usize {
    let mut rows = 0;
    let mut char_count = 0;

    todo_values.iter().enumerate().map(|(c, v)| format!("{}: {}", c, v)).fold(
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
    todo_values: &[String],
    page: usize,
    user_id: Option<u64>,
    channel_id: Option<u64>,
    guild_id: Option<u64>,
) -> CreateInteractionResponse {
    // let pager = TodoPager::new(page, user_id, channel_id, guild_id);

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

    let display_vec: Vec<String> = todo_values
        .iter()
        .enumerate()
        .map(|(c, v)| format!("`{}`: {}", c + 1, v))
        .skip_while(|p| {
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
        .take_while(|p| {
            rows += 1;
            char_count += p.len();

            char_count < EMBED_DESCRIPTION_MAX_LENGTH && rows <= SELECT_MAX_ENTRIES
        })
        .collect();

    let display = display_vec.join("\n");

    let title = if user_id.is_some() {
        "Your"
    } else if channel_id.is_some() {
        "Channel"
    } else {
        "Server"
    };

    let mut embed = CreateEmbed::default();
    embed
        .title(format!("{} Todo List", title))
        .description(display)
        .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
        .color(*THEME_COLOR);

    let mut response = CreateInteractionResponse::default();
    response.interaction_response_data(|d| d.embeds(vec![embed]));

    response
}
