use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestToolMessageContentPart,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
};
use rmcp::model::CallToolResult;

pub fn system(prompt: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
        content: ChatCompletionRequestSystemMessageContent::Text(prompt.to_owned()),
        name: None,
    })
}

pub fn user(prompt: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
        name: None,
    })
}

pub fn assistant(
    content: String,
    tool_calls: Vec<ChatCompletionMessageToolCall>,
) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(content)),
        refusal: None,
        name: None,
        audio: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        ..Default::default()
    })
}

pub fn call_tool_result(id: &str, result: &CallToolResult) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
        content: ChatCompletionRequestToolMessageContent::Array(
            result
                .content
                .iter()
                .map(|c| {
                    ChatCompletionRequestToolMessageContentPart::Text(
                        ChatCompletionRequestMessageContentPartText {
                            text: c.as_text().map(|c| c.text.clone()).unwrap_or("".to_owned()),
                        },
                    )
                })
                .collect(),
        ),
        tool_call_id: id.to_owned(),
    })
}
