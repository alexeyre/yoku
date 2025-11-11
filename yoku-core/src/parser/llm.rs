use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// use diesel_async::RunQueryDsl;
use anyhow::Result;
use anyhow::anyhow;
use openai::{Credentials, chat::*};
use tokio::sync::OnceCell;

/// A small BoxFuture alias so trait methods can return boxed futures and be object-safe.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

use crate::parser::{ParsedSet, PromptBuilder};
use ollama_rs::generation::parameters::TimeUnit;

/// Object-safe LLM interface. Implementations return boxed futures so that
/// `Box<dyn LlmInterface>` is usable.
pub trait LlmInterface: Send + Sync {
    /// Parse an input set string. Returns a boxed future tied to the borrow lifetime.
    fn parse_set_string<'a>(
        &'a self,
        input: &'a str,
        known_exercises: &'a [String],
    ) -> BoxFuture<'a, Result<ParsedSet>>;
}

use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::parameters::KeepAlive;
use ollama_rs::{Ollama as OllamaSdk, models::ModelOptions};

fn strip_code_fences(s: &str) -> &str {
    let mut trimmed = s.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        trimmed = stripped;
    } else if let Some(stripped) = trimmed.strip_prefix("```") {
        trimmed = stripped;
    }
    if let Some(stripped) = trimmed.strip_suffix("```") {
        trimmed = stripped;
    }
    trimmed.trim()
}

pub struct Ollama {
    client: Arc<OllamaSdk>,
    model: String,
}

const OLLAMA_DEFAULT_MODEL: &str = "llama3.2:3b";
const OLLAMA_CLIENT: OnceCell<Arc<OllamaSdk>> = OnceCell::const_new();

impl Ollama {
    async fn get_client() -> Result<Arc<OllamaSdk>> {
        Ok(OLLAMA_CLIENT
            .get_or_init(|| async { Arc::new(OllamaSdk::default()) })
            .await
            .clone())
    }

    /// Public constructor returning a boxed trait object.
    pub async fn new(model: Option<String>) -> Result<Box<dyn LlmInterface + Send + Sync>> {
        Ok(Box::new(Self {
            client: Self::get_client().await?,
            model: model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string()),
        }))
    }
}
impl LlmInterface for Ollama {
    fn parse_set_string<'a>(
        &'a self,
        input: &'a str,
        known_exercises: &'a [String],
    ) -> BoxFuture<'a, Result<ParsedSet>> {
        Box::pin(async move {
            let prompt_builder = PromptBuilder::new(known_exercises);
            let system_prompt = prompt_builder.system_prompt();
            let user_prompt = prompt_builder.user_prompt(input);

            let options = ModelOptions::default().temperature(0.001);

            let res = self
                .client
                .generate(
                    GenerationRequest::new(self.model.clone(), user_prompt)
                        .options(options)
                        .system(system_prompt)
                        .keep_alive(KeepAlive::Until {
                            time: 30,
                            unit: TimeUnit::Minutes,
                        }),
                )
                .await;
            let res = res?;
            let response = strip_code_fences(res.response.trim());
            match serde_json::from_str(response) {
                Ok(parsed) => Ok(ParsedSet::with_original(parsed, input.into())),
                Err(e) => Err(anyhow!(
                    "Cannot parse LLM output: {}\nGot error: {}",
                    response,
                    e
                )),
            }
        })
    }
}

static OPENAI_CREDS: OnceCell<Credentials> = OnceCell::const_new();
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

pub struct OpenAi {
    model: String,
}
impl OpenAi {
    async fn get_creds() -> Result<Credentials> {
        Ok(OPENAI_CREDS
            .get_or_init(|| async { Credentials::from_env() })
            .await
            .clone())
    }

    /// Public constructor returning a boxed trait object.
    pub async fn new(model: Option<String>) -> Result<Box<dyn LlmInterface + Send + Sync>> {
        Ok(Box::new(Self {
            model: model.unwrap_or_else(|| OPENAI_DEFAULT_MODEL.to_string()),
        }))
    }
}
impl LlmInterface for OpenAi {
    fn parse_set_string<'a>(
        &'a self,
        input: &'a str,
        known_exercises: &'a [String],
    ) -> BoxFuture<'a, Result<ParsedSet>> {
        Box::pin(async move {
            let creds = Self::get_creds().await?;
            let prompt_builder = PromptBuilder::new(known_exercises);

            let messages = vec![
                ChatCompletionMessage {
                    role: ChatCompletionMessageRole::System,
                    content: Some(prompt_builder.system_prompt()),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(prompt_builder.user_prompt(input)),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            ];
            let result_completion = ChatCompletion::builder(&self.model, messages.clone())
                .response_format(ChatCompletionResponseFormat::json_object())
                .credentials(creds.clone())
                //.temperature(0.1)
                .create()
                .await?;
            let result_message = result_completion.choices.first().unwrap().message.clone();
            let parsed: ParsedSet = serde_json::from_str(&result_message.content.unwrap().trim())?;
            Ok(parsed)
        })
    }
}
