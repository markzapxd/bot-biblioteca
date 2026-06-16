use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("lockdown")
            .description("Assign a role to all members")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(CommandOptionType::Role, "role", "Role to assign").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let role_id = interaction.data.options.iter().find(|o| o.name == "role").and_then(|o| o.value.as_role_id()).ok_or(crate::errors::BotError::Validation("Role required".into()))?;

    interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let members = guild_id.members(ctx, None, None).await.unwrap_or_default();
    let mut count = 0u32;
    for mut m in members {
        if !m.roles.contains(&role_id) {
            if m.add_role(ctx, role_id).await.is_ok() {
                count += 1;
            }
        }
    }

    let embed = embeds::success("Lockdown Complete", &format!("Assigned role to {} members.", count));
    interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
