use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use bolt::{config::Config, relay::MemoryRelay};
use futures::stream::StreamExt;
use nostr::message::ClientMessage;
use serde_json::Value;
use std::time::Duration;
use tokio::signal;
use tower_http::timeout::TimeoutLayer;

#[tokio::main]
async fn main() {
    let config = Config::new("config.yml").unwrap();
    let memory_relay = MemoryRelay::new().unwrap();
    let app = Router::new()
        .route("/", get(websocket_handler))
        .layer(TimeoutLayer::new(Duration::from_secs(10)));

    let addr = format!("{}:{}", config.relay_bind_address, config.relay_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket))
}

async fn websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(msg) = message {
            let client_message_json: Value = serde_json::from_str(&msg).unwrap();
            let client_message = ClientMessage::from_value(client_message_json).unwrap();
            match client_message {
                ClientMessage::Event(event) => {
                    println!("received event: {:?}", event);
                }
                ClientMessage::Req {
                    subscription_id,
                    filter,
                } => {
                    println!(
                        "received req: subscription_id={:?}, filter={:?}",
                        subscription_id, filter
                    );
                }
                ClientMessage::Count {
                    subscription_id,
                    filter,
                } => {
                    println!(
                        "received count: subscription_id={:?}, filter={:?}",
                        subscription_id, filter
                    );
                }
                ClientMessage::Close(subscription_id) => {
                    println!("received close: subscription_id={:?}", subscription_id);
                }
                _ => {
                    println!("received unsupported message: {:?}", client_message);
                }
            }
        }
    }
}
