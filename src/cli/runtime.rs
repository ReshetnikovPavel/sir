use std::{
    io::{self, Read, Write},
    sync::Arc,
};

use crate::{
    audio::{openai_stt::OpenAISpeechToText, recording::Recording},
    chat::pipeline::Pipeline,
};

use super::displayer::CliChunkDisplayer;

pub async fn cli_runtime(pipeline: &mut Pipeline, stt: Arc<OpenAISpeechToText>) {
    let chunk_displayer = Arc::new(CliChunkDisplayer::new());

    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                let s = match trimmed {
                    "!voice" => &listen_voice(stt.clone()).await.unwrap(),
                    _ => trimmed,
                };
                if s.eq_ignore_ascii_case("exit") {
                    return;
                }
                pipeline
                    .call(s, chunk_displayer.clone())
                    .await
                    .unwrap();
            }
            Err(error) => {
                eprintln!("❌ Error reading input: {}", error);
            }
        }
    }
}

async fn listen_voice(client: Arc<OpenAISpeechToText>) -> anyhow::Result<String> {
    println!("Recording voice...");
    let recording = Recording::start()?;

    loop {
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input)?;
        if input.trim() == "!stop" {
            break;
        }
    }
    let mut file = recording.stop()?;
    println!("Stopped recording");

    let mut buf = vec![];
    let _ = file.read_to_end(&mut buf)?;

    println!("Starting transcribing...");
    let transcription = client.transcribe(buf).await?;
    println!("Trascribed::: {}", transcription);

    return Ok(transcription);
}
