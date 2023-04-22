use axum::async_trait;

use crate::WsMessage;

pub mod console;
pub mod sql;

#[async_trait]
pub trait MessageLogger {
    async fn log(&self, message: &WsMessage) -> Result<(), String>;
}
