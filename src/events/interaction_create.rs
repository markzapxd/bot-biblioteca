use serenity::all::{Context, Interaction};

pub async fn handle(_ctx: Context, interaction: Interaction) {
    if let Interaction::Command(command) = interaction {
        tracing::info!("Received command: {}", command.data.name);
    }
}
