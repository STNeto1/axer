use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing, Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WsMessage {
    channel: String,
    topic: String,
    value: serde_json::Value,
}

struct AppState {
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

    let (tx, _rx) = broadcast::channel::<WsMessage>(100);

    // let thread_rx = tx.clone();
    // tokio::spawn(async move {
    //     loop {
    //         // If there are no subscribers, we don't need to send anything.
    //         if thread_rx.receiver_count() == 0 {
    //             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //             continue;
    //         }

    //         if let Err(e) = thread_rx.send(WsMessage {
    //             channel: "channel".to_string(),
    //             topic: "topic".to_string(),
    //             value: serde_json::Value::Bool(true),
    //         }) {
    //             tracing::debug!("could not send message: {}", e);
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //     }
    // });

    let app_state = Arc::new(AppState { tx });
    let app = Router::new()
        .route("/send", routing::post(send_message_handler))
        .route("/ws/:channel/:topic", routing::get(websocket_handler))
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

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path((channel, topic)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(channel, topic, socket, state))
}

async fn websocket(channel: String, topic: String, stream: WebSocket, state: Arc<AppState>) {
    // By splitting, we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    if let Err(_) = sender.send(Message::Ping("Ping".into())).await {
        tracing::debug!("could not ping");
        return;
    }

    // We subscribe *before* sending the "joined" message, so that we will also
    // display it to our client.
    let mut rx = state.tx.subscribe();

    // Spawn the first task that will receive broadcast messages and send text
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg.channel == channel && msg.topic == topic {
                if sender
                    .send(Message::Text(serde_json::to_string(&msg.value).unwrap()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // End connection if client sends a close frame
            if msg == Message::Close(None) {
                break;
            }
        }
    });

    //wait for either task to finish and kill the other task
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    tracing::debug!("websocket connection closed");
}
