use std::sync::Arc;

use libsql::{Connection, params};

use crate::{
    db::id::Id,
    domain::messages::{AssistantMessage, Message, SystemMessage, ToolMessage, UserMessage},
};

async fn create_tables(conn: &Connection) -> Result<(), libsql::Error> {
    let create_chat_table = "
        CREATE TABLE IF NOT EXISTS chats (
            id INTEGER PRIMARY KEY
    )";
    conn.execute(create_chat_table, ()).await.unwrap();

    let create_message_table = "
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            chat_id INTEGER NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool', 'function')),
            content TEXT NOT NULL,
            tool_calls TEXT,
            tool_call_id TEXT,
            is_error INTEGER CHECK (is_error IN (0, 1)),
        FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
    )";
    conn.execute(create_message_table, ()).await.unwrap();

    Ok(())
}

pub struct ChatRepo {
    conn: Arc<Connection>,
}

impl ChatRepo {
    pub async fn init(conn: Arc<Connection>) -> Result<Self, libsql::Error> {
        create_tables(&conn).await?;
        Ok(Self { conn })
    }

    pub async fn new_chat(&self) -> Result<Id, libsql::Error> {
        let insert_chat = "INSERT INTO chats(id) VALUES(NULL) RETURNING id";
        let mut rows = self.conn.query(insert_chat, params![]).await.unwrap();

        rows.next().await.unwrap().unwrap().get::<Id>(0)
    }

    pub async fn add_message(&self, chat_id: Id, message: Message) -> Result<Id, libsql::Error> {
        let insert_message = "
            INSERT INTO messages(chat_id, role, content, tool_calls, tool_call_id, is_error) VALUES
            (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING id";

        let mut rows = self
            .conn
            .query(insert_message, into_add_message_params(chat_id, message))
            .await
            .unwrap();

        rows.next().await.unwrap().unwrap().get::<Id>(0)
    }

    pub async fn get_messages(&self, chat_id: Id) -> Result<Vec<Message>, libsql::Error> {
        let select_messages = "SELECT role, content, tool_calls, tool_call_id, is_error FROM messages WHERE chat_id = ?1";
        let params = params![chat_id];
        let mut rows = self.conn.query(select_messages, params).await.unwrap();

        let mut messages = vec![];
        while let Some(row) = rows.next().await.unwrap() {
            let role = row.get::<String>(0).unwrap();
            let content = row.get(1).unwrap();
            let message = match role.as_str() {
                "system" => Message::System(SystemMessage { content }),
                "user" => Message::User(UserMessage { content }),
                "assistant" => Message::Assistant(AssistantMessage {
                    content,
                    tool_calls: serde_json::from_str(&row.get::<String>(2).unwrap()).unwrap(),
                }),
                "tool" => Message::Tool(ToolMessage {
                    content: serde_json::from_str(&content).unwrap(),
                    tool_call_id: row.get::<String>(3).unwrap(),
                    is_error: row.get::<bool>(4).unwrap(),
                }),
                _ => unreachable!(),
            };
            messages.push(message);
        }
        Ok(messages)
    }
}

fn into_add_message_params(
    chat_id: Id,
    message: Message,
) -> [Result<libsql::Value, libsql::Error>; 6] {
    match message {
        Message::System(system_message) => {
            let content = system_message.content;
            params![
                chat_id,
                "system",
                content,
                None::<String>,
                None::<String>,
                None::<i32>
            ]
        }
        Message::User(user_message) => {
            let content = user_message.content;
            params![
                chat_id,
                "user",
                content,
                None::<String>,
                None::<String>,
                None::<i32>
            ]
        }
        Message::Assistant(assistant_message) => {
            let content = assistant_message.content;
            let tool_calls = serde_json::to_string(&assistant_message.tool_calls).unwrap();
            params![
                chat_id,
                "assistant",
                content,
                tool_calls,
                None::<String>,
                None::<i32>
            ]
        }
        Message::Tool(tool_message) => {
            let content = serde_json::to_string(&tool_message.content).unwrap();
            let tool_call_id = tool_message.tool_call_id;
            let is_error = tool_message.is_error;
            params![
                chat_id,
                "tool",
                content,
                None::<String>,
                tool_call_id,
                is_error
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr as _, sync::Arc};

    use rand::distr::{Alphanumeric, SampleString as _};
    use rmcp::model::{Annotated, RawTextContent};
    use tempfile::TempDir;

    use crate::domain::messages::ToolCallMessage;

    use super::*;

    async fn setup() -> (TempDir, Arc<ChatRepo>) {
        let tmp_dir = TempDir::new().unwrap();
        let name = Alphanumeric.sample_string(&mut rand::rng(), 16);
        let tmp_path = tmp_dir.path().join(name);

        let db = libsql::Builder::new_local(tmp_path).build().await.unwrap();
        let db_connection = db.connect().unwrap();
        let db_connection = Arc::new(db_connection);
        let chat_repo = ChatRepo::init(db_connection).await.unwrap();
        let chat_repo = Arc::new(chat_repo);

        (tmp_dir, chat_repo)
    }

    fn get_messages() -> Vec<Message> {
        vec![
            Message::System(SystemMessage {
                content: "System message".to_owned(),
            }),
            Message::User(UserMessage {
                content: "Do something pls".to_owned(),
            }),
            Message::Assistant(AssistantMessage {
                content: "Content".to_owned(),
                tool_calls: vec![ToolCallMessage {
                    id: "1".to_owned(),
                    name: "tool1".to_owned(),
                    arguments: serde_json::Map::from_str("{\"a\": 1,\"b\": {\"c\": \"abc\"}}")
                        .unwrap(),
                }],
            }),
            Message::Tool(ToolMessage {
                content: vec![Annotated::new(
                    rmcp::model::RawContent::Text(RawTextContent {
                        text: "result".to_owned(),
                    }),
                    None,
                )],
                tool_call_id: "1".to_owned(),
                is_error: false,
            }),
        ]
    }

    async fn add_messages(chat_repo: Arc<ChatRepo>, chat_id: Id, messages: &[Message]) {
        for message in messages {
            chat_repo
                .add_message(chat_id, message.clone())
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn should_get_chat_in_order() {
        let (_tmp_dir, chat_repo) = setup().await;

        let chat_id = chat_repo.new_chat().await.unwrap();
        let expected_messages = get_messages();
        add_messages(chat_repo.clone(), chat_id, &expected_messages).await;

        let actual_messages = chat_repo.get_messages(chat_id).await.unwrap();

        let expected = serde_json::to_string(&expected_messages).unwrap();
        let actual = serde_json::to_string(&actual_messages).unwrap();
        assert_eq!(expected, actual)
    }

    #[tokio::test]
    async fn should_get_chat_from_correct_chat() {
        let (_tmp_dir, chat_repo) = setup().await;

        let chat1_id = chat_repo.new_chat().await.unwrap();
        let messages1 = get_messages();

        let chat2_id = chat_repo.new_chat().await.unwrap();
        let mut messages2 = get_messages();
        messages2.reverse();

        tokio::join!(
            add_messages(chat_repo.clone(), chat1_id, &messages1),
            add_messages(chat_repo.clone(), chat2_id, &messages2)
        );

        let actual_messages1 = chat_repo.get_messages(chat1_id).await.unwrap();
        let actual_messages2 = chat_repo.get_messages(chat2_id).await.unwrap();

        let expected = serde_json::to_string(&messages1).unwrap();
        let actual = serde_json::to_string(&actual_messages1).unwrap();
        assert_eq!(expected, actual);

        let expected = serde_json::to_string(&messages2).unwrap();
        let actual = serde_json::to_string(&actual_messages2).unwrap();
        assert_eq!(expected, actual);
    }
}
