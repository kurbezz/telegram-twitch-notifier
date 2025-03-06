use std::collections::{HashMap, HashSet};

use futures::StreamExt;
use mongodb::{
    Client, Collection,
    bson::{Document, doc},
};
use tokio::sync::RwLock;

use crate::config::CONFIG;

pub struct SubscriptionManager {
    pub subscriptions: RwLock<HashMap<String, HashSet<u64>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    async fn get_collection() -> mongodb::error::Result<Collection<Document>> {
        let client = Client::with_uri_str(CONFIG.mongodb_connection_string.clone()).await?;

        let database = client.database("telegram-twitch-notifier");

        Ok(database.collection("subscriptions"))
    }

    pub async fn load(&self) -> mongodb::error::Result<()> {
        let collection = Self::get_collection().await?;

        let mut subs = collection.find(doc! {}).await?;

        while let Some(sub) = subs.next().await {
            let sub = sub?;

            let username = sub.get_str("streamer").unwrap();
            let telegram_user_id = sub.get_i64("telegram_user_id").unwrap() as u64;

            self.subscribe(telegram_user_id, username.to_string()).await;
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

        Self::get_collection()
            .await
            .unwrap()
            .insert_one(doc! {
                "streamer": username,
                "telegram_user_id": telegram_user_id as i64,
            })
            .await
            .unwrap();
    }

    pub async fn unsubscribe(&self, telegram_user_id: u64, username: String) {
        tracing::debug!("Unsubscribing {} from {}", telegram_user_id, username);

        self.subscriptions
            .write()
            .await
            .entry(username.clone())
            .and_modify(|set| {
                set.remove(&telegram_user_id);
            });

        Self::get_collection()
            .await
            .unwrap()
            .delete_one(doc! {
                "streamer": username,
                "telegram_user_id": telegram_user_id as i64,
            })
            .await
            .unwrap();
    }
}
