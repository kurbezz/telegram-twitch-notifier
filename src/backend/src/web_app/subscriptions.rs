use axum::{
    Extension, Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};

use crate::repositories::subscriptions::SubscriptionRepository;

use super::auth::{AuthLayer, UserId};

async fn get_subscriptions(Extension(UserId(user_id)): Extension<UserId>) -> impl IntoResponse {
    let subs = SubscriptionRepository::all_by_user(user_id).await.unwrap();

    Json(subs).into_response()
}

async fn create_subscription(
    Path(streamer): Path<String>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> impl IntoResponse {
    let sub = SubscriptionRepository::get_or_create(streamer, user_id)
        .await
        .unwrap();

    Json(sub).into_response()
}

async fn delete_subscription(
    Path(streamer): Path<String>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> impl IntoResponse {
    SubscriptionRepository::delete(streamer, user_id)
        .await
        .unwrap();

    StatusCode::NO_CONTENT
}

pub fn get_api_router() -> Router {
    Router::new()
        .route("/subscriptions/", get(get_subscriptions))
        .route("/subscriptions/:streamer/", post(create_subscription))
        .route("/subscriptions/:streamer/", delete(delete_subscription))
        .layer(AuthLayer)
}
