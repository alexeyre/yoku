use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use ollama_rs::generation::parameters::TimeUnit;
use openai::{Credentials, chat::*};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::OnceCell;
use tokio::time::sleep;

use log::{debug, error, info, warn};

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
    OpenAi {
        model: String,
        api_key: Option<String>,
    },
    Ollama {
        model: String,
    },
    Mock {
        responder: MockFn,
    },
}

pub struct LlmInterface {
    backend: LlmBackend,
}

static OPENAI_CREDS: OnceCell<Credentials> = OnceCell::const_new();
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

static OLLAMA_CLIENT: OnceCell<Arc<ollama_rs::Ollama>> = OnceCell::const_new();
const OLLAMA_DEFAULT_MODEL: &str = "llama3.2:3b";

impl LlmInterface {
    pub async fn new_openai(api_key: Option<String>, model: Option<String>) -> Result<Self> {
        let model = model.unwrap_or_else(|| OPENAI_DEFAULT_MODEL.to_string());
        info!("LlmInterface::new_openai selected model={}", model);
        Ok(Self {
            backend: LlmBackend::OpenAi { model, api_key },
        })
    }

    pub async fn new_ollama(model: Option<String>) -> Result<Self> {
        let model = model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string());
        info!("LlmInterface::new_ollama selected model={}", model);
        Ok(Self {
            backend: LlmBackend::Ollama { model },
        })
    }

    pub fn new_mock_fn(f: impl Fn(&str, &str) -> String + Send + Sync + 'static) -> Self {
        debug!("LlmInterface::new_mock_fn creating mock backend");
        Self {
            backend: LlmBackend::Mock {
                responder: Arc::new(f),
            },
        }
    }

    pub fn new_mock_map(map: HashMap<String, String>) -> Self {
        debug!(
            "LlmInterface::new_mock_map creating mock map backend with {} entries",
            map.len()
        );
        let m = Arc::new(map);
        Self::new_mock_fn(move |system, user| {
            let key = format!("{}\n--\n{}", system, user);
            match m.get(&key) {
                Some(v) => v.clone(),
                None => "".to_string(),
            }
        })
    }

    async fn get_openai_creds(api_key: &Option<String>) -> Result<Credentials> {
        debug!(
            "LlmInterface::get_openai_creds called; api_key provided={}",
            api_key.is_some()
        );
        Ok(OPENAI_CREDS
            .get_or_init(|| async {
                match api_key {
                    Some(key) => {
                        debug!("LlmInterface::get_openai_creds using provided API key");
                        Credentials::new(key, "")
                    }
                    None => {
                        debug!("LlmInterface::get_openai_creds loading from env");
                        Credentials::from_env()
                    }
                }
            })
            .await
            .clone())
    }

    async fn get_ollama_client() -> Result<Arc<ollama_rs::Ollama>> {
        debug!("LlmInterface::get_ollama_client called");
        Ok(OLLAMA_CLIENT
            .get_or_init(|| async { Arc::new(ollama_rs::Ollama::default()) })
            .await
            .clone())
    }

    pub async fn call(&self, system: &str, user: &str) -> Result<String> {
        debug!(
            "LlmInterface::call invoked backend={}",
            match &self.backend {
                LlmBackend::OpenAi { model, .. } => format!("openai({})", model),
                LlmBackend::Ollama { model } => format!("ollama({})", model),
                LlmBackend::Mock { .. } => "mock".to_string(),
            }
        );

        match &self.backend {
            LlmBackend::OpenAi { model, api_key } => {
                debug!(
                    "OpenAI call using model={} api_key_present={}",
                    model,
                    api_key.is_some()
                );
                let creds = Self::get_openai_creds(api_key).await?;
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
                    .await
                    .map_err(|e| {
                        error!("OpenAI ChatCompletion.create() failed: {}", e);
                        e
                    })?;
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
                debug!("OpenAI response length={}", content.len());
                Ok(content)
            }
            LlmBackend::Ollama { model } => {
                debug!("Ollama call using model={}", model);
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
                    .await
                    .map_err(|e| {
                        error!("Ollama generate failed: {}", e);
                        e
                    })?;
                debug!("Ollama response length={}", res.response.len());
                Ok(res.response.trim().to_string())
            }
            LlmBackend::Mock { responder } => {
                debug!("Mock LLM responder invoked");
                let r = responder(system, user);
                debug!("Mock response length={}", r.len());
                Ok(r.trim().to_string())
            }
        }
    }

    pub async fn call_json<T>(&self, system: &str, user: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        debug!("call_json invoked; user_input_len={}", user.len());
        let raw = self.call(system, user).await?;
        debug!("raw LLM output len={}", raw.len());
        let stripped = strip_code_fences(&raw);
        let parsed: T = serde_json::from_str(stripped).map_err(|e| {
            error!("Cannot parse LLM JSON output: {} -- error: {}", stripped, e);
            anyhow!(format!(
                "Cannot parse LLM JSON output: {}\nError: {}",
                stripped, e
            ))
        })?;
        debug!("call_json parsed successfully");
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
            debug!(
                "call_with_retry attempt={} max_attempts={}",
                attempt, max_attempts
            );
            match self.call(system, user).await {
                Ok(s) => {
                    debug!("call_with_retry succeeded on attempt={}", attempt);
                    return Ok(s);
                }
                Err(e) => {
                    warn!("call failed on attempt {}: {}", attempt, e);
                    if attempt >= max_attempts {
                        error!("call_with_retry exhausted attempts={}", attempt);
                        return Err(e);
                    }
                    let cap_shift = ((attempt - 1) as u32).min(20);
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
                    debug!(
                        "call_with_retry sleeping ms={} before next attempt",
                        sleep_ms
                    );
                    sleep(Duration::from_millis(sleep_ms)).await;
                    continue;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ParseExample {
    pub input: String,
    pub output_json: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EquipmentToExercisesExample {
    pub equipment: String,
    pub result_json: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExerciseToEqMusExample {
    pub exercise: String,
    pub result_json: String,
}

#[derive(Clone)]
pub struct PromptContext {
    pub known_exercises: Vec<String>,
    pub known_equipment: Vec<String>,
    pub known_muscles: Vec<String>,
    pub parse_examples: Vec<ParseExample>,
    pub equipment_examples: Vec<EquipmentToExercisesExample>,
    pub exercise_examples: Vec<ExerciseToEqMusExample>,
    pub max_examples: usize,
    pub max_example_chars: usize,
}

impl Default for PromptContext {
    fn default() -> Self {
        Self {
            known_exercises: vec![],
            known_equipment: vec![],
            known_muscles: vec![],
            parse_examples: vec![],
            equipment_examples: vec![],
            exercise_examples: vec![],
            max_examples: 3,
            max_example_chars: 1500,
        }
    }
}

pub enum LinkKind {
    EquipmentToExercises,
    ExerciseToEquipmentMusclesVariants,
}

pub struct PromptBuilder {
    ctx: PromptContext,
}

impl PromptBuilder {
    pub fn new(ctx: PromptContext) -> Self {
        debug!(
            "PromptBuilder::new created with known_exercises={} known_equipment={} known_muscles={}",
            ctx.known_exercises.len(),
            ctx.known_equipment.len(),
            ctx.known_muscles.len()
        );
        Self { ctx }
    }

    fn examples_block_for_parse(&self) -> String {
        let mut block = String::new();
        let mut count = 0usize;
        for ex in &self.ctx.parse_examples {
            if count >= self.ctx.max_examples {
                break;
            }
            if block.len() + ex.input.len() + ex.output_json.len() > self.ctx.max_example_chars {
                break;
            }
            block.push_str(&format!(
                "Input: \"{}\"\nOutput:\n{}\n\n",
                ex.input, ex.output_json
            ));
            count += 1;
        }
        debug!("examples_block_for_parse returning {} examples", count);
        block
    }

    fn examples_block_for_equipment_links(&self) -> String {
        let mut block = String::new();
        let mut count = 0usize;
        for ex in &self.ctx.equipment_examples {
            if count >= self.ctx.max_examples {
                break;
            }
            if block.len() + ex.equipment.len() + ex.result_json.len() > self.ctx.max_example_chars
            {
                break;
            }
            block.push_str(&format!(
                "Equipment: {}\nExercises: {}\n\n",
                ex.equipment, ex.result_json
            ));
            count += 1;
        }
        debug!(
            "examples_block_for_equipment_links returning {} examples",
            count
        );
        block
    }

    fn examples_block_for_exercise_links(&self) -> String {
        let mut block = String::new();
        let mut count = 0usize;
        for ex in &self.ctx.exercise_examples {
            if count >= self.ctx.max_examples {
                break;
            }
            if block.len() + ex.exercise.len() + ex.result_json.len() > self.ctx.max_example_chars {
                break;
            }
            block.push_str(&format!(
                "Exercise: {}\nResult: {}\n\n",
                ex.exercise, ex.result_json
            ));
            count += 1;
        }
        debug!(
            "examples_block_for_exercise_links returning {} examples",
            count
        );
        block
    }

    pub fn system_parse_prompt(&self) -> String {
        "You are a precise workout set parser. Return only a single JSON object matching the schema: {\"exercise\": string|null, \"weight\": float|null, \"reps\": integer|null, \"rpe\": float|null, \"set_count\": integer|null, \"tags\": [string], \"aoi\": string|null, \"original_string\": string}. 'reps' and 'set_count' must be integers.".to_string()
    }

    pub fn user_parse_prompt(&self, input: &str) -> String {
        let known = if self.ctx.known_exercises.is_empty() {
            "".to_string()
        } else {
            format!(
                "\nKnown exercises: {}\n",
                self.ctx.known_exercises.join(", ")
            )
        };
        let ex_block = self.examples_block_for_parse();
        format!(
            "Parse the following workout log:\n{}\n{}{}\nReturn only valid JSON matching the schema.",
            input, known, ex_block
        )
    }

    pub fn system_link_prompt(&self, kind: LinkKind) -> String {
        match kind {
            LinkKind::EquipmentToExercises => {
                "Given a piece of equipment, return a JSON array of exercise names that typically use that equipment. Return only a JSON array of strings.".to_string()
            }
            LinkKind::ExerciseToEquipmentMusclesVariants => {
                "Given an exercise name, return a JSON object with keys \"equipment\", \"muscles\", \"related_exercises\". \"equipment\" should be an array of strings. \"muscles\" should be an array of 3-element tuples (as JSON arrays) where each tuple is [muscle_name (string), relation_type (string, e.g., \"primary\", \"secondary\"), strength (number between 0.0 and 1.0)]. \"related_exercises\" should be an array of strings of names of exercises this exercise is related to. Return only valid JSON and nothing else.".to_string()
            }
        }
    }

    pub fn user_link_prompt_equipment(&self, equipment: &str) -> String {
        let known_section = if self.ctx.known_exercises.is_empty() {
            "".to_string()
        } else {
            format!("Known exercises: {}\n", self.ctx.known_exercises.join(", "))
        };
        let ex_block = self.examples_block_for_equipment_links();
        format!(
            "Equipment: {}\n{}{}\nReturn the most likely exercises (names) that use this equipment as a JSON array.",
            equipment, known_section, ex_block
        )
    }

    pub fn user_link_prompt_exercise(&self, exercise: &str) -> String {
        let known_eq = if self.ctx.known_equipment.is_empty() {
            "".to_string()
        } else {
            format!("Known equipment: {}\n", self.ctx.known_equipment.join(", "))
        };
        let known_m = if self.ctx.known_muscles.is_empty() {
            "".to_string()
        } else {
            format!("Known muscles: {}\n", self.ctx.known_muscles.join(", "))
        };
        let known_ex = if self.ctx.known_exercises.is_empty() {
            "".to_string()
        } else {
            format!("Known exercises: {}\n", self.ctx.known_exercises.join(", "))
        };
        let ex_block = self.examples_block_for_exercise_links();
        // Example output schema now expects muscles as arrays: [["Biceps","primary",0.9], ["Triceps","secondary",0.4]]
        let base = format!(
            "Exercise: {}\n{}{}{}Return JSON like: {{\"equipment\": [\"...\"], \"muscles\": [[\"Muscle Name\",\"relation_type\",strength], ...], \"related_exercises\": [\"...\", ...]}}",
            exercise, known_eq, known_m, known_ex
        );
        base + &ex_block
    }
}

pub async fn parse_set_string(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    input: &str,
) -> Result<ParsedSet> {
    debug!("parse_set_string called input_len={}", input.len());
    let system_prompt = builder.system_parse_prompt();
    let user_prompt = builder.user_parse_prompt(input);
    let mut parsed: ParsedSet = llm.call_json(&system_prompt, &user_prompt).await?;
    parsed = ParsedSet::with_original(parsed, input.to_string());
    info!(
        "parse_set_string parsed exercise='{}' reps={:?} rpe={:?}",
        parsed.exercise, parsed.reps, parsed.rpe
    );
    Ok(parsed)
}

pub async fn generate_equipment_to_exercise_links(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    equipment: &str,
) -> Result<Vec<String>> {
    debug!(
        "generate_equipment_to_exercise_links called equipment='{}'",
        equipment
    );
    let system = builder.system_link_prompt(LinkKind::EquipmentToExercises);
    let user = builder.user_link_prompt_equipment(equipment);
    let res: Vec<String> = llm.call_json(&system, &user).await?;
    info!(
        "generate_equipment_to_exercise_links returned {} suggestions",
        res.len()
    );
    Ok(res)
}

pub async fn generate_exercise_to_equipment_and_muscles(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    exercise: &str,
) -> Result<(Vec<String>, Vec<(String, String, f32)>, Vec<String>)> {
    debug!(
        "generate_exercise_to_equipment_and_muscles called exercise='{}'",
        exercise
    );
    let system = builder.system_link_prompt(LinkKind::ExerciseToEquipmentMusclesVariants);
    let user = builder.user_link_prompt_exercise(exercise);
    #[derive(Deserialize)]
    struct ResShape {
        equipment: Vec<String>,
        muscles: Vec<(String, String, f32)>,
        related_exercises: Vec<String>,
    }
    let res: ResShape = llm.call_json(&system, &user).await?;
    info!(
        "generate_exercise_to_equipment_and_muscles parsed equipment={} muscles={} related_exercises={}",
        res.equipment.len(),
        res.muscles.len(),
        res.related_exercises.len()
    );
    Ok((res.equipment, res.muscles, res.related_exercises))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_parse_examples() {
        let ctx = PromptContext {
            known_exercises: vec!["Barbell Back Squat".into(), "Deadlift".into()],
            parse_examples: vec![
                ParseExample { input: "5x5 barbell squat 100kg @8 RPE".into(), output_json: r#"{"exercise":"Barbell Back Squat","weight":100.0,"reps":5,"rpe":8.0,"set_count":5,"tags":[],"aoi":null,"original_string":"5x5 barbell squat 100kg @8 RPE"}"#.into() }
            ],
            ..Default::default()
        };
        let builder = PromptBuilder::new(ctx);
        let reply = r#"{"exercise":"Barbell Back Squat","weight":100.0,"reps":5,"rpe":8.0,"set_count":5,"tags":[],"aoi":null,"original_string":""}"#;
        let llm = LlmInterface::new_mock_fn(move |_s, _u| reply.to_string());
        let parsed = parse_set_string(&llm, &builder, "5x5 barbell squat 100kg @8 RPE")
            .await
            .unwrap();
        assert_eq!(parsed.exercise.to_lowercase(), "barbell back squat");
        assert_eq!(parsed.reps, Some(5));
        let links_ctx = PromptContext {
            known_exercises: vec!["Barbell Back Squat".into(), "Deadlift".into()],
            equipment_examples: vec![EquipmentToExercisesExample {
                equipment: "Barbell".into(),
                result_json: r#"["Barbell Back Squat","Deadlift"]"#.into(),
            }],
            ..Default::default()
        };
        let lbuilder = PromptBuilder::new(links_ctx);
        let equip_llm = LlmInterface::new_mock_fn(move |_s, _u| {
            r#"["Barbell Back Squat","Deadlift"]"#.to_string()
        });
        let suggestions = generate_equipment_to_exercise_links(&equip_llm, &lbuilder, "Barbell")
            .await
            .unwrap();
        assert!(
            suggestions
                .iter()
                .any(|s| s.to_lowercase().contains("squat"))
        );
    }
}
