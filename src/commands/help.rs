use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(CreateCommand::new("help").description("Listar comandos disponiveis"));
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let user_id = interaction.user.id.get();
    let is_admin = interaction.member.as_ref().map_or(false, |m| {
        permissions::is_owner(user_id) || permissions::is_admin(m)
    });

    let embed = CreateEmbed::new()
        .title("Bibliotecario — Ajuda")
        .description("Selecione uma categoria abaixo para ver os comandos disponiveis.")
        .colour(Colour::new(0x2B2D31));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "help", embed).await;

    let mut buttons = vec![
        CreateButton::new("help_membro").label("Comandos de Membro").style(ButtonStyle::Primary),
    ];
    if is_admin {
        buttons.push(CreateButton::new("help_admin").label("Comandos de Admin").style(ButtonStyle::Danger));
    }

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)])
        .ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_membro(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Comandos — Membros")
        .colour(Colour::new(0x2B2D31))
        .field("Geral", "`/help` — Mostrar esta ajuda\n`/info` — Informacoes do servidor\n`/lookup` — Buscar informacoes de um usuario", false)
        .field("Usuario", "`/userinfo` / `/ficha` — Ficha tactica do usuario\n`/names` / `/nomes` — Historico de nomes\n`/privacy` / `/privacidade` — Alternar modo privado", false)
        .field("Voz", "`/stats` / `/ranking` — Ranking de tempo em voz\n`/lastcall` / `/ultimochamada` — Ultima sessao de voz\n`/voicehistory` / `/historicovoz` — Historico completo", false);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "help", embed).await;

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(vec![
            CreateButton::new("help_back").label("Voltar").style(ButtonStyle::Secondary),
        ])]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_admin(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    handle_admin_p1(ctx, component).await
}

async fn build_admin_p1() -> CreateEmbed {
    let desc = format!(
        "**── Moderacao ──**\n\
         `/admin ban` — Banir usuario\n\
         `/admin kick` — Expulsar usuario\n\
         `/admin mute` — Silenciar usuario\n\
         `/admin unmute` — Remover silencio\n\
         `/admin warn` — Advertir usuario\n\
         \n\
         **── Mensagens ──**\n\
         `/clearuser` / `/limpar` — Limpar mensagens\n\
         `/nuke` — Clonar e recriar o canal\n\
         \n\
         **── Seguranca ──**\n\
         `/verify` — Exibir painel de verificacao\n\
         `/raidmode` / `/modoraid` — Ativar/desativar modo raid\n\
         `/lockdown` — Atribuir cargo a todos membros"
    );
    CreateEmbed::new()
        .title("Comandos — Admin (1/2)")
        .description(desc)
        .colour(Colour::new(0x2B2D31))
}

async fn build_admin_p2() -> CreateEmbed {
    let desc = format!(
        "**── Modulos & Setup ──**\n\
         `/setup` / `/configurar` — Configurar sistemas\n\
         `/modulos` — Gerenciar modulos ativos\n\
         `/sync` — Re-registrar comandos\n\
         \n\
         **── Tickets ──**\n\
         `/ticketpanel` — Postar painel de tickets\n\
         `/addticket` — Adicionar usuario ao ticket\n\
         `/removeticket` — Remover usuario do ticket\n\
         `/closeticket` — Fechar ticket\n\
         `/claimticket` — Assumir ticket\n\
         \n\
         **── Bot ──**\n\
         `/botstats` — Estatisticas do bot\n\
         `/shutdown` — Desligar o bot\n\
         `!reloadslash` — Recarregar comandos"
    );
    CreateEmbed::new()
        .title("Comandos — Admin (2/2)")
        .description(desc)
        .colour(Colour::new(0x2B2D31))
}

pub async fn handle_admin_p1(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let member = component.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = component.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = build_admin_p1().await;
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "help", embed).await;

    let buttons = vec![
        CreateButton::new("help_admin_p2").label("Proximo →").style(ButtonStyle::Secondary),
        CreateButton::new("help_back").label("Voltar").style(ButtonStyle::Danger),
    ];

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_admin_p2(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let member = component.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = component.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = build_admin_p2().await;
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "help", embed).await;

    let buttons = vec![
        CreateButton::new("help_admin_p1").label("← Anterior").style(ButtonStyle::Secondary),
        CreateButton::new("help_back").label("Voltar").style(ButtonStyle::Danger),
    ];

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_back(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let user_id = component.user.id.get();
    let is_admin = component.member.as_ref().map_or(false, |m| {
        permissions::is_owner(user_id) || permissions::is_admin(m)
    });

    let embed = CreateEmbed::new()
        .title("Bibliotecario — Ajuda")
        .description("Selecione uma categoria abaixo para ver os comandos disponiveis.")
        .colour(Colour::new(0x2B2D31));
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "help", embed).await;

    let mut buttons = vec![
        CreateButton::new("help_membro").label("Comandos de Membro").style(ButtonStyle::Primary),
    ];
    if is_admin {
        buttons.push(CreateButton::new("help_admin").label("Comandos de Admin").style(ButtonStyle::Danger));
    }

    let mut msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}
