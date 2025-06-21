use std::io::{self, Write};
use tokio_stream::StreamExt;

use async_openai::types::ChatCompletionResponseStream;

use crate::LLM;

pub async fn cli(llm: &mut LLM) -> Result<(), ()> {
    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.eq_ignore_ascii_case("exit") {
                    return Ok(());
                }
                if let Ok(mut stream) = llm.get_response_stream(trimmed).await {
                    print_stream(&mut stream).await;
                }
            }
            Err(error) => {
                eprintln!("❌ Error reading input: {}", error);
            }
        }
    }
}

async fn print_stream(stream: &mut ChatCompletionResponseStream) {
    while let Some(Ok(response)) = stream.next().await {
        if let Some(text) = &response.choices[0].delta.content {
            print!("{}", text);
        }
    }
    println!();
}
