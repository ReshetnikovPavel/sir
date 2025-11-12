use tokio::sync::mpsc::Sender;

use crate::{db::id::Id, voice_assistant};

pub enum Event {
    Error(anyhow::Error),
    Message(ChatId, MessageId),
    VoiceAssistantState(voice_assistant::state::StateKind),
}

type ChatId = Id;
type MessageId = Id;

pub struct EventEmitter {
    pub tx: Sender<Event>,
}

impl EventEmitter {
    pub async fn emit(&self, event: Event) {
        if let Err(e) = self.tx.send(event).await {
            log::error!("{:#?}", e)
        }
    }
}
