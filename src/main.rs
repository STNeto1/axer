use axum::{extract::State, http::StatusCode, response::IntoResponse, routing, Json, Router};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast::{self};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::logger::log_all_messages;

mod logger;
mod ws;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsMessage {
    channel: String,
    topic: String,
    value: serde_json::Value,
}

pub struct AppState {
    tx: broadcast::Sender<WsMessage>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chat=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv().expect("Could not load .env file");

    let (tx, rx) = broadcast::channel::<WsMessage>(100);

    let logger = logger::sql::SqlLogger::new().await;
    tokio::spawn(log_all_messages(rx, logger));

    let app_state = Arc::new(AppState { tx });
    let app = Router::new()
        .route("/send", routing::post(send_message_handler))
        .route(
            "/ws/dashboard",
            routing::get(ws::dashboard::dashboard_websocket_handler),
        )
        .route(
            "/ws/:channel/:topic",
            routing::get(ws::channel::websocket_handler),
        )
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn send_message_handler(
    State(state): State<Arc<AppState>>,
    Json(msg): Json<WsMessage>,
) -> impl IntoResponse {
    return match state.tx.send(msg.clone()) {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Could not send message: {}", e.to_string()),
        )
            .into_response(),
    };
}
