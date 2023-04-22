use axum::async_trait;
use serde::Serialize;

use crate::WsMessage;

use super::MessageLogger;

pub struct SqlLogger {
    conn: sqlx::Pool<sqlx::Postgres>,
}

impl SqlLogger {
    pub async fn new(conn: &sqlx::Pool<sqlx::Postgres>) -> Self {
        Self { conn: conn.clone() }
    }
}

#[async_trait]
impl MessageLogger for SqlLogger {
    async fn log(&self, message: &WsMessage) -> Result<(), String> {
        return match sqlx::query!(
            r#"
          INSERT INTO messages (channel, topic, value) VALUES ($1, $2, $3);
          "#,
            message.channel,
            message.topic,
            message.value.to_string()
        )
        .execute(&self.conn)
        .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Could not log message to SQL: {}", e.to_string())),
        };
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SqlRecord {
    pub id: i32,
    pub channel: String,
    pub topic: String,
    pub value: String,
    pub created_at: String,
}
