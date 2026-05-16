use crate::text::pipeline::TextPipeline;

pub struct Tui {
    pub pipeline: TextPipeline,
}

impl Tui {
    pub async fn run(&self) {
        print!(">>> ");
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Incorrect string");

        let response = self.pipeline.generate_new_assistant_message(0, true).await;
    }
}
