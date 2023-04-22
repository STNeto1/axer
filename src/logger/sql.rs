use axum::async_trait;
use sqlx::{postgres::PgPoolOptions, Postgres};

use crate::WsMessage;

use super::MessageLogger;

pub struct SqlLogger {
    conn: sqlx::Pool<Postgres>,
}

impl SqlLogger {
    pub async fn new() -> Self {
        Self {
            conn: PgPoolOptions::new()
                .max_connections(5)
                .connect(std::env::var("DATABASE_URL").unwrap().as_str())
                .await
                .expect("Could not connect to SQL"),
        }
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
