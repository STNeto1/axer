use axum::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::remote::http::{Client, Http},
    opt::auth::Root,
    sql::Thing,
    Surreal,
};

use super::MessageLogger;
use crate::WsMessage;

pub struct SurrealLogger {
    conn: Surreal<Client>,
}

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurrealMessage {
    channel: String,
    topic: String,
    value: String,
}

impl SurrealLogger {
    pub async fn new() -> Self {
        let conn = Surreal::new::<Http>("localhost:8000")
            .await
            .expect("Could not connect to Surreal");

        conn.signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .expect("Could not sign in to Surreal");

        conn.use_ns("axer")
            .use_db("axer")
            .await
            .expect("Could not use database");

        Self { conn }
    }
}

#[async_trait]
impl MessageLogger for SurrealLogger {
    async fn log(&self, message: &WsMessage) -> Result<(), String> {
        let r: Record = self
            .conn
            .create("messages")
            .content(SurrealMessage {
                channel: message.channel.clone(),
                topic: message.topic.clone(),
                value: message.value.to_string(),
            })
            .await
            .expect("Could not insert message");
        dbg!(r);

        return Ok(());
    }
}
