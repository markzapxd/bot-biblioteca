use crate::models::{User, VoiceSession};
use serenity::all::{Colour, CreateEmbed};

pub fn success(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x2B2D31))
}

pub fn error(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x2B2D31))
}

pub fn info(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x2B2D31))
}

pub fn warning(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x2B2D31))
}

pub fn user_info(user_data: &User) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Informações do Usuário")
        .colour(Colour::new(0x2B2D31));
    embed = embed.field("ID do Usuário", &user_data.user_id, true);
    embed = embed.field("Privado", format!("{}", user_data.is_private_mode()), true);
    embed = embed.field(
        "Tempo Total em Voz",
        format!("{} ms", user_data.total_voice_time.unwrap_or(0)),
        true,
    );
    embed
}

pub fn voice_session(session: &VoiceSession) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Sessão de Voz")
        .colour(Colour::new(0x2B2D31));
    embed = embed.field("Usuário", &session.user_id, true);
    embed = embed.field("Canal", &session.channel_name, true);
    embed = embed.field("Duração", session.duration_formatted(), true);
    embed
}

pub fn member_join(member: &serenity::all::Member) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Membro Entrou")
        .thumbnail(member.user.face())
        .colour(Colour::new(0x2B2D31));
    if let Some(user) = &member.user.global_name {
        embed = embed.field("Usuário", user, true);
    }
    embed = embed.field("ID do Usuário", member.user.id.to_string(), true);
    embed
}

pub fn member_leave(member: &serenity::all::Member) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Membro Saiu")
        .thumbnail(member.user.face())
        .colour(Colour::new(0x2B2D31));
    if let Some(user) = &member.user.global_name {
        embed = embed.field("Usuário", user, true);
    }
    embed = embed.field("ID do Usuário", member.user.id.to_string(), true);
    embed
}

pub fn message_edit(old: &str, new: &str, author: &serenity::all::User) -> CreateEmbed {
    CreateEmbed::new()
        .title("Mensagem Editada")
        .thumbnail(author.face())
        .field("Autor", author.name.clone(), true)
        .field("Antes", old, false)
        .field("Depois", new, false)
        .colour(Colour::new(0x2B2D31))
}

pub fn message_delete(content: &str, author: &serenity::all::User, channel: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Mensagem Deletada")
        .thumbnail(author.face())
        .field("Autor", author.name.clone(), true)
        .field("Canal", channel, true)
        .field("Conteúdo", content, false)
        .colour(Colour::new(0x2B2D31))
}

pub fn raid_detected(guild: &str, reason: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Raid Detectada")
        .field("Servidor", guild, true)
        .field("Motivo", reason, true)
        .colour(Colour::new(0x2B2D31))
}

pub fn ticket_opened(user: &serenity::all::User, channel: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Ticket Aberto")
        .field("Usuário", user.name.clone(), true)
        .field("Canal", channel, true)
        .colour(Colour::new(0x2B2D31))
}

pub fn avatar_history(user_id: &str, url: &str, page: usize, total: usize) -> CreateEmbed {
    CreateEmbed::new()
        .title("Histórico de Avatares")
        .field("Usuário", user_id, true)
        .field("Página", format!("{} / {}", page, total), true)
        .image(url)
        .colour(Colour::new(0x2B2D31))
}
