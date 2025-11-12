use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use ollama_rs::generation::parameters::TimeUnit;
use openai::{Credentials, chat::*};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::OnceCell;
use tokio::time::sleep;

pub struct PromptBuilder {
    known_exercises: Vec<String>,
}

impl PromptBuilder {
    pub fn new(known_exercises: &[String]) -> Self {
        Self {
            known_exercises: known_exercises.to_vec(),
        }
    }

    pub fn system_prompt(&self) -> String {
        "You are a precise workout set parser. \
         Your goal is to extract structured information from short workout log strings. \
         Always return a strict JSON object matching this schema: \
         {\"exercise\": string, \"weight\": float|null, \"reps\": integer|null, \"rpe\": float|null, \"set_count\": integer|null, \"tags\": [string], \"aoi\": string|null, \"original_string\": string}. \
         CRITICAL: 'reps' and 'set_count' must be integers (5, not 5.0). \
         Never include explanations or text outside of the JSON object.".to_string()
    }

    pub fn user_prompt(&self, input: &str) -> String {
        let known_exercises_section = if self.known_exercises.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nKnown exercises in the database:\n{}\n\n\
                 IMPORTANT: When the user mentions an exercise, try to match it to one of the known exercises above if it's clearly the same exercise (e.g., 'bench' -> 'bench press'). \
                 However, you are FREE TO CREATE NEW EXERCISES if:\n\
                 - The user clearly means a different exercise not in the list\n\
                 - The user uses a specific variation (e.g., 'incline bench press' vs 'bench press')\n\
                 - You're not confident about the match\n\n\
                 If the user ONLY provides weight/reps/RPE without mentioning an exercise name, set 'exercise' to null.",
                self.known_exercises.join(", ")
            )
        };

        let rpe_scale = "\n\nRPE (Rate of Perceived Exertion) Scale:\n\
                         - 10: Maximum effort, absolute limit, could not do another rep, also known as a one-rep max (1RM)\n                         - 9.5: Could not do another rep, but could have added slightly more weight\n                         - 9: Could do 1 more rep\n                         - 8.5: Could definitely do 1 more rep, maybe 2\n                         - 8: Could do 2 more reps\n                         - 7.5: Could do 2-3 more reps\n                         - 7: Could do 3 more reps with good form\n                         - 6: Could do 4-5 more reps\n                         - 5 and below: Very light effort, many reps in reserve\n                         Common descriptions: 'hard', 'tough', 'difficult' → ~8-9 RPE; 'easy', 'light' → ~5-6 RPE; 'moderate' → ~7 RPE";

        format!(
            "Parse the following workout log:\n{}{}{}\n\n\
             - 'exercise': the movement name. If no exercise is mentioned, use null \"\"\n\
             - 'weight': numeric load in kilograms or pounds if specified; otherwise null\n\
             - 'reps': INTEGER number of repetitions (e.g., 5, 8, 12, NOT 5.0); otherwise null\n\
             - 'rpe': Rate of Perceived Exertion (1-10 scale, can be decimal like 8.5). Use the RPE scale above to interpret user descriptions\n\
             - 'set_count': INTEGER number of sets if mentioned (e.g., '5 sets of 5 reps' -> set_count: 5, reps: 5); otherwise null or 1\n\
             - 'tags': any hashtags or key terms (like 'strength', 'hypertrophy', 'warmup') as a list\n\
             - 'aoi': any other information you feel is pertinent to include that does not fit in another category\n\
             - 'original_string': the exact input string\n\
             IMPORTANT: 'reps' and 'set_count' must be integers (5, not 5.0). 'weight' and 'rpe' can be floats.\n\
             Return only valid JSON conforming to the schema.",
            input, known_exercises_section, rpe_scale
        )
    }
}

fn deserialize_reps<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrFloat {
        Int(i32),
        Float(f64),
    }

    match Option::<IntOrFloat>::deserialize(deserializer)? {
        None => Ok(None),
        Some(IntOrFloat::Int(i)) => Ok(Some(i)),
        Some(IntOrFloat::Float(f)) => {
            if f.is_finite() && f >= 0.0 {
                Ok(Some(f.round() as i32))
            } else {
                Err(Error::custom(format!("invalid reps value: {}", f)))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedSet {
    pub exercise: String,
    pub weight: Option<f32>,
    #[serde(deserialize_with = "deserialize_reps")]
    pub reps: Option<i32>,
    pub rpe: Option<f32>,
    #[serde(deserialize_with = "deserialize_reps")]
    pub set_count: Option<i32>,
    pub tags: Vec<String>,
    pub aoi: Option<String>,
    #[serde(skip_deserializing)]
    pub original_string: String,
}

impl ParsedSet {
    pub fn with_original(mut p: ParsedSet, original: String) -> ParsedSet {
        p.original_string = original;
        p
    }
}

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

type MockFn = Arc<dyn Fn(&str, &str) -> String + Send + Sync>;

enum LlmBackend {
    OpenAi { model: String },
    Ollama { model: String },
    Mock { responder: MockFn },
}

pub struct LlmInterface {
    backend: LlmBackend,
}

static OPENAI_CREDS: OnceCell<Credentials> = OnceCell::const_new();
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

static OLLAMA_CLIENT: OnceCell<Arc<ollama_rs::Ollama>> = OnceCell::const_new();
const OLLAMA_DEFAULT_MODEL: &str = "llama3.2:3b";

impl LlmInterface {
    pub async fn new_openai(model: Option<String>) -> Result<Self> {
        let model = model.unwrap_or_else(|| OPENAI_DEFAULT_MODEL.to_string());
        Ok(Self {
            backend: LlmBackend::OpenAi { model },
        })
    }

    pub async fn new_ollama(model: Option<String>) -> Result<Self> {
        let model = model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string());
        Ok(Self {
            backend: LlmBackend::Ollama { model },
        })
    }

    pub fn new_mock_fn(f: impl Fn(&str, &str) -> String + Send + Sync + 'static) -> Self {
        Self {
            backend: LlmBackend::Mock {
                responder: Arc::new(f),
            },
        }
    }

    pub fn new_mock_map(map: std::collections::HashMap<String, String>) -> Self {
        let m = Arc::new(map);
        Self::new_mock_fn(move |system, user| {
            let key = format!("{}\n--\n{}", system, user);
            match m.get(&key) {
                Some(v) => v.clone(),
                None => "".to_string(),
            }
        })
    }

    async fn get_openai_creds() -> Result<Credentials> {
        Ok(OPENAI_CREDS
            .get_or_init(|| async { Credentials::from_env() })
            .await
            .clone())
    }

    async fn get_ollama_client() -> Result<Arc<ollama_rs::Ollama>> {
        Ok(OLLAMA_CLIENT
            .get_or_init(|| async { Arc::new(ollama_rs::Ollama::default()) })
            .await
            .clone())
    }

    pub async fn call(&self, system: &str, user: &str) -> Result<String> {
        match &self.backend {
            LlmBackend::OpenAi { model } => {
                let creds = Self::get_openai_creds().await?;
                let messages = vec![
                    ChatCompletionMessage {
                        role: ChatCompletionMessageRole::System,
                        content: Some(system.to_string()),
                        name: None,
                        function_call: None,
                        tool_call_id: None,
                        tool_calls: None,
                    },
                    ChatCompletionMessage {
                        role: ChatCompletionMessageRole::User,
                        content: Some(user.to_string()),
                        name: None,
                        function_call: None,
                        tool_call_id: None,
                        tool_calls: None,
                    },
                ];
                let result_completion = ChatCompletion::builder(model, messages.clone())
                    .response_format(ChatCompletionResponseFormat::json_object())
                    .credentials(creds.clone())
                    .create()
                    .await?;
                let result_message = result_completion
                    .choices
                    .first()
                    .ok_or_else(|| anyhow!("OpenAI returned no choices"))?
                    .message
                    .clone();
                let content = result_message
                    .content
                    .unwrap_or_else(|| "".to_string())
                    .trim()
                    .to_string();
                Ok(content)
            }
            LlmBackend::Ollama { model } => {
                let client = Self::get_ollama_client().await?;
                let options = ollama_rs::models::ModelOptions::default().temperature(0.001);
                let res = client
                    .generate(
                        ollama_rs::generation::completion::request::GenerationRequest::new(
                            model.clone(),
                            user.to_string(),
                        )
                        .options(options)
                        .system(system.to_string())
                        .keep_alive(
                            ollama_rs::generation::parameters::KeepAlive::Until {
                                time: 30,
                                unit: TimeUnit::Minutes,
                            },
                        ),
                    )
                    .await?;
                Ok(res.response.trim().to_string())
            }
            LlmBackend::Mock { responder } => {
                let r = responder(system, user);
                Ok(r.trim().to_string())
            }
        }
    }

    pub async fn call_json<T>(&self, system: &str, user: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let raw = self.call(system, user).await?;
        let stripped = strip_code_fences(&raw);
        let parsed: T = serde_json::from_str(stripped)
            .map_err(|e| anyhow!("Cannot parse LLM JSON output: {}\nError: {}", stripped, e))?;
        Ok(parsed)
    }

    pub async fn call_with_retry(
        &self,
        system: &str,
        user: &str,
        max_attempts: usize,
        base_delay: Duration,
    ) -> Result<String> {
        if max_attempts == 0 {
            return Err(anyhow!("max_attempts must be >= 1"));
        }

        let mut attempt: usize = 0;
        loop {
            attempt += 1;
            match self.call(system, user).await {
                Ok(s) => return Ok(s),
                Err(e) => {
                    if attempt >= max_attempts {
                        return Err(e);
                    }
                    let cap_shift = ((attempt - 1) as u32).min(10);
                    let exp = 1u128 << cap_shift;
                    let base_ms = base_delay.as_millis() as u128;
                    let delay_ms = base_ms.saturating_mul(exp);
                    let jitter = ((attempt as u64).wrapping_mul(37) % 100) as u128;
                    let total_ms = delay_ms.saturating_add(jitter);
                    let sleep_ms = if total_ms > u64::MAX as u128 {
                        u64::MAX
                    } else {
                        total_ms as u64
                    };
                    sleep(Duration::from_millis(sleep_ms)).await;
                    continue;
                }
            }
        }
    }
}

pub async fn parse_set_string(
    llm: &LlmInterface,
    input: &str,
    known_exercises: &[String],
) -> Result<ParsedSet> {
    let prompt_builder = PromptBuilder::new(known_exercises);
    let system_prompt = prompt_builder.system_prompt();
    let user_prompt = prompt_builder.user_prompt(input);
    let mut parsed: ParsedSet = llm.call_json(&system_prompt, &user_prompt).await?;
    parsed = ParsedSet::with_original(parsed, input.to_string());
    Ok(parsed)
}

pub async fn generate_equipment_to_exercise_links(
    llm: &LlmInterface,
    equipment: &str,
    known_exercises: &[String],
) -> Result<Vec<String>> {
    let system = "You are a helpful assistant that, given a piece of equipment, returns a JSON array of exercise names that typically use that equipment. Return only a JSON array of strings (no additional text). Example: [\"barbell back squat\", \"front squat\"]";
    let known_section = if known_exercises.is_empty() {
        "".to_string()
    } else {
        format!("\nKnown exercises: {}", known_exercises.join(", "))
    };
    let user = format!(
        "Equipment: {}\n{}\nReturn the most likely exercises (names) that use this equipment.",
        equipment, known_section
    );
    let res: Vec<String> = llm.call_json(system, &user).await?;
    Ok(res)
}

pub async fn generate_exercise_to_equipment_and_muscles(
    llm: &LlmInterface,
    exercise: &str,
    known_equipment: &[String],
    known_muscles: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    let system = "You are a helpful assistant that, given the name of an exercise, returns a JSON object with two keys: \"equipment\" and \"muscles\". Each value should be an array of strings. Return only valid JSON.";
    let known_eq = if known_equipment.is_empty() {
        "".to_string()
    } else {
        format!("\nKnown equipment: {}", known_equipment.join(", "))
    };
    let known_m = if known_muscles.is_empty() {
        "".to_string()
    } else {
        format!("\nKnown muscles: {}", known_muscles.join(", "))
    };
    let user = format!(
        "Exercise: {}\n{}\n{}\nReturn JSON like: {{\"equipment\": [\"...\"], \"muscles\": [\"...\"]}}",
        exercise, known_eq, known_m
    );

    #[derive(Deserialize)]
    struct ResShape {
        equipment: Vec<String>,
        muscles: Vec<String>,
    }

    let res: ResShape = llm.call_json(system, &user).await?;
    Ok((res.equipment, res.muscles))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mock_parse_set() {
        let reply = r#"{"exercise":"bench press","weight":100.0,"reps":5,"rpe":8.0,"set_count":5,"tags":[],"aoi":null,"original_string":""}"#;
        let llm = LlmInterface::new_mock_fn(move |_sys, _usr| reply.to_string());
        let parsed = parse_set_string(&llm, "5x5 bench press @ 8 RPE", &[])
            .await
            .unwrap();
        assert_eq!(parsed.exercise.to_lowercase(), "bench press");
        assert_eq!(parsed.reps, Some(5));
    }

    #[tokio::test]
    async fn test_retry_backoff_basic() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let llm = LlmInterface::new_mock_fn(move |_s, _u| {
            let prev = attempts_clone.fetch_add(1, Ordering::SeqCst);
            if prev < 2 {
                "".to_string()
            } else {
                r#"{"equipment":["barbell"],"muscles":["quadriceps"]}"#.to_string()
            }
        });
        let res = llm
            .call_with_retry("sys", "user", 5, Duration::from_millis(10))
            .await;
        assert!(res.is_ok());
    }
}
