pub mod dashboard {
    use axum::{
        extract::{
            ws::{Message, WebSocket},
            State, WebSocketUpgrade,
        },
        response::IntoResponse,
    };
    use futures::{SinkExt, StreamExt};
    use std::sync::Arc;

    use crate::{logger::sql::SqlRecord, AppState};

    pub async fn dashboard_websocket_handler(
        ws: WebSocketUpgrade,
        State(state): State<Arc<AppState>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| dashboard_websocket(socket, state))
    }

    async fn dashboard_websocket(stream: WebSocket, state: Arc<AppState>) {
        // By splitting, we can send and receive at the same time.
        let (mut sender, mut receiver) = stream.split();

        if let Err(_) = sender.send(Message::Ping("Ping".into())).await {
            tracing::debug!("could not ping");
            return;
        }

        let result = sqlx::query!(
            r#"
            SELECT * FROM messages ORDER BY id DESC;
            "#
        )
        .fetch_all(&state.pool)
        .await
        .unwrap_or_else(|_| vec![])
        .into_iter()
        .map(|r| SqlRecord {
            id: r.id,
            channel: r.channel.unwrap_or_default(),
            topic: r.topic.unwrap_or_default(),
            value: r.value.unwrap_or_default(),
            created_at: r.created_at.unwrap().to_string(),
        })
        .collect::<Vec<SqlRecord>>();

        if let Err(_) = sender
            .send(Message::Text(serde_json::to_string(&result).unwrap()))
            .await
        {
            tracing::debug!("could not send messages");
            return;
        }

        // We subscribe *before* sending the "joined" message, so that we will also
        // display it to our client.
        let mut rx = state.tx.subscribe();

        // Spawn the first task that will receive broadcast messages and send text
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if sender
                    .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
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

        tracing::debug!("dashboard websocket connection closed");
    }
}

pub mod channel {
    use std::sync::Arc;

    use axum::{
        extract::{
            ws::{Message, WebSocket},
            Path, State, WebSocketUpgrade,
        },
        response::IntoResponse,
    };
    use futures::{SinkExt, StreamExt};

    use crate::AppState;

    pub async fn websocket_handler(
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
}
