use std::{io::{self, Write}, sync::Arc};

use crate::chat::pipeline::Pipeline;

use super::displayer::CliChunkDisplayer;

pub async fn cli_runtime(pipeline: &mut Pipeline) {
    let chunk_displayer = Arc::new(CliChunkDisplayer::new());

    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.eq_ignore_ascii_case("exit") {
                    return;
                }
                pipeline.call(trimmed, chunk_displayer.clone()).await.unwrap();
            }
            Err(error) => {
                eprintln!("❌ Error reading input: {}", error);
            }
        }
    }
}
