use std::{
    io::{self, Write},
    sync::Arc,
};

use crate::{audio::audio_service::AudioService, chat::pipeline::TextPipeline};

use super::displayer::CliChunkDisplayer;

pub struct CliRuntime {
    pub text_pipeline: Arc<TextPipeline>,
    pub audio_service: Arc<AudioService>,
}

impl CliRuntime {
    pub async fn run(&self) {
        let chunk_displayer = Arc::new(CliChunkDisplayer::new());

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

            let result = self.text_pipeline
                .process(&input, chunk_displayer.clone())
                .await;

            if let Err(err) = result {
                println!("Something went wrong while processing your request");
                log::error!("{}", err)
            }
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
