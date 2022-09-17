use poise::CreateReply;

use crate::{
    component_models::pager::{MacroPager, Pager},
    consts::THEME_COLOR,
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
    ((macros.len() as f64) / 25.0).ceil() as usize
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

    let lower = (page * 25).min(macros.len());
    let upper = ((page + 1) * 25).min(macros.len());

    let fields = macros[lower..upper].iter().map(|m| {
        if let Some(description) = &m.description {
            (
                m.name.clone(),
                format!("*{}*\n- Has {} commands", description, m.commands.len()),
                true,
            )
        } else {
            (m.name.clone(), format!("- Has {} commands", m.commands.len()), true)
        }
    });

    let mut reply = CreateReply::default();

    reply
        .embed(|e| {
            e.title("Macros")
                .fields(fields)
                .footer(|f| f.text(format!("Page {} of {}", page + 1, pages)))
                .color(*THEME_COLOR)
        })
        .components(|comp| {
            pager.create_button_row(pages, comp);

            comp
        });

    reply
}
