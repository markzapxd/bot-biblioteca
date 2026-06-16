use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub client_id: u64,
    pub database_url: String,
    pub log_level: String,
    pub port: u16,
    pub owner_id: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        Ok(Self {
            discord_token: env::var("DISCORD_TOKEN").map_err(|_| "DISCORD_TOKEN not set")?,
            client_id: env::var("CLIENT_ID")
                .map_err(|_| "CLIENT_ID not set")?
                .parse()
                .map_err(|_| "CLIENT_ID must be a valid u64")?,
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://bibliotecario:bibliotecario@localhost:5432/bibliotecario".into()
            }),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3050".into())
                .parse()
                .unwrap_or(3050),
            owner_id: env::var("OWNER_ID")
                .unwrap_or_else(|_| "852879300336418837".into())
                .parse()
                .unwrap_or(852879300336418837),
        })
    }
}
