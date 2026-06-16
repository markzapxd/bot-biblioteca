use crate::errors::{BotError, Result};
use crate::models::Guild;
use serenity::all::{Member, Permissions};
use std::env;

pub fn is_owner(user_id: u64) -> bool {
    env::var("OWNER_ID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(852879300336418837)
        == user_id
}

pub fn is_admin(member: &Member) -> bool {
    member
        .permissions
        .map_or(false, |p| p.contains(Permissions::ADMINISTRATOR))
}

pub fn is_moderator(member: &Member) -> bool {
    member
        .permissions
        .map_or(false, |p| p.contains(Permissions::MODERATE_MEMBERS))
}

pub fn is_staff(member: &Member, guild_config: &Guild) -> bool {
    if let Some(staff_role_id) = &guild_config.staff_role_id {
        let staff_role_id_num: u64 = staff_role_id.parse().unwrap_or(0);
        member.roles.iter().any(|r| r.get() == staff_role_id_num)
    } else {
        false
    }
}

pub fn require_owner(user_id: u64) -> Result<()> {
    if !is_owner(user_id) {
        return Err(BotError::Unauthorized("Owner only".to_string()));
    }
    Ok(())
}

pub fn require_admin(member: &Member) -> Result<()> {
    if !is_admin(member) {
        return Err(BotError::Unauthorized("Admin only".to_string()));
    }
    Ok(())
}

pub fn is_immune(member: &Member, guild_config: &Guild) -> bool {
    if let Some(owner_id) = member.guild_id.map(|g| g.get()) {
        if member.user.id.get() == owner_id {
            return true;
        }
    }
    is_admin(member)
}
