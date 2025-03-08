use std::{error::Error, sync::Arc};

use teloxide::{
    Bot as OriginBot,
    adaptors::{CacheMe, Throttle, throttle::Limits},
    dispatching::{HandlerExt, UpdateFilterExt as _, dialogue::GetChatId},
    dptree::{self, Handler},
    macros::BotCommands,
    prelude::{Dispatcher, LoggingErrorHandler, Requester, RequesterExt},
    types::{BotCommand, Message, Update},
    update_listeners::webhooks,
};

use crate::{config::CONFIG, subscription_manager::SubscriptionManager};

pub type Bot = CacheMe<Throttle<OriginBot>>;

pub type BotHandlerInternal = Result<(), Box<dyn Error + Send + Sync>>;
type BotHandler = Handler<
    'static,
    dptree::di::DependencyMap,
    BotHandlerInternal,
    teloxide::dispatching::DpHandlerDescription,
>;

/// These commands are supported:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Help,
    Subscribe(String),
    Unsubscribe(String),
}

pub async fn help_message_handler(bot: Bot, message: Message) -> BotHandlerInternal {
    const HELP_MESSAGE: &str = r#"
Welcome!

This bot allow you to subscribe to receive start stream notifications.
    "#;

    match bot
        .send_message(message.chat_id().unwrap(), HELP_MESSAGE)
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(Box::new(err)),
    }
}

pub async fn subscribe_handler(
    bot: Bot,
    message: Message,
    subscription_manager: Arc<SubscriptionManager>,
    username: String,
) -> BotHandlerInternal {
    let user_id = message.clone().from.unwrap().id;

    subscription_manager.subscribe(user_id.0, username).await;

    match bot
        .send_message(message.chat_id().unwrap(), "Subscribed!")
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(Box::new(err)),
    }
}

pub async fn unsubscribe_handler(
    bot: Bot,
    message: Message,
    subscription_manager: Arc<SubscriptionManager>,
    username: String,
) -> BotHandlerInternal {
    let user_id = message.clone().from.unwrap().id;

    subscription_manager.unsubscribe(user_id.0, username).await;

    match bot
        .send_message(message.chat_id().unwrap(), "Unsubscribed!")
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(Box::new(err)),
    }
}

pub async fn get_handler() -> BotHandler {
    dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(|bot, message, command, subscription_manager| async move {
                match command {
                    Command::Start | Command::Help => help_message_handler(bot, message).await,
                    Command::Subscribe(username) => {
                        subscribe_handler(bot, message, subscription_manager, username).await
                    }
                    Command::Unsubscribe(username) => {
                        unsubscribe_handler(bot, message, subscription_manager, username).await
                    }
                }
            }),
    )
}

pub async fn get_commands() -> Vec<BotCommand> {
    vec![
        BotCommand {
            command: "start".into(),
            description: "Start the bot".into(),
        },
        BotCommand {
            command: "help".into(),
            description: "Show help".into(),
        },
        BotCommand {
            command: "subscribe".into(),
            description: "Subscribe to the newsletter".into(),
        },
        BotCommand {
            command: "unsubscribe".into(),
            description: "Unsubscribe from the newsletter".into(),
        },
    ]
}

pub fn get_telegram_bot() -> Bot {
    OriginBot::new(CONFIG.telegram_bot_token.clone())
        .throttle(Limits::default())
        .cache_me()
}

pub async fn start_telegram_bot(subscription_manager: Arc<SubscriptionManager>) {
    let bot = get_telegram_bot();

    let handler = get_handler().await;
    let commands = get_commands().await;

    let _ = bot.delete_webhook().await;
    let _ = bot.set_my_commands(commands).await;

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![subscription_manager])
        .build();

    let addr = ([0, 0, 0, 0], CONFIG.telegram_webhook_port).into();
    let url = CONFIG.telegram_webhook_url.parse().unwrap();
    let update_listener = webhooks::axum(
        bot,
        webhooks::Options::new(addr, url).path("/telegram/".to_string()),
    )
    .await
    .expect("Couldn't setup webhook");

    dispatcher
        .dispatch_with_listener(
            update_listener,
            LoggingErrorHandler::with_custom_text("An error from the update listener"),
        )
        .await;
}
