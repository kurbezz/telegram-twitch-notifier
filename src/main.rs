pub mod bot;
pub mod config;
pub mod subscription_manager;
pub mod twitch_webhook;

use std::sync::Arc;

use bot::start_bot;
use subscription_manager::SubscriptionManager;
use twitch_webhook::start_twitch_webhook;

#[tokio::main]
async fn main() {
    let subscription_manager = Arc::new(SubscriptionManager::new());

    subscription_manager.init().await;

    let (_, webhook_result) = tokio::join!(
        start_bot(subscription_manager.clone()),
        start_twitch_webhook(subscription_manager)
    );

    if let Err(e) = webhook_result {
        eprintln!("Error in webhook: {:?}", e);
    }
}
