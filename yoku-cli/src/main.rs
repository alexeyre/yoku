use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use yoku_core::db::models::DisplayableSet;
use yoku_core::db::operations::{
    create_workout_session, delete_workout_session, delete_workout_set, get_all_exercises,
    get_all_workout_sessions, get_exercise, get_sets_for_session,
};
use yoku_core::parser::ParsedSet;
use yoku_core::parser::llm::{LlmInterface, Ollama, OpenAi};
use yoku_core::session::Session;

/// CLI entry for yoku
#[derive(Parser, Debug)]
#[command(version, about = "Yoku - Workout Tracker CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Parser backend to use for parsing freeform set strings
    #[arg(short, long, value_enum, default_value_t = ParserType::Ollama)]
    parser: ParserType,

    /// Model name to pass to the selected LLM backend (optional)
    #[arg(short, long)]
    model: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all workout sessions
    List {},

    /// Create a new workout session (optionally provide a name)
    Create {
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Delete a workout session by UUID
    Delete { id: String },

    /// List all sets in a session
    ListSets { session_id: String },

    /// Add a parsed set to a session (we use the LLM parser)
    AddSet {
        session_id: String,
        #[arg(value_parser)]
        input: String,
    },

    /// Delete a set by UUID
    DeleteSet { set_id: String },
}

#[derive(Debug, Clone, ValueEnum)]
enum ParserType {
    Ollama,
    #[value(name = "openai")]
    OpenAI,
}

impl fmt::Display for ParserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserType::Ollama => write!(f, "ollama"),
            ParserType::OpenAI => write!(f, "openai"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    // Initialize LLM parser if needed by a command that uses it.
    let parser: Option<Box<dyn LlmInterface>> = match cli.command {
        Commands::AddSet { .. } => {
            let boxed: Box<dyn LlmInterface> = match cli.parser {
                ParserType::Ollama => Ollama::new(cli.model.clone()).await?,
                ParserType::OpenAI => OpenAi::new(cli.model.clone()).await?,
            };
            Some(boxed)
        }
        _ => None,
    };

    match cli.command {
        Commands::List {} => cmd_list().await?,
        Commands::Create { name } => cmd_create(name).await?,
        Commands::Delete { id } => cmd_delete(&id).await?,
        Commands::ListSets { session_id } => cmd_list_sets(&session_id).await?,
        Commands::AddSet { session_id, input } => {
            if let Some(p) = parser {
                cmd_add_set(&session_id, &input, p).await?
            } else {
                eprintln!("Parser not initialized");
            }
        }
        Commands::DeleteSet { set_id } => cmd_delete_set(&set_id).await?,
    }

    Ok(())
}

async fn cmd_list() -> Result<()> {
    let sessions = get_all_workout_sessions().await?;
    if sessions.is_empty() {
        println!("No workout sessions found.");
        return Ok(());
    }
    for s in sessions {
        let id = s.id;
        let name = s.name.unwrap_or_else(|| "(unnamed)".to_string());
        println!("{}  —  {}", id, name);
    }
    Ok(())
}

async fn cmd_create(name: Option<String>) -> Result<()> {
    // create_workout_session(user_id, name, notes, duration_seconds)
    let ws = create_workout_session(None, name, None, None).await?;
    println!(
        "Created workout session: {} (id {})",
        ws.name.unwrap_or_default(),
        ws.id
    );
    Ok(())
}

async fn cmd_delete(id: &str) -> Result<()> {
    let uuid = Uuid::from_str(id)?;
    let deleted = delete_workout_session(uuid).await?;
    println!("Deleted {} rows for session {}", deleted, uuid);
    Ok(())
}

async fn cmd_list_sets(session_id: &str) -> Result<()> {
    let uuid = Uuid::from_str(session_id)?;
    let sets = get_sets_for_session(uuid).await?;
    if sets.is_empty() {
        println!("No sets for session {}", uuid);
        return Ok(());
    }
    for s in sets {
        // get exercise name for display
        let exercise = get_exercise(&s.exercise_id).await?;
        let display = DisplayableSet::new(s, exercise.name);
        println!("{}", display);
    }
    Ok(())
}

async fn cmd_add_set(session_id: &str, input: &str, parser: Box<dyn LlmInterface>) -> Result<()> {
    let session_uuid = Uuid::from_str(session_id)?;

    // Build a client-side Session and attach the workout id
    let mut sess = Session::new_blank().await;
    sess.set_workout_id(session_uuid).await?;

    // Fetch known exercises to help the parser be consistent
    let exercises = get_all_exercises().await?;
    let known_exs: Vec<String> = exercises.into_iter().map(|e| e.name).collect();

    // Parse the input using the selected LLM backend
    let parsed: ParsedSet = parser.parse_set_string(input, &known_exs).await?;

    // Let the session handle adding the set (it will create/get exercises as needed)
    sess.add_set_from_parsed(&parsed).await?;

    println!("Added set to session {}: {}", session_uuid, parsed.exercise);
    Ok(())
}

async fn cmd_delete_set(set_id: &str) -> Result<()> {
    let uuid = Uuid::from_str(set_id)?;
    let deleted = delete_workout_set(uuid).await?;
    println!("Deleted {} rows for set {}", deleted, uuid);
    Ok(())
}
