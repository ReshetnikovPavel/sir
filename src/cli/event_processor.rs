use std::collections::HashMap;

use simple_stopwatch::Stopwatch;
use tokio::sync::mpsc::Receiver;

use crate::{
    entities::{
        messages::{ToolCallMessage, ToolMessage},
        tools::Tool,
    },
    text::events::Event,
};

pub struct CliEventProcessor {
    pub rx: Receiver<Event>,
    pub stopwatches: HashMap<String, Stopwatch>,
}

impl CliEventProcessor {
    pub async fn run(&mut self) {
        while let Some(event) = self.rx.recv().await {
            self.process(event).await;
        }
        panic!("Something went wrong with the event channel")
    }

    async fn process(&mut self, event: Event) {
        match event {
            Event::Error(error) => log::error!("{}", error),
            Event::ResponseTextChunk(chunk) => self.chunk(chunk),
            Event::ToolCall(tool_call_message) => self.tool_call(tool_call_message),
            Event::ToolCallResult(tool_message) => self.tool_call_result(tool_message),
            Event::AssistantResponded => self.assistant_responeded(),
            Event::StartLoadingTools => self.start_loading_tools(),
            Event::FinishLoadingTools => self.finish_loading_tools(),
            Event::FilteredTools(tools) => self.filtered_tools(tools),
            Event::RequestedAssistant => self.requested_assistant(),
        }
    }

    fn chunk(&mut self, chunk: String) {
        print!("{}", chunk);
    }

    fn requested_assistant(&mut self) {
        let stopwatch_key = "assistant_response";
        self.stopwatches
            .insert(stopwatch_key.to_string(), Stopwatch::start_new());
    }

    fn assistant_responeded(&mut self) {
        let stopwatch_key = "assistant_response";
        println!();
        self.stop_stopwatch(stopwatch_key);
    }

    fn tool_call(&mut self, tool_call_message: ToolCallMessage) {
        let stopwatch_key = format!("tool_call_{}", tool_call_message.id);
        self.stopwatches
            .insert(stopwatch_key, Stopwatch::start_new());
        println!("Calling tool `{}`", tool_call_message.name);
    }

    fn tool_call_result(&mut self, tool_message: ToolMessage) {
        let stopwatch_key = format!("tool_call_{}", tool_message.tool_call_id);
        self.stop_stopwatch(&stopwatch_key);
    }

    fn start_loading_tools(&mut self) {
        let stopwatch_key = "loading_tools";
        self.stopwatches
            .insert(stopwatch_key.to_string(), Stopwatch::start_new());
        println!("Loading tools");
    }

    fn finish_loading_tools(&mut self) {
        let stopwatch_key = "loading_tools";
        self.stop_stopwatch(stopwatch_key);
    }

    fn stop_stopwatch(&mut self, key: &str) {
        match self.stopwatches.remove(key) {
            Some(stopwatch) => {
                let seconds = stopwatch.s();
                println!("Finished in {}", seconds);
            }
            None => {
                log::error!("Stopwatch `{}` wasn't started", key);
            }
        }
    }

    fn filtered_tools(&mut self, tools: Vec<Tool>) {
        let tools = tools
            .into_iter()
            .map(|t| format!("`{}`", t.name))
            .collect::<Vec<_>>()
            .join(", ");
        println!("Filtered tools: {}", tools)
    }
}
