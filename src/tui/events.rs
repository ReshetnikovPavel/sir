use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{events::Event, messages::Message},
    voice_assistant,
};

pub struct EventProcessor {
    pub rx: Receiver<Event>,
    pub chat_repo: Arc<ChatRepo>,
}

impl EventProcessor {
    pub async fn run(&mut self) {
        while let Some(event) = self.rx.recv().await {
            if let Err(err) = self.process(event).await {
                log::error!("{}", err);
            }
        }
        panic!("Something went wrong with the event channel")
    }

    async fn process(&self, event: Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Error(error) => Err(error),
            Event::Message(chat_id, message_id) => self.message(chat_id, message_id).await,
            Event::VoiceAssistantState(state) => self.voice_assistant_state(state).await,
        }
    }

    async fn message(&self, chat_id: Id, message_id: Id) -> Result<(), anyhow::Error> {
        let message = self.chat_repo.get_message(chat_id, message_id).await?;
        match message {
            Message::System(system_message) => println!("System::: {}", system_message.content),
            Message::User(user_message) => println!("User::: {}", user_message.content),
            Message::Assistant(assistant_message) => {
                println!("Assistant::: {}", assistant_message.content)
            }
            Message::Tool(tool_message) => println!("Called tool::: {}", tool_message.tool_call_id),
        }
        Ok(())
    }

    async fn voice_assistant_state(
        &self,
        state: voice_assistant::state::StateKind,
    ) -> Result<(), anyhow::Error> {
        println!("{:#?}", state);
        Ok(())
    }
}
