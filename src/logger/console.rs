use crate::WsMessage;

use super::MessageLogger;

#[derive(Debug, Default)]
pub struct ConsoleLogger;

impl MessageLogger for ConsoleLogger {
    fn log(&self, message: &WsMessage) {
        tracing::debug!("Received message: {:?}", message);
    }
}
