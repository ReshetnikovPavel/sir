use async_trait::async_trait;

use crate::text::events::{Event, EventProcessor};

pub struct CliEventProcessor {}

#[async_trait]
impl EventProcessor for CliEventProcessor {
    async fn process(&self, event: Event) {
        match event {
            Event::Error(error) => log::error!("{}", error),
            Event::ResponseTextChunk(chunk) => print!("{}", chunk),
            Event::ToolCall(tool_call) => println!("A tool was called::: {:?}", tool_call),
            Event::ToolCallResult(tool_message) => {
                println!("Got a tool result::: {:?}", tool_message)
            }
            Event::AssistantResponded => println!(),
        }
    }
}
