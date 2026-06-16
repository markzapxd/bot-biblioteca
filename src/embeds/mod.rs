use crate::models::{User, VoiceSession};
use serenity::all::{Colour, CreateEmbed};

pub fn success(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x00FF00))
}

pub fn error(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0xFF0000))
}

pub fn info(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0x3498DB))
}

pub fn warning(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Colour::new(0xFFFF00))
}

pub fn user_info(user_data: &User) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("User Info")
        .colour(Colour::new(0x3498DB));
    embed = embed.field("User ID", &user_data.user_id, true);
    embed = embed.field("Private", format!("{}", user_data.is_private_mode()), true);
    embed = embed.field(
        "Total Voice Time",
        format!("{} ms", user_data.total_voice_time.unwrap_or(0)),
        true,
    );
    embed
}

pub fn voice_session(session: &VoiceSession) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Voice Session")
        .colour(Colour::new(0x3498DB));
    embed = embed.field("User", &session.user_id, true);
    embed = embed.field("Channel", &session.channel_name, true);
    embed = embed.field("Duration", session.duration_formatted(), true);
    embed
}

pub fn member_join(member: &serenity::all::Member) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Member Joined")
        .colour(Colour::new(0x00FF00));
    if let Some(user) = &member.user.global_name {
        embed = embed.field("User", user, true);
    }
    embed = embed.field("User ID", member.user.id.to_string(), true);
    embed
}

pub fn member_leave(member: &serenity::all::Member) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Member Left")
        .colour(Colour::new(0xFF0000));
    if let Some(user) = &member.user.global_name {
        embed = embed.field("User", user, true);
    }
    embed = embed.field("User ID", member.user.id.to_string(), true);
    embed
}

pub fn message_edit(old: &str, new: &str, author: &serenity::all::User) -> CreateEmbed {
    CreateEmbed::new()
        .title("Message Edited")
        .field("Author", author.name.clone(), true)
        .field("Before", old, false)
        .field("After", new, false)
        .colour(Colour::new(0xFFFF00))
}

pub fn message_delete(content: &str, author: &serenity::all::User, channel: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Message Deleted")
        .field("Author", author.name.clone(), true)
        .field("Channel", channel, true)
        .field("Content", content, false)
        .colour(Colour::new(0xFF0000))
}

pub fn raid_detected(guild: &str, reason: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Raid Detected")
        .field("Guild", guild, true)
        .field("Reason", reason, true)
        .colour(Colour::new(0xFF0000))
}

pub fn ticket_opened(user: &serenity::all::User, channel: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Ticket Opened")
        .field("User", user.name.clone(), true)
        .field("Channel", channel, true)
        .colour(Colour::new(0x00FF00))
}

pub fn avatar_history(user_id: &str, url: &str, page: usize, total: usize) -> CreateEmbed {
    CreateEmbed::new()
        .title("Avatar History")
        .field("User", user_id, true)
        .field("Page", format!("{} / {}", page, total), true)
        .image(url)
        .colour(Colour::new(0x3498DB))
}
