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
    if is_owner(user_id) || is_admin(member, guild_config) {
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
    is_owner(user_id) || is_admin(member, guild_config)
}
