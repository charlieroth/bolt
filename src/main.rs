use axum::{
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use bolt::app::AppState;
use bolt::config;
use bolt::handlers::{index_handler::index_handler, websocket_handler::websocket_handler};
use nostr_ndb::NdbDatabase;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let config = config::Config::new("config.yml").unwrap();
    let app_state = Arc::new(AppState {
        config: config.clone(),
        db: NdbDatabase::open("relay.db").unwrap(),
    });

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any)
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::USER_AGENT,
            header::HOST,
            header::REFERER,
            header::ORIGIN,
            header::ACCESS_CONTROL_REQUEST_METHOD,
            header::ACCESS_CONTROL_REQUEST_HEADERS,
        ]);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(websocket_handler))
        .fallback(handler_404)
        .layer(cors_layer)
        .with_state(app_state);

    let addr = format!("{}:{}", config.relay_bind_address, config.relay_port);
    let listener = tokio::net::TcpListener::bind(addr.clone()).await.unwrap();
    println!("⚡️ Bolt Relay running on {}", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
    tokio::signal::ctrl_c().await.unwrap();
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "No route found")
}
