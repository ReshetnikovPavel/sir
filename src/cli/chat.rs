// use std::{io, sync::Arc};
//
// use anyhow;
// use async_openai::{
//     config::OpenAIConfig,
//     types::{
//         ChatChoiceStream, ChatCompletionResponseStream, ChatCompletionTool,
//         ChatCompletionToolChoiceOption, CreateChatCompletionRequest,
//     },
//     Client,
// };
// use log::error;
// use tokio_stream::StreamExt;
//
// use crate::{
//     chat::messages,
//     history::history_repo::HistoryRepo,
//     tools::{tool_stream_collector::ToolStreamCollector, tools_repo::ToolsRepo},
// };
//
// pub struct CliChat {
//     openai_client: Client<OpenAIConfig>,
//     model: String,
//     system_prompt: String,
//     history_repo: Arc<dyn HistoryRepo>,
//     tools_repo: Arc<ToolsRepo>,
// }
//
// impl CliChat {
//     pub async fn run(&self) -> anyhow::Result<()> {
//         self.history_repo
//             .set_system_message(messages::system(&self.system_prompt))
//             .await?;
//
//         loop {
//             let user_prompt = read_user_prompt()?;
//             self.history_repo.add(&messages::user(&user_prompt)).await?;
//
//             let tools = self.get_tools().await;
//             loop {
//                 let history = self.history_repo.history().await?;
//                 let request = CreateChatCompletionRequest {
//                     model: self.model.clone(),
//                     messages: history,
//                     stream: Some(true),
//                     tools: Some(tools.clone()),
//                     tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
//                     ..Default::default()
//                 };
//
//                 let mut stream = self
//                     .openai_client
//                     .chat()
//                     .create_stream(request)
//                     .await
//                     .unwrap();
//             }
//         }
//     }
//
//     async fn get_tools(&self) -> Vec<ChatCompletionTool> {
//         self.tools_repo
//             .tools()
//             .await
//             .unwrap_or_else(|(tools, errs)| {
//                 for err in errs {
//                     error!("{}", err)
//                 }
//                 tools
//             })
//             .into_iter()
//             .map(|t| t.into())
//             .collect()
//     }
//
//     async fn process_stream(&self, stream: ChatCompletionResponseStream) -> ProcessedStreamStatus {
//         let mut tool_call_messages = vec![];
//         let mut tool_call_result_messages = vec![];
//
//         let mut tool_stream_collector = ToolStreamCollector::new();
//
//         let mut content = String::new();
//
//         while let Some(Ok(response)) = stream.next().await {
//             let choice = &response.choices[0];
//             // println!("{:?}", choice);
//             if choice.delta.tool_calls.is_none() {
//                 if let Some(chunk) = &choice.delta.content {
//                     content.push_str(&chunk);
//                     print_chunk(choice);
//                 }
//             }
//
//             let call_message = tool_stream_collector.add_data(choice);
//             if let Some(call_message) = call_message {
//                 tool_call_messages.push(call_message.clone());
//                 let call = call_message.clone().try_into().unwrap();
//                 let call_tool_result = self.tools_repo.call_tool(call).await.unwrap();
//                 tool_call_result_messages.push(messages::call_tool_result(
//                     &call_message.id,
//                     &call_tool_result,
//                 ));
//             }
//         }
//         todo!()
//     }
// }
//
// enum ProcessedStreamStatus {
//     NeedsNextAssistantCall,
//     UserInput,
// }
//
// fn read_user_prompt() -> io::Result<String> {
//     let mut input = String::new();
//     io::stdin().read_line(&mut input)?;
//     Ok(input)
// }
//
// fn print_chunk(chunk: &ChatChoiceStream) {
//     if let Some(content) = &chunk.delta.content {
//         print!("{}", content);
//     }
//     if let Some(_) = &chunk.finish_reason {
//         println!();
//     }
// }
