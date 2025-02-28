use std::{net::SocketAddr, sync::Arc};

use axum::{
    Extension, Router,
    body::HttpBody,
    http::{self, StatusCode},
    response::IntoResponse,
    routing::post,
};
use eyre::{Context, ContextCompat};
use futures::TryStreamExt as _;
use http_body_util::BodyExt as _;
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::trace::TraceLayer;
use twitch_api::{
    HelixClient,
    client::ClientDefault,
    eventsub::{
        Event, EventType, Status,
        stream::{StreamOnlineV1, StreamOnlineV1Payload},
    },
};
use twitch_oauth2::AppAccessToken;

use crate::{config::CONFIG, subscription_manager::SubscriptionManager};

pub async fn eventsub_register(
    token: Arc<RwLock<AppAccessToken>>,
    login: String,
    webhook_url: String,
) -> Result<(), eyre::Report> {
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let client: HelixClient<'_, reqwest::Client> = HelixClient::new();

    let channel_information = client
        .get_channel_from_login(&login, &*token.read().await)
        .await
        .wrap_err("when getting channel")?
        .wrap_err("when getting channel")?;

    let broadcaster_id = channel_information.broadcaster_id;

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60));

    loop {
        interval.tick().await;

        let subs = client
            .get_eventsub_subscriptions(Status::Enabled, None, None, &*token.read().await)
            .map_ok(|events| {
                futures::stream::iter(events.subscriptions.into_iter().map(Ok::<_, eyre::Report>))
            })
            .try_flatten()
            .try_filter(|event| futures::future::ready(event.transport.is_webhook()))
            .try_collect::<Vec<_>>()
            .await?;

        let online_exists = subs.iter().any(|sub| {
            sub.transport.as_webhook().unwrap().callback == webhook_url
                && sub.type_ == EventType::StreamOnline
                && sub.version == "1"
                && sub
                    .condition
                    .as_object()
                    .expect("a stream.online did not contain broadcaster")
                    .get("broadcaster_user_id")
                    .unwrap()
                    .as_str()
                    == Some(broadcaster_id.as_str())
        });

        let transport = twitch_api::eventsub::Transport::webhook(
            webhook_url.clone(),
            CONFIG.twitch_signing_secret.clone(),
        );

        if !online_exists {
            client
                .create_eventsub_subscription(
                    StreamOnlineV1::broadcaster_user_id(broadcaster_id.clone()),
                    transport.clone(),
                    &*token.read().await,
                )
                .await
                .wrap_err_with(|| "when registering online event")?;
        }
    }
}

pub async fn twitch_eventsub(
    Extension(cache): Extension<Arc<retainer::Cache<http::HeaderValue, ()>>>,
    request: http::Request<axum::body::Body>,
) -> impl IntoResponse {
    const MAX_ALLOWED_RESPONSE_SIZE: u64 = 64 * 1024;

    let (parts, body) = request.into_parts();
    let response_content_length = match body.size_hint().upper() {
        Some(v) => v,
        None => MAX_ALLOWED_RESPONSE_SIZE + 1,
    };
    let body = if response_content_length < MAX_ALLOWED_RESPONSE_SIZE {
        body.collect().await.unwrap().to_bytes().to_vec()
    } else {
        panic!("too big data given")
    };

    let request = http::Request::from_parts(parts, &*body);

    if !Event::verify_payload(&request, CONFIG.twitch_signing_secret.as_bytes()) {
        return (StatusCode::BAD_REQUEST, "Invalid signature".to_string());
    }

    if let Some(id) = request.headers().get("Twitch-Eventsub-Message-Id") {
        if cache.get(id).await.is_none() {
            cache.insert(id.clone(), (), 400).await;
        } else {
            return (StatusCode::OK, "".to_string());
        }
    }

    let event = Event::parse_http(&request).unwrap();

    if let Some(ver) = event.get_verification_request() {
        return (StatusCode::OK, ver.challenge.clone());
    }

    if event.is_revocation() {
        return (StatusCode::OK, "".to_string());
    }
    use twitch_api::eventsub::{Message as M, Payload as P};

    match event {
        Event::StreamOnlineV1(P {
            message:
                M::Notification(StreamOnlineV1Payload {
                    broadcaster_user_id,
                    started_at,
                    ..
                }),
            ..
        }) => {
            todo!(
                "StreamOnlineV1: broadcaster_user_id: {}, started_at: {}",
                broadcaster_user_id,
                started_at
            );
        }
        _ => {}
    }
    (StatusCode::OK, String::default())
}

struct TwitchWebhookServer {
    subscription_manager: Arc<SubscriptionManager>,
    subscribed_to: Arc<RwLock<Vec<String>>>,
    app_access_token: Arc<RwLock<AppAccessToken>>,
}

impl TwitchWebhookServer {
    pub fn new(
        subscription_manager: Arc<SubscriptionManager>,
        app_access_token: Arc<RwLock<AppAccessToken>>,
    ) -> Self {
        Self {
            subscription_manager,
            subscribed_to: Arc::new(RwLock::new(Vec::new())),
            app_access_token,
        }
    }

    pub async fn start_webhook_server(&self) {
        let retainer = Arc::new(retainer::Cache::<axum::http::HeaderValue, ()>::new());

        let ret = retainer.clone();
        let _: tokio::task::JoinHandle<Result<(), ()>> = tokio::spawn(async move {
            ret.monitor(10, 0.50, tokio::time::Duration::from_secs(86400 / 2))
                .await;
            Ok(())
        });

        let app = Router::new()
            .route(
                "/twitch/eventsub/",
                post(move |cache, request| twitch_eventsub(cache, request)),
            )
            .layer(Extension(retainer))
            .layer(TraceLayer::new_for_http());

        let address = SocketAddr::new([0, 0, 0, 0].into(), CONFIG.twitch_webhook_port);

        let _ = axum::serve(
            TcpListener::bind(address).await.unwrap(),
            app.into_make_service(),
        )
        .await;
    }

    pub async fn subscribe(&self, streamer: String) -> bool {
        match eventsub_register(
            self.app_access_token.clone(),
            streamer.clone(),
            format!("{}/twitch/eventsub/", CONFIG.twitch_webhook_url),
        )
        .await
        {
            Ok(_) => true,
            Err(err) => {
                eprintln!("Error subscribing to {}: {}", streamer, err);
                false
            }
        }
    }

    pub async fn check_subscriptions(&self) {
        loop {
            let streamers = self
                .subscription_manager
                .subscriptions
                .read()
                .await
                .keys()
                .cloned()
                .collect::<Vec<String>>();

            for streamer in streamers {
                let is_subscribed = self.subscribed_to.read().await.contains(&streamer);

                if !is_subscribed {
                    if self.subscribe(streamer.clone()).await {
                        self.subscribed_to.write().await.push(streamer);
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn start(&self) -> Result<(), eyre::Report> {
        let subscribe_future = self.check_subscriptions();
        let webhook_future = self.start_webhook_server();

        futures::join!(subscribe_future, webhook_future);

        Ok(())
    }
}

pub async fn start_twitch_webhook(
    subscription_manager: Arc<SubscriptionManager>,
) -> Result<(), eyre::Report> {
    let client: HelixClient<_> = twitch_api::HelixClient::with_client(
        <reqwest::Client>::default_client_with_name(Some(
            "twitch-rs/eventsub"
                .parse()
                .wrap_err_with(|| "when creating header name")
                .unwrap(),
        ))
        .wrap_err_with(|| "when creating client")?,
    );

    let token = twitch_oauth2::AppAccessToken::get_app_access_token(
        &client,
        CONFIG.twitch_client_id.clone().into(),
        CONFIG.twitch_client_secret.clone().into(),
        vec![],
    )
    .await?;

    let token = Arc::new(tokio::sync::RwLock::new(token));

    let twitch_webhook_server = TwitchWebhookServer::new(subscription_manager, token);
    let _ = twitch_webhook_server.start().await;

    Ok(())
}
