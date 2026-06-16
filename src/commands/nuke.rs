use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("nuke")
            .description("Clonar e recriar o canal atual no mesmo local")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let channel_id = interaction.channel_id;

    interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let channel = channel_id.to_channel(ctx).await?;
    if let Channel::Guild(guild_channel) = channel {
        let guild_id = guild_channel.guild_id;
        let name = guild_channel.name.clone();
        let kind = guild_channel.kind;
        let topic = guild_channel.topic.clone();
        let nsfw = guild_channel.nsfw;
        let bitrate = guild_channel.bitrate;
        let user_limit = guild_channel.user_limit;
        let position = guild_channel.position;
        let parent_id = guild_channel.parent_id;
        let rate_limit = guild_channel.rate_limit_per_user;
        let permission_overwrites = guild_channel.permission_overwrites.clone();

        let mut builder = CreateChannel::new(&name).kind(kind);
        if let Some(ref t) = topic { builder = builder.topic(t); }
        builder = builder.nsfw(nsfw);
        if kind == ChannelType::Voice {
            if let Some(b) = bitrate { builder = builder.bitrate(b as u32); }
            if let Some(u) = user_limit { builder = builder.user_limit(u as u32); }
        }
        builder = builder.position(position);
        if let Some(parent) = parent_id { builder = builder.category(parent); }
        if let Some(rate) = rate_limit { builder = builder.rate_limit_per_user(rate as u16); }
        builder = builder.permissions(permission_overwrites);

        let created = guild_id.create_channel(ctx, builder).await?;

        let _ = channel_id.delete(ctx).await;

        let embed = theme::success("Canal Recriado", &format!("Canal **#{}** foi clonado e recriado no mesmo local.", name));
        let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "nuke", embed).await;
        let mut msg = CreateMessage::new().embed(embed);
        if let Some(file) = attachment {
            msg = msg.add_file(file);
        }
        let _ = created.send_message(ctx, msg).await;
    }

    Ok(())
}
