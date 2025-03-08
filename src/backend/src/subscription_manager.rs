use std::collections::{HashMap, HashSet};

use tokio::sync::RwLock;

use crate::repositories::subscriptions::SubscriptionRepository;

pub struct SubscriptionManager {
    pub subscriptions: RwLock<HashMap<String, HashSet<u64>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn load(&self) -> mongodb::error::Result<()> {
        let subs = SubscriptionRepository::all().await?;

        for sub in subs {
            self.subscriptions
                .write()
                .await
                .entry(sub.streamer.clone())
                .or_insert(HashSet::new())
                .insert(sub.telegram_user_id);
        }

        Ok(())
    }

    pub async fn subscribe(&self, telegram_user_id: u64, username: String) {
        tracing::debug!("Subscribing {} to {}", telegram_user_id, username);

        let inserted = self
            .subscriptions
            .write()
            .await
            .entry(username.clone())
            .or_insert(HashSet::new())
            .insert(telegram_user_id);

        if !inserted {
            return;
        }

        SubscriptionRepository::get_or_create(username, telegram_user_id)
            .await
            .expect("Failed to create subscription");
    }

    pub async fn unsubscribe(&self, telegram_user_id: u64, username: String) {
        tracing::debug!("Unsubscribing {} from {}", telegram_user_id, username);

        SubscriptionRepository::delete(username, telegram_user_id)
            .await
            .expect("Failed to delete subscription");
    }
}
