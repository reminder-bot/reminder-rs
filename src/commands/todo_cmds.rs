use poise::CreateReply;

use crate::{
    component_models::{
        pager::{Pager, TodoPager},
        ComponentDataModel, TodoSelector,
    },
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, SELECT_MAX_ENTRIES, THEME_COLOR},
    models::CtxData,
    Context, Error,
};

/// Manage todo lists
#[poise::command(
    slash_command,
    rename = "todo",
    identifying_name = "todo_base",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Manage the server todo list
#[poise::command(
    slash_command,
    rename = "server",
    guild_only = true,
    identifying_name = "todo_guild_base",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_guild_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an item to the server todo list
#[poise::command(
    slash_command,
    rename = "add",
    guild_only = true,
    identifying_name = "todo_guild_add",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_guild_add(
    ctx: Context<'_>,
    #[description = "The task to add to the todo list"] task: String,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO todos (guild_id, value)
VALUES (?, ?)",
        ctx.guild_id().unwrap().0,
        task
    )
    .execute(&ctx.data().database)
    .await
    .unwrap();

    ctx.say("Item added to todo list").await?;

    Ok(())
}

/// View and remove from the server todo list
#[poise::command(
    slash_command,
    rename = "view",
    guild_only = true,
    identifying_name = "todo_guild_view",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_guild_view(ctx: Context<'_>) -> Result<(), Error> {
    let values = sqlx::query!(
        "SELECT todos.id, value FROM todos WHERE guild_id = ?",
        ctx.guild_id().unwrap().0,
    )
    .fetch_all(&ctx.data().database)
    .await
    .unwrap()
    .iter()
    .map(|row| (row.id as usize, row.value.clone()))
    .collect::<Vec<(usize, String)>>();

    let resp = show_todo_page(&values, 0, None, None, ctx.guild_id().map(|g| g.0));

    ctx.send(|r| {
        *r = resp;
        r
    })
    .await?;

    Ok(())
}

/// Manage the channel todo list
#[poise::command(
    slash_command,
    rename = "channel",
    guild_only = true,
    identifying_name = "todo_channel_base",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_channel_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an item to the channel todo list
#[poise::command(
    slash_command,
    rename = "add",
    guild_only = true,
    identifying_name = "todo_channel_add",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_channel_add(
    ctx: Context<'_>,
    #[description = "The task to add to the todo list"] task: String,
) -> Result<(), Error> {
    // ensure channel is cached
    let _ = ctx.channel_data().await;

    sqlx::query!(
        "INSERT INTO todos (guild_id, channel_id, value)
VALUES (?, (SELECT id FROM channels WHERE channel = ?), ?)",
        ctx.guild_id().unwrap().0,
        ctx.channel_id().0,
        task
    )
    .execute(&ctx.data().database)
    .await
    .unwrap();

    ctx.say("Item added to todo list").await?;

    Ok(())
}

/// View and remove from the channel todo list
#[poise::command(
    slash_command,
    rename = "view",
    guild_only = true,
    identifying_name = "todo_channel_view",
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn todo_channel_view(ctx: Context<'_>) -> Result<(), Error> {
    let values = sqlx::query!(
        "SELECT todos.id, value FROM todos
INNER JOIN channels ON todos.channel_id = channels.id
WHERE channels.channel = ?",
        ctx.channel_id().0,
    )
    .fetch_all(&ctx.data().database)
    .await
    .unwrap()
    .iter()
    .map(|row| (row.id as usize, row.value.clone()))
    .collect::<Vec<(usize, String)>>();

    let resp =
        show_todo_page(&values, 0, None, Some(ctx.channel_id().0), ctx.guild_id().map(|g| g.0));

    ctx.send(|r| {
        *r = resp;
        r
    })
    .await?;

    Ok(())
}

/// Manage your personal todo list
#[poise::command(slash_command, rename = "user", identifying_name = "todo_user_base")]
pub async fn todo_user_base(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an item to your personal todo list
#[poise::command(slash_command, rename = "add", identifying_name = "todo_user_add")]
pub async fn todo_user_add(
    ctx: Context<'_>,
    #[description = "The task to add to the todo list"] task: String,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO todos (user_id, value)
VALUES ((SELECT id FROM users WHERE user = ?), ?)",
        ctx.author().id.0,
        task
    )
    .execute(&ctx.data().database)
    .await
    .unwrap();

    ctx.say("Item added to todo list").await?;

    Ok(())
}

/// View and remove from your personal todo list
#[poise::command(slash_command, rename = "view", identifying_name = "todo_user_view")]
pub async fn todo_user_view(ctx: Context<'_>) -> Result<(), Error> {
    let values = sqlx::query!(
        "SELECT todos.id, value FROM todos
INNER JOIN users ON todos.user_id = users.id
WHERE users.user = ?",
        ctx.author().id.0,
    )
    .fetch_all(&ctx.data().database)
    .await
    .unwrap()
    .iter()
    .map(|row| (row.id as usize, row.value.clone()))
    .collect::<Vec<(usize, String)>>();

    let resp = show_todo_page(&values, 0, Some(ctx.author().id.0), None, None);

    ctx.send(|r| {
        *r = resp;
        r
    })
    .await?;

    Ok(())
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
) -> CreateReply {
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
        let mut reply = CreateReply::default();

        reply.embed(|e| {
            e.title(format!("{} Todo List", title))
                .description("Todo List Empty!")
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        });

        reply
    } else {
        let todo_selector =
            ComponentDataModel::TodoSelector(TodoSelector { page, user_id, channel_id, guild_id });

        let mut reply = CreateReply::default();

        reply
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
                                        .description(disp.split_once(' ').unwrap_or(("", "")).1)
                                });
                            }

                            opt
                        })
                    })
                })
            });

        reply
    }
}
