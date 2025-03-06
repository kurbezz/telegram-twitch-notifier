use once_cell::sync::Lazy;

pub struct Config {
    pub bot_token: String,

    pub telegram_webhook_url: String,
    pub telegram_webhook_port: u16,

    pub twitch_client_id: String,
    pub twitch_client_secret: String,

    pub twitch_signing_secret: String,

    pub twitch_webhook_url: String,
    pub twitch_webhook_port: u16,

    pub mongodb_connection_string: String,
}

impl Config {
    fn load() -> Self {
        Self {
            bot_token: std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set"),

            telegram_webhook_url: std::env::var("TELEGRAM_WEBHOOK_URL")
                .expect("TELEGRAM_WEBHOOK_URL is not set"),
            telegram_webhook_port: std::env::var("TELEGRAM_WEBHOOK_PORT")
                .expect("TELEGRAM_WEBHOOK_PORT is not set")
                .parse()
                .expect("TELEGRAM_WEBHOOK_PORT is not a valid u16"),

            twitch_client_id: std::env::var("TWITCH_CLIENT_ID")
                .expect("TWITCH_CLIENT_ID is not set"),
            twitch_client_secret: std::env::var("TWITCH_CLIENT_SECRET")
                .expect("TWITCH_CLIENT_SECRET is not set"),

            twitch_signing_secret: std::env::var("TWITCH_SIGNING_SECRET")
                .expect("TWITCH_SIGNING_SECRET is not set"),

            twitch_webhook_url: std::env::var("TWITCH_WEBHOOK_URL")
                .expect("TWITCH_WEBHOOK_URL is not set"),
            twitch_webhook_port: std::env::var("TWITCH_WEBHOOK_PORT")
                .expect("TWITCH_WEBHOOK_PORT is not set")
                .parse()
                .expect("TWITCH_WEBHOOK_PORT is not a valid u16"),

            mongodb_connection_string: std::env::var("MONGODB_CONNECTION_STRING")
                .expect("MONGODB_CONNECTION_STRING is not set"),
        }
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(Config::load);
