use crate::errors::{BotError, Result};
use crate::models::Guild;
use serenity::all::{Member, Permissions};
use std::env;

#[cfg(test)]
mod task2_tests {
    use super::*;

    #[test]
    fn unconfigured_owner_is_not_a_hardcoded_account() {
        if env::var("OWNER_ID").is_err() {
            assert!(!is_owner(852879300336418837));
        }
    }

    #[test]
    fn moderator_without_action_permission_is_denied() {
        assert!(!action_allowed(Action::Ban, Permissions::MODERATE_MEMBERS));
        assert!(!action_allowed(Action::Kick, Permissions::MODERATE_MEMBERS));
    }

    #[test]
    fn equal_or_higher_target_is_denied() {
        assert!(!hierarchy_allowed(false, 5, 5));
        assert!(!hierarchy_allowed(false, 5, 6));
        assert!(hierarchy_allowed(false, 6, 5));
        assert!(hierarchy_allowed(true, 0, 5));
    }

    #[test]
    fn revoked_permission_is_denied_on_recheck() {
        assert!(action_allowed(Action::Ban, Permissions::BAN_MEMBERS));
        assert!(!action_allowed(Action::Ban, Permissions::empty()));
    }
}

pub fn is_owner(user_id: u64) -> bool {
    env::var("OWNER_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|id: &u64| *id != 0)
        == Some(user_id)
}

pub fn is_admin(member: &Member, guild_config: &Guild) -> bool {
    if member
        .permissions
        .map_or(false, |p| p.contains(Permissions::ADMINISTRATOR))
    {
        return true;
    }
    if let Some(admin_role_id) = &guild_config.admin_role_id {
        if let Ok(role_id) = admin_role_id.parse::<u64>() {
            return member.roles.iter().any(|r| r.get() == role_id);
        }
    }
    false
}

pub fn is_moderator(member: &Member) -> bool {
    member
        .permissions
        .map_or(false, |p| p.contains(Permissions::MODERATE_MEMBERS))
}

pub fn is_staff(member: &Member, guild_config: &Guild) -> bool {
    let mut has_role = false;
    if let Some(staff_role_id) = &guild_config.staff_role_id {
        if let Ok(role_id) = staff_role_id.parse::<u64>() {
            if member.roles.iter().any(|r| r.get() == role_id) {
                has_role = true;
            }
        }
    }
    if let Some(admin_role_id) = &guild_config.admin_role_id {
        if let Ok(role_id) = admin_role_id.parse::<u64>() {
            if member.roles.iter().any(|r| r.get() == role_id) {
                has_role = true;
            }
        }
    }
    has_role
}

pub fn require_owner(user_id: u64) -> Result<()> {
    if !is_owner(user_id) {
        return Err(BotError::Unauthorized("Apenas o dono do bot".to_string()));
    }
    Ok(())
}

pub fn require_admin(user_id: u64, member: &Member, guild_config: &Guild) -> Result<()> {
    if is_owner(user_id) || is_admin(member, guild_config) || is_staff(member, guild_config) {
        return Ok(());
    }
    Err(BotError::Unauthorized("Apenas administradores".to_string()))
}

pub fn require_moderator(user_id: u64, member: &Member) -> Result<()> {
    if is_owner(user_id) || is_moderator(member) {
        return Ok(());
    }
    Err(BotError::Unauthorized("Apenas moderadores".to_string()))
}

pub fn require_staff(user_id: u64, member: &Member, guild_config: &Guild) -> Result<()> {
    if is_owner(user_id) || is_staff(member, guild_config) {
        return Ok(());
    }
    Err(BotError::Unauthorized("Apenas membros da staff".to_string()))
}

pub fn require_admin_or_staff(user_id: u64, member: &Member, guild_config: &Guild) -> Result<()> {
    if is_owner(user_id) || is_admin(member, guild_config) || is_staff(member, guild_config) {
        return Ok(());
    }
    Err(BotError::Unauthorized("Apenas administradores ou staff".to_string()))
}

pub fn is_immune(user_id: u64, member: &Member, guild_config: &Guild) -> bool {
    is_owner(user_id) || is_admin(member, guild_config) || is_staff(member, guild_config)
}

/// Each moderation operation requires its own Discord permission.
#[derive(Clone, Copy)]
pub enum Action { Ban, Kick, Timeout, Warn, Configure }

fn action_allowed(action: Action, permissions: Permissions) -> bool {
    permissions.contains(Permissions::ADMINISTRATOR) || permissions.contains(match action {
        Action::Ban => Permissions::BAN_MEMBERS,
        Action::Kick => Permissions::KICK_MEMBERS,
        Action::Timeout | Action::Warn => Permissions::MODERATE_MEMBERS,
        Action::Configure => Permissions::ADMINISTRATOR,
    })
}

#[cfg(test)]
mod review_regressions {
    use super::*;

    #[test]
    fn only_confirmed_unknown_member_allows_nonmember_ban() {
        assert!(allows_missing_target(Action::Ban, 404, 10007));
        assert!(!allows_missing_target(Action::Kick, 404, 10007));
        assert!(!allows_missing_target(Action::Ban, 403, 50013));
        assert!(!allows_missing_target(Action::Ban, 404, 10004));
    }

    #[tokio::test]
    async fn panel_permission_revocation_is_rechecked_by_callback_guard() {
        let current = std::cell::Cell::new(Permissions::BAN_MEMBERS);
        assert!(fresh_action_check(Action::Ban, || async { Ok(current.get()) }).await.is_ok());
        current.set(Permissions::empty());
        assert!(fresh_action_check(Action::Ban, || async { Ok(current.get()) }).await.is_err());
        assert!(fresh_action_check(Action::Ban, || async { Err(BotError::Unauthorized("unavailable".into())) }).await.is_err());
    }
}

fn allows_missing_target(action: Action, status: u16, code: isize) -> bool {
    matches!(action, Action::Ban) && status == 404 && code == 10007
}

async fn fresh_action_check<F, Fut>(action: Action, fetch: F) -> Result<()>
where F: FnOnce() -> Fut, Fut: std::future::Future<Output = Result<Permissions>> {
    if action_allowed(action, fetch().await?) { Ok(()) }
    else { Err(BotError::Unauthorized("Permissão insuficiente para esta ação".into())) }
}

fn hierarchy_allowed(guild_owner: bool, caller: u16, target: u16) -> bool {
    guild_owner || caller > target
}

/// Fetch fresh guild roles and membership; REST Member.permissions is optional and is not trusted.
pub async fn authorize(
    ctx: &serenity::all::Context,
    guild_id: serenity::all::GuildId,
    caller: serenity::all::UserId,
    action: Action,
    target: Option<serenity::all::UserId>,
) -> Result<()> {
    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let member = guild_id.member(&ctx.http, caller).await?;
    // Bot ownership does not confer guild moderation rights or bypass hierarchy.
    fresh_action_check(action, || async {
        let current = guild_id.member(&ctx.http, caller).await?;
        #[allow(deprecated)]
        Ok(guild.member_permissions(&current))
    }).await?;
    if let Some(target_id) = target {
        if target_id == caller || target_id == guild.owner_id || target_id == ctx.cache.current_user().id {
            return Err(BotError::Unauthorized("Alvo protegido".into()));
        }
        let target_member = match guild_id.member(&ctx.http, target_id).await {
            Ok(member) => Some(member),
            Err(serenity::Error::Http(serenity::http::HttpError::UnsuccessfulRequest(ref response)))
                if allows_missing_target(action, response.status_code.as_u16(), response.error.code) => None,
            Err(error) => return Err(error.into()),
        };
        let bot_id = ctx.cache.current_user().id;
        let bot = guild_id.member(&ctx.http, bot_id).await?;
        #[allow(deprecated)]
        let bot_permissions = guild.member_permissions(&bot);
        fresh_action_check(action, || async { Ok(bot_permissions) }).await?;
        // Only a confirmed Discord Unknown Member response skips member hierarchy.
        // Caller and bot action permissions have both been checked above.
        let Some(target_member) = target_member else { return Ok(()); };
        let position = |m: &Member| -> Result<u16> {
            let mut highest = 0;
            for id in &m.roles {
                let role = guild.roles.get(id).ok_or_else(|| BotError::Unauthorized("Cargo desconhecido".into()))?;
                highest = highest.max(role.position);
            }
            Ok(highest)
        };
        if !hierarchy_allowed(caller == guild.owner_id, position(&member)?, position(&target_member)?) {
            return Err(BotError::Unauthorized("Hierarquia do alvo não permite esta ação".into()));
        }
        if !hierarchy_allowed(false, position(&bot)?, position(&target_member)?) {
            return Err(BotError::Unauthorized("Permissão ou hierarquia do bot insuficiente".into()));
        }
        #[allow(deprecated)]
        if matches!(action, Action::Timeout) && guild.member_permissions(&target_member).contains(Permissions::ADMINISTRATOR) {
            return Err(BotError::Unauthorized("Administrador não pode ser silenciado".into()));
        }
    }
    Ok(())
}

/// Private history is self-only unless current guild administrator privileges are verified.
pub async fn can_view_profile(ctx: &serenity::all::Context, guild: Option<serenity::all::GuildId>, caller: serenity::all::UserId, target: u64, private: bool) -> bool {
    if caller.get() == target { return true; }
    let Some(guild_id) = guild else { return false; };
    if guild_id.member(&ctx.http, caller).await.is_err() { return false; }
    if !private { return true; }
    authorize(ctx, guild_id, caller, Action::Configure, None).await.is_ok()
}

pub async fn require_profile(ctx: &serenity::all::Context, interaction: &serenity::all::ComponentInteraction, pool: &sqlx::PgPool, target: u64) -> Result<bool> {
    let user = crate::repositories::user_repo::find_by_id(pool, &target.to_string()).await?;
    let private = user.map_or(true, |u| u.is_private_mode());
    if can_view_profile(ctx, interaction.guild_id, interaction.user.id, target, private).await { return Ok(true); }
    interaction.create_response(&ctx.http, serenity::all::CreateInteractionResponse::Message(
        serenity::all::CreateInteractionResponseMessage::new().content("Este perfil é privado.").ephemeral(true)
    )).await?;
    Ok(false)
}

pub async fn visible_voice_sessions(ctx: &serenity::all::Context, guild_id: serenity::all::GuildId, caller: serenity::all::UserId, sessions: Vec<crate::models::VoiceSession>) -> Result<Vec<crate::models::VoiceSession>> {
    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let member = guild_id.member(&ctx.http, caller).await?;
    let channels = guild_id.channels(&ctx.http).await?;
    Ok(sessions.into_iter().filter(|session| {
        session.guild_id == guild_id.to_string() && session.channel_id.parse::<u64>().ok()
            .and_then(|id| channels.get(&serenity::all::ChannelId::new(id)))
            .map_or(false, |channel| guild.user_permissions_in(channel, &member).contains(Permissions::VIEW_CHANNEL))
    }).collect())
}
