use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use yoku_core::db::models::DisplayableSet;
use yoku_core::db::operations::{
    create_workout_session, delete_workout_session, delete_workout_set, get_all_exercises,
    get_all_workout_sessions, get_exercise, get_or_create_exercise, get_sets_for_session,
};
use yoku_core::graph::GraphManager;
use yoku_core::llm::{
    LlmInterface, ParsedSet, PromptBuilder, PromptContext,
    generate_exercise_to_equipment_and_muscles,
};
use yoku_core::session::Session;

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
    Delete {
        id: String,
    },

    /// List all sets in a session
    ListSets {
        session_id: String,
    },

    /// Add a parsed set to a session (we use the LLM parser)
    AddSet {
        session_id: String,
        #[arg(value_parser)]
        input: String,
    },

    /// Delete a set by UUID
    DeleteSet {
        set_id: String,
    },

    SuggestExerciseLinks {
        name: String,
    },

    /// Dump a textual view of the graph (for debugging)
    DumpGraph {
        /// Max number of relationships to print
        #[arg(short, long, default_value_t = 50)]
        limit: i64,
    },
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
    let parser: Option<LlmInterface> = match cli.command {
        Commands::AddSet { .. } | Commands::SuggestExerciseLinks { .. } => {
            let llm = match cli.parser {
                ParserType::Ollama => LlmInterface::new_ollama(cli.model.clone()).await?,
                ParserType::OpenAI => LlmInterface::new_openai(cli.model.clone()).await?,
            };
            Some(llm)
        }
        _ => None,
    };

    let prompt_context = PromptContext::default();
    let prompt_builder = PromptBuilder::new(prompt_context);

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
        Commands::SuggestExerciseLinks { name } => {
            if let Some(p) = parser {
                cmd_suggest_exercise_links(&name, &p, &prompt_builder).await?
            } else {
                eprintln!("Parser not initialized");
            }
        }
        Commands::DumpGraph { limit } => {
            let gm = GraphManager::connect().await?;
            println!("Dumping graph with limit {}", limit);
            gm.dump_graph(limit).await?;
        }
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

async fn cmd_add_set(session_id: &str, input: &str, parser: LlmInterface) -> Result<()> {
    let session_uuid = Uuid::from_str(session_id)?;

    // Build a client-side Session and attach the workout id
    let mut sess = Session::new_blank().await;
    sess.set_workout_id(session_uuid).await?;

    // Fetch known exercises to help the parser be consistent
    let exercises = get_all_exercises().await?;
    let known_exs: Vec<String> = exercises.into_iter().map(|e| e.name).collect();

    // Build prompt context and builder (inject known exercises; examples may be provided from seed later)
    let ctx = yoku_core::llm::PromptContext {
        known_exercises: known_exs.clone(),
        ..Default::default()
    };
    let builder = yoku_core::llm::PromptBuilder::new(ctx);

    // Parse the input using the selected LLM backend and prompt builder
    let parsed: ParsedSet = yoku_core::llm::parse_set_string(&parser, &builder, input).await?;

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

async fn cmd_suggest_exercise_links(
    name: &str,
    llm: &LlmInterface,
    builder: &PromptBuilder,
) -> Result<()> {
    let exercise = get_or_create_exercise(name).await?;
    let (equip_links, muscle_links) =
        generate_exercise_to_equipment_and_muscles(llm, builder, &exercise.name).await?;
    for suggestion in equip_links {
        println!("Suggested equipment link: {}", suggestion);
    }
    for (muscle, link_type, strength) in muscle_links {
        println!(
            "Suggested muscle link: {}--{}<{}> {}",
            name, muscle, link_type, strength
        );
    }
    Ok(())
}
