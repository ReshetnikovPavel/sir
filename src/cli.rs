use std::io::{self, Write};

use crate::LLM;

pub async fn cli(llm: &mut LLM) -> Result<(), ()>{
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
                let result = llm.ask(trimmed).await;
                if let Ok(message) = result {
                    println!("{}", message);
                }
            }
            Err(error) => {
                eprintln!("❌ Error reading input: {}", error);
            }
        }
    }
}
