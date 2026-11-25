use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;
use serenity::all::*;
use sqlx::PgPool;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(CreateCommand::new("card").description("Sistema de cards personalizados"));
}

pub async fn handle(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    _guild_cache: &GuildCache,
) -> Result<()> {
    let member = interaction
        .member
        .as_ref()
        .ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_id = interaction
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let guild_config = _guild_cache
        .get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin_or_staff(user_id, member, &guild_config)?;

    let cards = crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id_str).await?;
    let embed = build_main_embed(&cards);
    let rows = build_main_rows(!cards.is_empty());

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

fn build_main_embed(cards: &[crate::models::CustomCard]) -> CreateEmbed {
    let mut desc = String::new();
    if cards.is_empty() {
        desc.push_str("Nenhum card salvo neste servidor.\n");
    } else {
        desc.push_str("Cards salvos:\n");
        for card in cards.iter().take(25) {
            desc.push_str(&format!(
                "`{}` — {}\n",
                truncate(&card.name, 40),
                truncate(&card.title, 80)
            ));
        }
    }
    desc.push_str("\nSelecione uma acao abaixo.");

    CreateEmbed::new()
        .title("Cards Personalizados")
        .description(desc)
        .colour(Colour::new(0x2B2D31))
}

fn build_main_rows(has_cards: bool) -> Vec<CreateActionRow> {
    let mut buttons = vec![CreateButton::new("card_action_create")
        .label("Criar")
        .style(ButtonStyle::Primary)];
    if has_cards {
        buttons.push(
            CreateButton::new("card_action_edit")
                .label("Editar")
                .style(ButtonStyle::Secondary),
        );
        buttons.push(
            CreateButton::new("card_action_send")
                .label("Enviar")
                .style(ButtonStyle::Secondary),
        );
        buttons.push(
            CreateButton::new("card_action_preview")
                .label("Preview")
                .style(ButtonStyle::Secondary),
        );
        buttons.push(
            CreateButton::new("card_action_delete")
                .label("Deletar")
                .style(ButtonStyle::Danger),
        );
    }
    vec![CreateActionRow::Buttons(buttons)]
}

fn build_card_select_menu(
    cards: &[crate::models::CustomCard],
    action: &str,
) -> Vec<CreateActionRow> {
    build_card_page(cards, action, 0)
}

fn truncate(text: &str, limit: usize) -> String {
    let mut units = 0;
    text.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= limit
        })
        .collect()
}

fn build_card_page(
    cards: &[crate::models::CustomCard],
    action: &str,
    page: usize,
) -> Vec<CreateActionRow> {
    let page = page.min(cards.len().saturating_sub(1) / 25);
    let options: Vec<CreateSelectMenuOption> = cards
        .iter()
        .skip(page * 25)
        .take(25)
        .map(|c| {
            CreateSelectMenuOption::new(truncate(&c.name, 100), c.id.to_string())
                .description(truncate(&c.title, 100))
        })
        .collect();

    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("card_select_{}", action),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Selecione um card"),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("card_page_{}_{}", action, page.saturating_sub(1)))
                .label("Anterior")
                .disabled(page == 0),
            CreateButton::new(format!("card_page_{}_{}", action, page + 1))
                .label("Proxima")
                .disabled((page + 1) * 25 >= cards.len()),
            CreateButton::new("card_action_back")
                .label("Voltar")
                .style(ButtonStyle::Secondary),
        ]),
    ]
}

pub async fn handle_create_button(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let modal = build_card_modal("card_create", None);
    component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;
    Ok(())
}

pub async fn handle_edit_button(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id.to_string()).await?;

    if cards.is_empty() {
        component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Nenhum card para editar.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let embed = build_main_embed(&cards);
    let rows = build_card_select_menu(&cards, "edit");

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

pub async fn handle_send_button(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id.to_string()).await?;

    if cards.is_empty() {
        component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Nenhum card para enviar.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let embed = build_main_embed(&cards);
    let rows = build_card_select_menu(&cards, "send");

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

pub async fn handle_preview_button(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id.to_string()).await?;

    if cards.is_empty() {
        component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Nenhum card para visualizar.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let embed = build_main_embed(&cards);
    let rows = build_card_select_menu(&cards, "preview");

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

pub async fn handle_delete_button(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id.to_string()).await?;

    if cards.is_empty() {
        component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Nenhum card para deletar.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let embed = build_main_embed(&cards);
    let rows = build_card_select_menu(&cards, "delete");

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

pub async fn handle_back_button(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id.to_string()).await?;

    let embed = build_main_embed(&cards);
    let rows = build_main_rows(!cards.is_empty());

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

pub async fn handle_select_edit(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return Err(BotError::Validation("Invalid selection".into())),
    };

    let card = crate::repositories::custom_card_repo::find_by_guild_id(
        pool,
        &guild_id_str,
        parse_card_id(&selected)?,
    )
    .await?;
    let card = match card {
        Some(c) => c,
        None => {
            component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Card `{}` nao encontrado.", selected))
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let modal = build_card_modal(&format!("card_edit_{}", selected), Some(&card));
    component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;
    Ok(())
}

pub async fn handle_select_send(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let channel_id = component.channel_id;

    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return Err(BotError::Validation("Invalid selection".into())),
    };

    let card = crate::repositories::custom_card_repo::find_by_guild_id(
        pool,
        &guild_id_str,
        parse_card_id(&selected)?,
    )
    .await?;
    match card {
        Some(card) => {
            let embed = crate::embeds::custom_card::build(&card);
            channel_id
                .send_message(ctx, CreateMessage::new().embed(embed))
                .await?;

            let cards =
                crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id_str).await?;
            let embed = build_main_embed(&cards);
            let rows = build_main_rows(!cards.is_empty());

            component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .components(rows),
                    ),
                )
                .await?;
        }
        None => {
            component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Card `{}` nao encontrado.", selected))
                            .ephemeral(true),
                    ),
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn handle_select_preview(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return Err(BotError::Validation("Invalid selection".into())),
    };

    let card = crate::repositories::custom_card_repo::find_by_guild_id(
        pool,
        &guild_id_str,
        parse_card_id(&selected)?,
    )
    .await?;
    match card {
        Some(card) => {
            let embed = crate::embeds::custom_card::preview(&card);
            component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .ephemeral(true),
                    ),
                )
                .await?;
        }
        None => {
            component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Card `{}` nao encontrado.", selected))
                            .ephemeral(true),
                    ),
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn handle_select_delete(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let guild_id = component
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return Err(BotError::Validation("Invalid selection".into())),
    };

    let deleted = crate::repositories::custom_card_repo::delete_by_id(
        pool,
        &guild_id_str,
        parse_card_id(&selected)?,
    )
    .await?;

    let cards = crate::repositories::custom_card_repo::list_by_guild(pool, &guild_id_str).await?;
    let embed = build_main_embed(&cards);
    let rows = build_main_rows(!cards.is_empty());

    let content = if deleted {
        format!("Card `{}` deletado.", selected)
    } else {
        format!("Card `{}` nao encontrado.", selected)
    };

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

fn build_card_modal(custom_id: &str, card: Option<&crate::models::CustomCard>) -> CreateModal {
    let name_input = CreateInputText::new(InputTextStyle::Short, "Nome (unico)", "card_name")
        .placeholder("ex: atualizacao")
        .max_length(100)
        .required(true);
    let name_input = if let Some(c) = card {
        name_input.value(&c.name)
    } else {
        name_input
    };

    let title_input = CreateInputText::new(InputTextStyle::Short, "Titulo", "card_title")
        .placeholder("ex: Nova Atualizacao")
        .max_length(256)
        .required(true);
    let title_input = if let Some(c) = card {
        title_input.value(&c.title)
    } else {
        title_input
    };

    let desc_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Descricao (opcional)",
        "card_description",
    )
    .placeholder("Texto descritivo do card...")
    .max_length(4000)
    .required(false);
    let desc_input = if let Some(c) = card {
        if let Some(ref d) = c.description {
            desc_input.value(d)
        } else {
            desc_input
        }
    } else {
        desc_input
    };

    let image_input = CreateInputText::new(
        InputTextStyle::Short,
        "URL da Imagem/GIF (opcional)",
        "card_image",
    )
    .placeholder("https://exemplo.com/imagem.png")
    .max_length(1024)
    .required(false);
    let image_input = if let Some(c) = card {
        if let Some(ref u) = c.image_url {
            image_input.value(u)
        } else {
            image_input
        }
    } else {
        image_input
    };

    let color_input =
        CreateInputText::new(InputTextStyle::Short, "Cor em Hex (opcional)", "card_color")
            .placeholder("#2B2D31")
            .max_length(16)
            .required(false);
    let color_input = if let Some(c) = card {
        if let Some(ref col) = c.color {
            color_input.value(col)
        } else {
            color_input
        }
    } else {
        color_input
    };

    CreateModal::new(
        custom_id,
        if card.is_some() {
            "Editar Card"
        } else {
            "Criar Card"
        },
    )
    .components(vec![
        CreateActionRow::InputText(name_input),
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(desc_input),
        CreateActionRow::InputText(image_input),
        CreateActionRow::InputText(color_input),
    ])
}

fn extract_modal_value(rows: &[ActionRowComponent], custom_id: &str) -> Option<String> {
    rows.iter().find_map(|row| {
        if let ActionRowComponent::InputText(input) = row {
            if input.custom_id == custom_id {
                input.value.clone()
            } else {
                None
            }
        } else {
            None
        }
    })
}

pub async fn handle_modal_submit(
    ctx: &Context,
    modal: &ModalInteraction,
    pool: &PgPool,
) -> Result<()> {
    let custom_id = &modal.data.custom_id;
    let guild_id = modal
        .guild_id
        .ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let user_id = modal.user.id.get();

    let rows: Vec<ActionRowComponent> = modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.clone())
        .collect();

    let name = extract_modal_value(&rows, "card_name")
        .ok_or(BotError::Validation("Nome e obrigatorio".into()))?;
    let title = extract_modal_value(&rows, "card_title")
        .ok_or(BotError::Validation("Titulo e obrigatorio".into()))?;
    let description = extract_modal_value(&rows, "card_description");
    let image_url = extract_modal_value(&rows, "card_image");
    let color = extract_modal_value(&rows, "card_color");

    if custom_id == "card_create" {
        let card = crate::repositories::custom_card_repo::create(
            pool,
            &guild_id_str,
            &name,
            &title,
            description.as_deref(),
            image_url.as_deref(),
            color.as_deref(),
            None,
            &user_id.to_string(),
        )
        .await?;

        let embed = crate::embeds::custom_card::preview(&card);
        modal
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Card `{}` criado com sucesso!", name))
                        .embed(embed)
                        .ephemeral(true),
                ),
            )
            .await?;
    } else if custom_id.starts_with("card_edit_") {
        let id = parse_card_id(custom_id.strip_prefix("card_edit_").unwrap_or_default())?;
        let card = crate::repositories::custom_card_repo::replace(
            pool,
            &guild_id_str,
            id,
            crate::repositories::custom_card_repo::Replacement {
                name: &name,
                title: &title,
                description: description.as_deref(),
                image: image_url.as_deref(),
                color: color.as_deref(),
            },
        )
        .await?
        .ok_or_else(|| BotError::NotFound("Card nao encontrado".into()))?;

        let embed = crate::embeds::custom_card::preview(&card);
        modal
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Card `{}` atualizado com sucesso!", name))
                        .embed(embed)
                        .ephemeral(true),
                ),
            )
            .await?;
    }

    Ok(())
}

fn parse_card_id(value: &str) -> Result<i32> {
    value
        .parse::<i32>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| BotError::Validation("Invalid card ID".into()))
}

pub async fn handle_page(
    ctx: &Context,
    component: &ComponentInteraction,
    pool: &PgPool,
) -> Result<()> {
    let suffix = component
        .data
        .custom_id
        .strip_prefix("card_page_")
        .ok_or_else(|| BotError::Validation("Invalid page".into()))?;
    let (action, page) = suffix
        .rsplit_once('_')
        .ok_or_else(|| BotError::Validation("Invalid page".into()))?;
    if !["edit", "send", "preview", "delete"].contains(&action) {
        return Err(BotError::Validation("Invalid card action".into()));
    }
    let page = page
        .parse::<usize>()
        .map_err(|_| BotError::Validation("Invalid page".into()))?;
    let guild = component
        .guild_id
        .ok_or_else(|| BotError::Validation("Guild only".into()))?;
    let cards =
        crate::repositories::custom_card_repo::list_by_guild(pool, &guild.to_string()).await?;
    let rows = if cards.is_empty() {
        build_main_rows(false)
    } else {
        build_card_page(&cards, action, page)
    };
    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_main_embed(&cards))
                    .components(rows),
            ),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod task7_tests {
    use super::*;
    fn cards() -> Vec<crate::models::CustomCard> {
        (1..=26)
            .map(|id| crate::models::CustomCard {
                id,
                guild_id: "1".into(),
                name: "名".repeat(100),
                title: "😀".repeat(256),
                description: None,
                image_url: None,
                color: None,
                footer: None,
                created_by: "1".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .collect()
    }
    #[test]
    fn task7_menu_limits_and_stable_ids() {
        let value = serde_json::to_value(build_card_select_menu(&cards(), "edit")).unwrap();
        let options = value[0]["components"][0]["options"].as_array().unwrap();
        assert_eq!(options.len(), 25);
        assert_eq!(options[0]["value"], "1");
        let last = serde_json::to_value(build_card_page(&cards(), "edit", usize::MAX)).unwrap();
        assert_eq!(
            last[0]["components"][0]["options"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(last[0]["components"][0]["options"][0]["value"], "26");
        let modal = serde_json::to_value(build_card_modal(
            &format!("card_edit_{}", i32::MAX),
            Some(&cards()[0]),
        ))
        .unwrap();
        assert!(modal["custom_id"].as_str().unwrap().len() <= 100);
        assert!(options[0]["description"].as_str().unwrap().chars().count() <= 100);
    }
    #[test]
    fn task7_main_embed_is_bounded() {
        let value = serde_json::to_value(build_main_embed(&cards())).unwrap();
        assert!(
            value["description"]
                .as_str()
                .unwrap()
                .encode_utf16()
                .count()
                <= 4096
        );
    }
}
