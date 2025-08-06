use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use uuid::Uuid;

use crate::{
    audio::audio_service::AudioService, chat::pipeline::TextPipeline, db::chat_repo::ChatRepo,
};

use super::displayer::CliChunkDisplayer;

pub struct CliRuntime {
    pub text_pipeline: TextPipeline,
    pub audio_service: AudioService,
    pub chat_repo: Rc<ChatRepo>,
    pub last_chat_id_path: PathBuf,
}

impl CliRuntime {
    pub async fn run(&self) {
        let chunk_displayer = Arc::new(CliChunkDisplayer::new());
        let chat_id = self.get_chat_id().await;

        loop {
            let input = match self.read_input() {
                Ok(input) => input,
                Err(err) => {
                    println!("Error reading input");
                    log::error!("{}", err);
                    continue;
                }
            };

            let input = input.trim();

            let input = match self.use_commands(input).await {
                Some(input) => input,
                None => continue,
            };

            let result = self
                .text_pipeline
                .process(chat_id, &input, chunk_displayer.clone())
                .await;

            if let Err(err) = result {
                println!("Something went wrong while processing your request");
                log::error!("{}", err)
            }
        }
    }

    async fn get_chat_id(&self) -> Uuid {
        match read_to_string(&self.last_chat_id_path) {
            Ok(id) => Uuid::parse_str(&id.trim()).unwrap(),
            Err(_) => {
                let id = self.chat_repo.new_chat().await.unwrap();
                fs::write(&self.last_chat_id_path, id.to_string()).unwrap();
                id
            },
        }
    }

    fn read_input(&self) -> io::Result<String> {
        print!(">>> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    }

    async fn use_commands(&self, input: &str) -> Option<String> {
        match input {
            "!voice" | "!v" => self.voice_command().await,
            _ => Some(input.to_owned()),
        }
    }

    async fn voice_command(&self) -> Option<String> {
        match self.audio_service.listen_input().await {
            Ok(text) => Some(text),
            Err(err) => {
                println!("Unable to listen");
                log::error!("{}", err);
                None
            }
        }
    }
}
