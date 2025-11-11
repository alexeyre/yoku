// parser module
pub mod llm;

use serde::{Deserialize, Deserializer, Serialize};

/// Centralized prompt builder for workout set parsing
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
                         - 10: Maximum effort, absolute limit, could not do another rep, also known as a one-rep max (1RM)\n\
                         - 9.5: Could not do another rep, but could have added slightly more weight\n\
                         - 9: Could do 1 more rep\n\
                         - 8.5: Could definitely do 1 more rep, maybe 2\n\
                         - 8: Could do 2 more reps\n\
                         - 7.5: Could do 2-3 more reps\n\
                         - 7: Could do 3 more reps with good form\n\
                         - 6: Could do 4-5 more reps\n\
                         - 5 and below: Very light effort, many reps in reserve\n\
                         Common descriptions: 'hard', 'tough', 'difficult' → ~8-9 RPE; 'easy', 'light' → ~5-6 RPE; 'moderate' → ~7 RPE";

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

// Custom deserializer that accepts both integers and floats for reps
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_reps_as_integer() {
        let json = r#"{
            "exercise": "bench press",
            "weight": 100.0,
            "reps": 5,
            "rpe": 8.0,
            "set_count": null,
            "tags": [],
            "aoi": null
        }"#;

        let parsed: ParsedSet = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.reps, Some(5));
    }

    #[test]
    fn test_deserialize_reps_as_float() {
        // LLM sometimes returns 5.0 instead of 5
        let json = r#"{
            "exercise": "bench press",
            "weight": 100.0,
            "reps": 5.0,
            "rpe": 8.0,
            "set_count": null,
            "tags": [],
            "aoi": null
        }"#;

        let parsed: ParsedSet = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.reps, Some(5));
    }

    #[test]
    fn test_deserialize_reps_rounds_float() {
        let json = r#"{
            "exercise": "bench press",
            "weight": 100.0,
            "reps": 5.7,
            "rpe": 8.0,
            "set_count": null,
            "tags": [],
            "aoi": null
        }"#;

        let parsed: ParsedSet = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.reps, Some(6)); // 5.7 rounds to 6
    }

    #[test]
    fn test_deserialize_reps_null() {
        let json = r#"{
            "exercise": "plank",
            "weight": null,
            "reps": null,
            "rpe": 8.0,
            "set_count": null,
            "tags": [],
            "aoi": "held for 60 seconds"
        }"#;

        let parsed: ParsedSet = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.reps, None);
    }

    #[test]
    fn test_deserialize_negative_reps_fails() {
        let json = r#"{
            "exercise": "bench press",
            "weight": 100.0,
            "reps": -5,
            "rpe": 8.0,
            "set_count": null,
            "tags": [],
            "aoi": null
        }"#;

        let result: Result<ParsedSet, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_multiple_sets() {
        let json = r#"{
            "exercise": "bench press",
            "weight": 100.0,
            "reps": 5,
            "rpe": 8.0,
            "set_count": 5,
            "tags": [],
            "aoi": null
        }"#;

        let parsed: ParsedSet = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.set_count, Some(5));
        assert_eq!(parsed.reps, Some(5));
    }
}
