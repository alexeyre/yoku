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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub selected_set_backend_id: Option<i64>,
    pub visible_set_backend_ids: Vec<i64>,
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
            selected_set_backend_id: None,
            visible_set_backend_ids: vec![],
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

    pub fn system_input_classification_prompt(&self) -> String {
        r#"You are a command classifier for a workout tracking app. Analyze the user input and return a JSON array of commands to execute.

Return a JSON object with a "commands" array. Each command should be fully parsed with all fields extracted.

Command types:
1. "add_set" - Add one or more workout sets. Fields: exercise (string), weight (number|null), reps (integer|null), rpe (number|null), set_count (integer|null, defaults to 1), tags (array of strings), aoi (string|null), original_string (string)
   - If user says "add 3 sets of bench press 100kg x 5", return 3 separate add_set commands
   - Parse exercise names, weights, reps, RPE from natural language
   - RPE is rate of perceived exersion 0 is No effort, 1 Very light, 2 to 3 Light, 4 to 6 Moderate, 7 to 8 Vigorous, 9 Very Hard, and 10 is Maximum Effort. The scale can also be interpreted as the number of reps in reserve, where one rep in reserve is 9 (10 minus 1), etc. The user may say "one rep max" indicating 0 reps in reserve and an RPE 10 for example.
   - Use known exercises from context when possible

2. "remove_set" - Remove one or more sets. Fields: set_id (integer|null), description (string|null)
   - If set_id is provided, use it directly
   - If description is provided (e.g., "last bench press set"), the backend will resolve it
   - If user says "remove the last 2 sets", return 2 remove_set commands
   - If user says "this set" and a currently selected set ID is provided in context, use that set_id
   - If user says "all the sets I can see" or similar and visible set IDs are provided, use those set_ids (one command per set)

3. "edit_set" - Edit an existing set. Fields: set_id (integer|null), description (string|null), exercise (string|null), weight (number|null), reps (integer|null), rpe (number|null)
   - Only include fields that should be changed
   - If user says "change last bench press to 105kg", include set_id or description pointing to the last bench press set, and weight=105.0
   - If user says "no that should be 80kg" referring to most recent set, use description or set_id from recent sets
   - If user says "this set" and a currently selected set ID is provided in context, use that set_id

4. "change_intention" - Change workout intention/goal. Fields: intention (string)
   - Extract the intention from natural language

5. "unknown" - Fallback for unclassifiable input. Fields: input (string)

Examples:
- "add 3 sets of bench press 100kg x 5" → [{"command_type": "add_set", "exercise": "Bench Press", "weight": 100.0, "reps": 5, "set_count": 1, "tags": [], "aoi": null, "original_string": "bench press 100kg x 5"}, ... (3 times)]
- "remove the last 2 sets" → [{"command_type": "remove_set", "set_id": null, "description": "last set"}, {"command_type": "remove_set", "set_id": null, "description": "second to last set"}]
- "change last bench press to 105kg" → [{"command_type": "edit_set", "set_id": null, "description": "last bench press set", "weight": 105.0, "exercise": null, "reps": null, "rpe": null}]
- "no that should be 80kg" → [{"command_type": "edit_set", "set_id": null, "description": "most recent set", "weight": 80.0, ...}]

Return only valid JSON: {"commands": [...]}"#.to_string()
    }

    pub fn user_input_classification_prompt(&self, input: &str, workout_context: &str) -> String {
        let mut context_parts = vec![format!("User input: \"{}\"", input)];
        
        // Add set context information
        if let Some(selected_id) = self.ctx.selected_set_backend_id {
            context_parts.push(format!(
                "Currently selected set ID: {} (when user says 'this set', they mean set ID {})",
                selected_id, selected_id
            ));
        }
        
        if !self.ctx.visible_set_backend_ids.is_empty() {
            let visible_ids_str = self.ctx.visible_set_backend_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            context_parts.push(format!(
                "Visible set IDs: [{}] (when user says 'all the sets I can see' or similar, they mean these set IDs)",
                visible_ids_str
            ));
        }
        
        context_parts.push(format!("Workout Context:\n{}", workout_context));
        context_parts.push("Analyze the input and return a JSON array of commands to execute. All fields should be fully parsed.".to_string());
        
        context_parts.join("\n\n")
    }

    pub fn system_suggestion_prompt(&self) -> String {
        r#"You are an expert fitness coach providing actionable workout suggestions. Your suggestions must be SPECIFIC and ACTIONABLE, not vague general advice.

Return a JSON object with a 'suggestions' array. Each suggestion should have:
- 'title' (string): A specific, actionable suggestion
- 'subtitle' (optional string): Additional context or details
- 'suggestion_type' (one of: 'exercise', 'progression', 'volume', 'accessory', 'completion')
- 'exercise_name' (optional string): For exercise or progression suggestions, specify the exercise name
- 'reasoning' (optional string): Brief explanation

CRITICAL: Avoid vague suggestions like "do progressive overload" or "focus on form". Instead, provide specific, actionable recommendations.

GOOD EXAMPLES:
1. Exercise recommendation:
   {"title": "Add Barbell Rows", "subtitle": "3 sets of 8-10 reps @7-8 RPE", "suggestion_type": "exercise", "exercise_name": "Barbell Row", "reasoning": "Balances the pressing work you've done"}

2. Specific progression:
   {"title": "Increase Bench Press to 87.5kg", "subtitle": "You've been doing 85kg x 5, try 87.5kg x 4-5 @8 RPE", "suggestion_type": "progression", "exercise_name": "Bench Press", "reasoning": "2.5kg increase based on your recent performance"}

3. Workout completion:
   {"title": "Consider wrapping up", "subtitle": "You've done 4 heavy compounds and 3 accessories - good volume for today", "suggestion_type": "completion", "reasoning": "High volume and intensity already achieved"}

4. Volume adjustment:
   {"title": "Add 1 more set to Squats", "subtitle": "You did 3 sets, add a 4th at 90% of your working weight", "suggestion_type": "volume", "exercise_name": "Barbell Back Squat", "reasoning": "Room for more volume based on RPE"}

BAD EXAMPLES (DO NOT DO THIS):
- {"title": "Do progressive overload"} - Too vague
- {"title": "Focus on form"} - Not actionable
- {"title": "Add more volume"} - Not specific
- {"title": "Try a new exercise"} - Doesn't specify which

GUIDELINES:
- For exercise suggestions: Always specify the exercise name and provide rep/set/RPE guidance
- For progression suggestions: Specify exact weight/rep changes based on past performance
- For completion suggestions: Use when workout is already very taxing (high volume, high intensity, or user seems fatigued)
- For volume suggestions: Specify exactly how many sets/reps to add
- Base all suggestions on the actual past performance data provided
- Consider workout intention if provided
- Balance muscle groups appropriately

Return only valid JSON."#.to_string()
    }

    pub fn user_suggestion_prompt(
        &self,
        current_exercises: &[(String, i64)], // (exercise_name, set_count)
        intention: Option<&str>,
        past_performance: &str, // Summary of past performance
    ) -> String {
        let exercises_list: String = current_exercises
            .iter()
            .map(|(name, count)| format!("- {} ({} sets)", name, count))
            .collect::<Vec<_>>()
            .join("\n");

        let intention_section = if let Some(intent) = intention {
            format!("\nWorkout Intention: {}\n", intent)
        } else {
            String::new()
        };

        let total_sets: i64 = current_exercises.iter().map(|(_, count)| count).sum();
        let workout_intensity_note = if total_sets > 15 {
            "\nNOTE: This workout already has significant volume. Consider whether to suggest completion or lighter accessory work.\n"
        } else if current_exercises.is_empty() {
            "\nNOTE: Workout just starting. Focus on exercise recommendations based on intention.\n"
        } else {
            "\nNOTE: Room for more work. Consider progression on current exercises or adding complementary exercises.\n"
        };

        format!(
            "Current workout:\n{}\n{}Past Performance Summary:\n{}\n{}\nProvide 3-5 SPECIFIC, ACTIONABLE suggestions. For each suggestion:\n\n1. EXERCISE RECOMMENDATIONS: If suggesting a new exercise, specify the exact exercise name, rep range, and RPE (e.g., \"Add Barbell Rows: 3 sets of 8-10 reps @7-8 RPE\")\n\n2. PROGRESSION SUGGESTIONS: If suggesting progression on an existing exercise, specify:\n   - Exact weight change (e.g., \"Increase Bench Press from 85kg to 87.5kg\")\n   - Rep range (e.g., \"Try 4-5 reps @8 RPE\")\n   - Base this on the past performance data provided\n\n3. COMPLETION SUGGESTIONS: If the workout is already very taxing (high volume, high intensity, or user appears fatigued), suggest wrapping up with a completion-type suggestion\n\n4. VOLUME SUGGESTIONS: If suggesting more volume, specify exactly how many sets/reps to add (e.g., \"Add 1 more set to Squats at 90% working weight\")\n\nBase all suggestions on the actual past performance data. Be specific with weights, reps, and RPE ranges. Avoid vague advice.\n\nReturn JSON with a 'suggestions' array.",
            exercises_list, intention_section, past_performance, workout_intensity_note
        )
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutSuggestion {
    pub title: String,
    pub subtitle: Option<String>,
    pub suggestion_type: String, // "exercise", "progression", "volume", "accessory"
    pub exercise_name: Option<String>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    #[serde(rename = "intention")]
    Intention,
    #[serde(rename = "set")]
    Set,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputClassification {
    pub input_type: InputType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command_type")]
pub enum Command {
    #[serde(rename = "add_set")]
    AddSet {
        exercise: String,
        weight: Option<f64>,
        reps: Option<i64>,
        rpe: Option<f64>,
        set_count: Option<i64>,
        tags: Vec<String>,
        aoi: Option<String>,
        original_string: String,
    },
    #[serde(rename = "remove_set")]
    RemoveSet {
        set_id: Option<i64>,
        description: Option<String>,
    },
    #[serde(rename = "edit_set")]
    EditSet {
        set_id: Option<i64>,
        description: Option<String>,
        exercise: Option<String>,
        weight: Option<f64>,
        reps: Option<i64>,
        rpe: Option<f64>,
    },
    #[serde(rename = "change_intention")]
    ChangeIntention { intention: String },
    #[serde(rename = "unknown")]
    Unknown { input: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandList {
    pub commands: Vec<Command>,
}

pub async fn classify_input_type(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    input: &str,
) -> Result<InputType> {
    debug!("classify_input_type called input_len={}", input.len());
    let system = builder.system_input_classification_prompt();
    let user = builder.user_input_classification_prompt(input, "");
    let classification: InputClassification = llm.call_json(&system, &user).await?;
    info!(
        "classify_input_type classified as {:?}",
        classification.input_type
    );
    Ok(classification.input_type)
}

pub async fn classify_commands(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    input: &str,
    workout_context: &str,
) -> Result<Vec<Command>> {
    debug!(
        "classify_commands called input_len={} context_len={}",
        input.len(),
        workout_context.len()
    );
    let system = builder.system_input_classification_prompt();
    let user = builder.user_input_classification_prompt(input, workout_context);
    let command_list: CommandList = llm.call_json(&system, &user).await?;
    info!(
        "classify_commands returned {} commands",
        command_list.commands.len()
    );
    Ok(command_list.commands)
}

pub async fn generate_workout_suggestions(
    llm: &LlmInterface,
    builder: &PromptBuilder,
    current_exercises: &[(String, i64)],
    intention: Option<&str>,
    past_performance: &str,
) -> Result<Vec<WorkoutSuggestion>> {
    debug!(
        "generate_workout_suggestions called exercises={} intention={:?}",
        current_exercises.len(),
        intention
    );
    let system = builder.system_suggestion_prompt();
    let user = builder.user_suggestion_prompt(current_exercises, intention, past_performance);

    #[derive(Deserialize)]
    struct ResShape {
        suggestions: Vec<WorkoutSuggestion>,
    }

    let res: ResShape = llm.call_json(&system, &user).await?;
    info!(
        "generate_workout_suggestions returned {} suggestions",
        res.suggestions.len()
    );
    Ok(res.suggestions)
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
