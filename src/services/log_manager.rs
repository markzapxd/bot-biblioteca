use serenity::all::*;
use sqlx::PgPool;
use tracing::error;

use crate::errors::Result;

pub async fn send_log(ctx: &Context, guild_id: u64, embed: CreateEmbed, pool: &PgPool) -> Result<()> {
    let guild = match crate::repositories::guild_repo::find_by_id(pool, &guild_id.to_string()).await {
        Ok(Some(g)) => g,
        Ok(None) => return Ok(()),
        Err(e) => {
            error!("Failed to fetch guild config for logging: {}", e);
            return Err(e);
        }
    };

    if !guild.is_module_enabled("logs") {
        return Ok(());
    }

    if let Some(log_channel_id) = &guild.log_channel_id {
        if let Ok(channel_id) = log_channel_id.parse::<u64>() {
            if let Err(e) = ChannelId::new(channel_id).send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
                error!("Failed to send log message: {}", e);
            }
        }
    }

    Ok(())
}

pub async fn log_message_edit(ctx: &Context, old: &str, new: &str, author: &User, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = crate::embeds::message_edit(old, new, author);
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_message_delete(ctx: &Context, content: &str, author_name: &str, channel_name: &str, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Message Deleted")
        .field("Author", author_name, true)
        .field("Channel", channel_name, true)
        .field("Content", content, false)
        .colour(Colour::new(0xFF0000));
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_member_add(ctx: &Context, member: &Member, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = crate::embeds::member_join(member);
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_member_remove(ctx: &Context, member: &Member, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = crate::embeds::member_leave(member);
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_member_update(ctx: &Context, old_roles: &[RoleId], new_roles: &[RoleId], member: &Member, guild_id: u64, pool: &PgPool) -> Result<()> {
    let added: Vec<String> = new_roles.iter()
        .filter(|r| !old_roles.contains(r))
        .map(|r| format!("<@&{}>", r.get()))
        .collect();
    let removed: Vec<String> = old_roles.iter()
        .filter(|r| !new_roles.contains(r))
        .map(|r| format!("<@&{}>", r.get()))
        .collect();

    let mut description = format!("<@{}> roles updated", member.user.id.get());
    if !added.is_empty() {
        description.push_str(&format!("\n**Added:** {}", added.join(", ")));
    }
    if !removed.is_empty() {
        description.push_str(&format!("\n**Removed:** {}", removed.join(", ")));
    }

    let embed = CreateEmbed::new()
        .title("Member Updated")
        .description(description)
        .colour(Colour::new(0xFFFF00));
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_voice_join(ctx: &Context, user_name: &str, channel_name: &str, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Voice Channel Join")
        .field("User", user_name, true)
        .field("Channel", channel_name, true)
        .colour(Colour::new(0x00FF00));
    send_log(ctx, guild_id, embed, pool).await
}

pub async fn log_voice_leave(ctx: &Context, user_name: &str, channel_name: &str, duration: i64, guild_id: u64, pool: &PgPool) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Voice Channel Leave")
        .field("User", user_name, true)
        .field("Channel", channel_name, true)
        .field("Duration", crate::utils::time::format_duration(duration), true)
        .colour(Colour::new(0xFF0000));
    send_log(ctx, guild_id, embed, pool).await
}
