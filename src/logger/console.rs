use axum::async_trait;

use crate::WsMessage;

use super::MessageLogger;

#[derive(Debug, Default)]
pub struct ConsoleLogger;

#[async_trait]
impl MessageLogger for ConsoleLogger {
    async fn log(&self, message: &WsMessage) -> Result<(), String> {
        tracing::debug!("Received message: {:?}", message);

        Ok(())
    }
}
