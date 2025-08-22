use std::rc::Rc;

use libsql::{params, Connection};
use uuid::Uuid;

use crate::domain::messages::Message;

pub async fn create_tables(conn: &Connection) -> Result<(), libsql::Error> {
    let create_chat_table = "
        CREATE TABLE IF NOT EXISTS chats (
        id BLOB PRIMARY KEY NOT NULL
    );";
    conn.execute(create_chat_table, ()).await?;

    let create_message_table = "
        CREATE TABLE IF NOT EXISTS messages (
        id BLOB PRIMARY KEY NOT NULL,
        chat_id BLOB NOT NULL,
        role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool', 'function')),
        content TEXT NOT NULL,
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
    );";
    conn.execute(create_message_table, ()).await?;

    Ok(())
}

pub struct ChatRepo {
    conn: Rc<Connection>,
}

impl ChatRepo {
    pub async fn init(conn: Rc<Connection>) -> Result<Self, libsql::Error> {
        create_tables(&conn).await?;
        Ok(Self { conn })
    }

    pub async fn new_chat(&self) -> Result<Uuid, libsql::Error> {
        let id = Uuid::now_v7();

        let insert_chat = "INSERT INTO chats(id) VALUES(?1)";
        let params = params![id.into_bytes().as_slice()];
        self.conn.execute(insert_chat, params).await?;

        Ok(id)
    }

    pub async fn add_message(
        &self,
        chat_id: Uuid,
        message: &Message,
    ) -> Result<Uuid, libsql::Error> {
        let id = Uuid::now_v7();
        let message_json = serde_json::to_value(message).unwrap();
        let role = message_json.get("role").unwrap().as_str().unwrap();

        let insert_message =
            "INSERT INTO messages(id, chat_id, role, content) VALUES (?1, ?2, ?3, ?4)";
        let params = params![
            id.into_bytes().as_slice(),
            chat_id.into_bytes().as_slice(),
            role,
            message_json.to_string()
        ];
        self.conn.execute(insert_message, params).await?;

        Ok(id)
    }

    pub async fn get_messages(&self, chat_id: Uuid) -> Result<Vec<Message>, libsql::Error> {
        let select_messages = "SELECT content FROM messages WHERE chat_id = ?1";
        let params = params![chat_id.into_bytes().as_slice()];
        let mut rows = self.conn.query(select_messages, params).await?;

        let mut messages = vec![];
        while let Some(row) = rows.next().await? {
            messages.push(serde_json::from_str(row.get_str(0).unwrap()).unwrap());
        }
        Ok(messages)
    }
}
