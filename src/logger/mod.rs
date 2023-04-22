use axum::async_trait;
use tokio::sync::broadcast::Receiver;

use crate::WsMessage;

pub mod console;
pub mod sql;
pub mod surreal;

#[async_trait]
pub trait MessageLogger {
    async fn log(&self, message: &WsMessage) -> Result<(), String>;
}

pub async fn log_all_messages(mut rx: Receiver<WsMessage>, logger: impl MessageLogger) {
    while let Ok(msg) = rx.recv().await {
        match logger.log(&msg).await {
            Ok(_) => {}
            Err(e) => tracing::error!("could not log message: {}", e),
        }
    }
}
