use std::sync::Arc;

// use diesel_async::RunQueryDsl;
use openai::{chat::*, Credentials};
use anyhow::Result;
use tokio::sync::OnceCell;

use crate::parser::ParsedSet;

#[allow(async_fn_in_trait)] // TODO: if something weird breaks, maybe this is why
pub trait LlmInterface {
    async fn parse_set_string(input: &str) -> Result<ParsedSet>;
}

use ollama_rs::{
    models::ModelOptions,
    Ollama as OllamaSdk
};
use ollama_rs::generation::completion::request::GenerationRequest;


pub struct Ollama;

const OLLAMA_MODEL: &str = "llama3.2:3b";
const OLLAMA_CLIENT: OnceCell<Arc<OllamaSdk>> = OnceCell::const_new();

impl Ollama {
    async fn get_client() -> Result<Arc<OllamaSdk>> {
        Ok(OLLAMA_CLIENT
           .get_or_init(|| async { Arc::new(OllamaSdk::default()) })
           .await
           .clone()
        )
    }
}
impl LlmInterface for Ollama {
    async fn parse_set_string(input: &str) -> Result<ParsedSet> {
        let client = Self::get_client().await?;
        let system_prompt = "You are a precise workout set parser. \
                             Your goal is to extract structured information from short workout log strings. \
                             Always return a strict JSON object matching this schema: \
                             {\"exercise\": string, \"weight\": float|null, \"reps\": float|null, \"rpe\": float|null, \"tags\": [string], \"aoi\": string|null, \"original_string\": string}. \
                             Never include explanations or text outside of the JSON object.";

        let user_prompt = format!(
            "Parse the following workout log:\n{}\n\n\
             - 'exercise': the movement name (e.g., bench press, squat, deadlift, pull-ups)\n\
             - 'weight': numeric load in kilograms or pounds if specified; otherwise null\n\
             - 'reps': number of repetitions; otherwise null\n\
             - 'rpe': numeric rate of perceived exertion (1–10) if mentioned, otherwise null\n\
             - 'tags': any hashtags or key terms (like 'strength', 'hypertrophy', 'warmup') as a list\n\
             - 'aoi': any other information you feel is pertinent to include that does not fit in another category\n\
             - 'original_string': the exact input string\n\
             Return only valid JSON conforming to the schema.",
            input
        );

        let options = ModelOptions::default()
            .temperature(0.1);

        let res = client.generate(GenerationRequest::new(OLLAMA_MODEL.to_string(), user_prompt).options(options).system(system_prompt)).await;

        let parsed = serde_json::from_str(&res?.response.trim())?;
        let parsed = ParsedSet::with_original(parsed, input.into());
        Ok(parsed)
    }
}





static OPENAI_CREDS: OnceCell<Credentials> = OnceCell::const_new();
pub struct OpenAi;
impl OpenAi {
    async fn get_creds() -> Result<Credentials> {
        Ok(
            OPENAI_CREDS
                .get_or_init(|| async {
                    Credentials::from_env()
                })
                .await
                .clone()
        )
    }
}
impl LlmInterface for OpenAi {
    async fn parse_set_string(input: &str) -> Result<ParsedSet> {

    let creds = Self::get_creds().await?;
    println!("CREDS: {}", creds.api_key());
    let messages = vec![
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(
            "You are a precise workout set parser. \
             Your goal is to extract structured information from short workout log strings. \
             Always return a strict JSON object matching this schema: \
             {\"exercise\": string, \"weight\": float|null, \"reps\": float|null, \"rpe\": float|null, \"tags\": [string], \"aoi\": string|null, \"original_string\": string}. \
             Never include explanations or text outside of the JSON object."
                .to_string(),
                ),
            name: None,
            function_call: None,
            tool_call_id: None,
            tool_calls: None
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(format!(
            "Parse the following workout log:\n{}\n\n\
             - 'exercise': the movement name (e.g., bench press, squat, deadlift, pull-ups)\n\
             - 'weight': numeric load in kilograms or pounds if specified; otherwise null\n\
             - 'reps': number of repetitions; otherwise null\n\
             - 'rpe': numeric rate of perceived exertion (1–10) if mentioned, otherwise null\n\
             - 'tags': any hashtags or key terms (like 'strength', 'hypertrophy', 'warmup') as a list\n\
             - 'aoi': any other information you feel is pertinent to include that does not fit in another category\n\
             - 'original_string': the exact input string\n\
             Return only valid JSON conforming to the schema.",
            input
        )),
            name: None,
            function_call: None,
            tool_call_id: None,
            tool_calls: None
        }
    ];
    let result_completion = ChatCompletion::builder("gpt-4.1-mini", messages.clone())
        .response_format(ChatCompletionResponseFormat::json_object())
        .credentials(creds.clone())
        .temperature(0.1)
        .create()
        .await?;
    let result_message = result_completion.choices.first().unwrap().message.clone();
    let parsed: ParsedSet = serde_json::from_str(&result_message.content.unwrap().trim())?;
    Ok(parsed)
    }
}
