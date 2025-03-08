pub mod config;
pub mod repositories;
pub mod subscription_manager;
pub mod telegram_bot;
pub mod twitch_webhook;
pub mod web_app;

use std::sync::Arc;

use subscription_manager::SubscriptionManager;
use telegram_bot::start_telegram_bot;
use twitch_webhook::start_twitch_webhook;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let subscription_manager = Arc::new(SubscriptionManager::new());

    subscription_manager.load().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    let (_, webhook_result) = tokio::join!(
        start_telegram_bot(subscription_manager.clone()),
        start_twitch_webhook(subscription_manager)
    );

    if let Err(e) = webhook_result {
        tracing::error!("Error starting Twitch webhook: {}", e);
    }
}
