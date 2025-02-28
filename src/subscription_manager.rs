use std::collections::{HashMap, HashSet};

use tokio::sync::RwLock;

pub struct SubscriptionManager {
    pub subscriptions: RwLock<HashMap<String, HashSet<u64>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn init(&self) {
        tracing::debug!("Initializing subscription manager");
    }

    pub async fn subscribe(&self, telegram_user_id: u64, username: String) {
        tracing::debug!("Subscribing {} to {}", telegram_user_id, username);

        self.subscriptions
            .write()
            .await
            .entry(username)
            .or_insert(HashSet::new())
            .insert(telegram_user_id);
    }

    pub async fn unsubscribe(&self, telegram_user_id: u64, username: String) {
        tracing::debug!("Unsubscribing {} from {}", telegram_user_id, username);

        self.subscriptions
            .write()
            .await
            .entry(username)
            .and_modify(|set| {
                set.remove(&telegram_user_id);
            });
    }
}
