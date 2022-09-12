use poise::CreateReply;

use crate::{
    component_models::pager::{MacroPager, Pager},
    consts::{EMBED_DESCRIPTION_MAX_LENGTH, THEME_COLOR},
    models::{command_macro::CommandMacro, CtxData},
    Context, Error,
};

/// List recorded macros
#[poise::command(
    slash_command,
    rename = "list",
    guild_only = true,
    default_member_permissions = "MANAGE_GUILD",
    identifying_name = "list_macro"
)]
pub async fn list_macro(ctx: Context<'_>) -> Result<(), Error> {
    let macros = ctx.command_macros().await?;

    let resp = show_macro_page(&macros, 0);

    ctx.send(|m| {
        *m = resp;
        m
    })
    .await?;

    Ok(())
}

pub fn max_macro_page<U, E>(macros: &[CommandMacro<U, E>]) -> usize {
    let mut skipped_char_count = 0;

    macros
        .iter()
        .map(|m| {
            if let Some(description) = &m.description {
                format!("**{}**\n- *{}*\n- Has {} commands", m.name, description, m.commands.len())
            } else {
                format!("**{}**\n- Has {} commands", m.name, m.commands.len())
            }
        })
        .fold(1, |mut pages, p| {
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH {
                skipped_char_count = p.len();
                pages += 1;
            }

            pages
        })
}

pub fn show_macro_page<U, E>(macros: &[CommandMacro<U, E>], page: usize) -> CreateReply {
    let pager = MacroPager::new(page);

    if macros.is_empty() {
        let mut reply = CreateReply::default();

        reply.embed(|e| {
            e.title("Macros")
                .description("No Macros Set Up. Use `/macro record` to get started.")
                .color(*THEME_COLOR)
        });

        return reply;
    }

    let pages = max_macro_page(macros);

    let mut page = page;
    if page >= pages {
        page = pages - 1;
    }

    let mut char_count = 0;
    let mut skipped_char_count = 0;

    let mut skipped_pages = 0;

    let display_vec: Vec<String> = macros
        .iter()
        .map(|m| {
            if let Some(description) = &m.description {
                format!("**{}**\n- *{}*\n- Has {} commands", m.name, description, m.commands.len())
            } else {
                format!("**{}**\n- Has {} commands", m.name, m.commands.len())
            }
        })
        .skip_while(|p| {
            skipped_char_count += p.len();

            if skipped_char_count > EMBED_DESCRIPTION_MAX_LENGTH {
                skipped_char_count = p.len();
                skipped_pages += 1;
            }

            skipped_pages < page
        })
        .take_while(|p| {
            char_count += p.len();

            char_count < EMBED_DESCRIPTION_MAX_LENGTH
        })
        .collect::<Vec<String>>();

    let display = display_vec.join("\n");

    let mut reply = CreateReply::default();

    reply
        .embed(|e| {
            e.title("Macros")
                .description(display)
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        })
        .components(|comp| {
            pager.create_button_row(pages, comp);

            comp
        });

    reply
}
