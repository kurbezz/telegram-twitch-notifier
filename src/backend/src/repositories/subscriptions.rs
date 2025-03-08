use futures::StreamExt as _;
use mongodb::{
    Client, Collection,
    bson::{Document, doc, oid::ObjectId},
};
use serde::Serialize;

use crate::config::CONFIG;

pub struct SubscriptionRepository {}

#[derive(Serialize)]
pub struct Subscription {
    pub id: ObjectId,
    pub streamer: String,
    pub telegram_user_id: u64,
}

impl From<Document> for Subscription {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.get_object_id("_id").unwrap(),
            streamer: doc.get_str("streamer").unwrap().to_string(),
            telegram_user_id: doc.get_i64("telegram_user_id").unwrap() as u64,
        }
    }
}

impl SubscriptionRepository {
    async fn get_collection() -> mongodb::error::Result<Collection<Document>> {
        let client = Client::with_uri_str(CONFIG.mongodb_connection_string.clone()).await?;

        let database = client.database("telegram-twitch-notifier");

        Ok(database.collection("subscriptions"))
    }

    pub async fn get_by_id(id: ObjectId) -> mongodb::error::Result<Option<Subscription>> {
        let collection = Self::get_collection().await?;

        let doc = collection.find_one(doc! { "_id": id }).await?;

        match doc {
            Some(doc) => Ok(Some(Subscription::from(doc))),
            None => Ok(None),
        }
    }

    pub async fn get_or_create(
        streamer: String,
        telegram_user_id: u64,
    ) -> mongodb::error::Result<Subscription> {
        let collection = Self::get_collection().await?;

        let existing = collection
            .find_one(doc! {
                "streamer": streamer.clone(),
                "telegram_user_id": telegram_user_id as i64,
            })
            .await?;

        if let Some(v) = existing {
            return Ok(Subscription::from(v));
        }

        let created = collection
            .insert_one(doc! {
                "streamer": streamer,
                "telegram_user_id": telegram_user_id as i64,
            })
            .await?;

        let inserted_id = created.inserted_id.as_object_id().unwrap();

        Ok(SubscriptionRepository::get_by_id(inserted_id.clone())
            .await?
            .unwrap())
    }

    pub async fn delete(streamer: String, telegram_user_id: u64) -> mongodb::error::Result<()> {
        let collection = Self::get_collection().await?;

        collection
            .delete_one(doc! {
                "streamer": streamer,
                "telegram_user_id": telegram_user_id as i64,
            })
            .await?;

        Ok(())
    }

    pub async fn all_by_user(telegram_user_id: u64) -> mongodb::error::Result<Vec<Subscription>> {
        let collection = Self::get_collection().await?;

        let mut subs = collection
            .find(doc! { "telegram_user_id": telegram_user_id as i64 })
            .await?;

        let mut result = Vec::new();

        while let Some(sub) = subs.next().await {
            result.push(Subscription::from(sub?));
        }

        Ok(result)
    }

    pub async fn all() -> mongodb::error::Result<Vec<Subscription>> {
        let collection = Self::get_collection().await?;

        let mut subs = collection.find(doc! {}).await?;

        let mut result = Vec::new();

        while let Some(sub) = subs.next().await {
            result.push(Subscription::from(sub?));
        }

        Ok(result)
    }
}
