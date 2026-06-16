use chrono::{DateTime, Utc};

pub fn time_ago(date: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(date);
    let seconds = diff.num_seconds();
    if seconds < 0 {
        return "no futuro".to_string();
    }
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let months = days / 30;
    let years = months / 12;
    if years > 0 {
        return format!("há {} {}", years, if years == 1 { "ano" } else { "anos" });
    }
    if months > 0 {
        return format!(
            "há {} {}",
            months,
            if months == 1 { "mês" } else { "meses" }
        );
    }
    if days > 0 {
        return format!("há {} {}", days, if days == 1 { "dia" } else { "dias" });
    }
    if hours > 0 {
        return format!("há {} {}", hours, if hours == 1 { "hora" } else { "horas" });
    }
    if minutes > 0 {
        return format!(
            "há {} {}",
            minutes,
            if minutes == 1 { "minuto" } else { "minutos" }
        );
    }
    format!(
        "há {} {}",
        seconds,
        if seconds == 1 { "segundo" } else { "segundos" }
    )
}

pub fn format_duration(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }
    parts.join(" ")
}

pub fn format_duration_long(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!(
            "{} {}",
            hours,
            if hours == 1 { "hour" } else { "hours" }
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{} {}",
            minutes,
            if minutes == 1 { "minute" } else { "minutes" }
        ));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!(
            "{} {}",
            seconds,
            if seconds == 1 { "second" } else { "seconds" }
        ));
    }
    parts.join(", ")
}
