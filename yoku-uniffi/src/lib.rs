// yoku/yoku-uniffi/src/lib.rs
// NOTE: This file provides uniffi-exported stubs / TODO skeletons to match
// the iOS app's expectations. Implementations are intentionally lightweight
// placeholders so the Swift UI can call these APIs during UI development.
//
// TODO: Replace stubs with real implementations backed by `yoku_core` and
// your real database/session logic as needed.

uniffi::setup_scaffolding!();

use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tokio::sync::OnceCell;
use yoku_core::*;

/// Existing exported functions used by the app's startup flow and log polling.
/// Keep these as-is (small adaptions allowed).

#[uniffi::export]
pub async fn setup_database(path: &str) {
    // TODO: Wire up to actual DB initialization. This currently delegates
    // to the yoku_core/db helpers (if present). Keep the call so the iOS app
    // can continue to call `setupDatabase(path:)` synchronously from Swift's
    // async task.
    db::set_db_path(path).await.unwrap();
    db::init_database().await.unwrap();
}

static SESSION: std::sync::LazyLock<OnceCell<session::Session>> =
    std::sync::LazyLock::new(|| OnceCell::new());

async fn get_session() -> &'static session::Session {
    SESSION
        .get_or_init(async || session::Session::new_blank().await)
        .await
}

#[uniffi::export]
pub async fn start_blank_workout() -> i32 {
    // Create a new blank workout using the shared session object.
    let session = get_session().await;
    session.new_workout().await.unwrap();
    backend_log("Initalised a blank workout");
    session.workout_id.lock().await.clone().unwrap()
}

// Simple in-memory backend log queue that Swift can poll.
static BACKEND_LOGS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Push a backend log entry (from Rust) into the in-app queue and also emit to stderr
/// so it appears in Xcode's debug console while running under the debugger.
#[uniffi::export]
pub fn backend_log(message: &str) {
    // write to our in-memory queue
    if let Ok(mut q) = BACKEND_LOGS.lock() {
        q.push(message.into());
    }
    // also print to stderr for immediate visibility in Xcode
    eprintln!("[BE] {}", message);
}

/// Return the number of backend log entries currently stored.
#[uniffi::export]
pub fn backend_logs_count() -> i32 {
    let q = BACKEND_LOGS.lock().unwrap();
    q.len() as i32
}

/// Return all backend log entries after `start_index` (0-based).
/// Swift can call `backend_logs_count` / `backend_logs_since(lastIndex)` periodically
/// to sync new logs into the UI `LogCenter`.
#[uniffi::export]
pub fn backend_logs_since(start_index: i32) -> Vec<String> {
    let q = BACKEND_LOGS.lock().unwrap();
    let total = q.len() as i32;
    if start_index >= total || start_index < 0 {
        return Vec::new();
    }
    let start_usize = start_index as usize;
    q.iter().skip(start_usize).cloned().collect()
}

/*
    The remainder of this file contains lightweight "skeleton" APIs that map to the
    features sketched in the iOS app. They are intentionally simple and purely
    in-memory so the Swift UI can iterate while you flesh out the real backend.

    When you implement these for real, prefer to:
    - Use the shared `session` / DB to persist and retrieve workouts/exercises.
    - Return structured types via uniffi (consider an UDL if you need complex objects).
    - Move heavy processing off the main thread.
*/

// Lightweight in-memory representation that mirrors the minimal shape used by the UI.
#[derive(Clone, Debug)]
struct ExerciseData {
    name: String,
    sets: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct InMemoryWorkout {
    exercises: Vec<ExerciseData>,
    active_exercise: Option<String>,
    active_set_index: Option<usize>,

    // Simple elapsed time tracking: we store accumulated seconds and an optional
    // running start Instant. We compute elapsed on demand so we don't spawn threads.
    elapsed_accum_seconds: i32,
    elapsed_running_since: Option<Instant>,
}

static IN_MEMORY_WORKOUT: LazyLock<Mutex<InMemoryWorkout>> = LazyLock::new(|| {
    let mut default = InMemoryWorkout::default();

    // Mirror the iOS preview dummy data so UI looks populated before backend is wired.
    default.exercises = vec![
        ExerciseData {
            name: "Deadlift".into(),
            sets: vec![
                "Set 1: 5 reps".into(),
                "Set 2: 5 reps".into(),
                "Set 3: 5 reps".into(),
            ],
        },
        ExerciseData {
            name: "Bench Press".into(),
            sets: vec![
                "Set 1: 8 reps".into(),
                "Set 2: 8 reps".into(),
                "Set 3: 6 reps".into(),
            ],
        },
        ExerciseData {
            name: "Pull-Up".into(),
            sets: vec![
                "Set 1: 10 reps".into(),
                "Set 2: 8 reps".into(),
                "Set 3: 6 reps".into(),
            ],
        },
        ExerciseData {
            name: "Squat".into(),
            sets: vec![
                "Set 1: 5 reps".into(),
                "Set 2: 5 reps".into(),
                "Set 3: 5 reps".into(),
            ],
        },
        ExerciseData {
            name: "Overhead Press".into(),
            sets: vec![
                "Set 1: 8 reps".into(),
                "Set 2: 6 reps".into(),
                "Set 3: 6 reps".into(),
            ],
        },
    ];

    // Default selection mirrors the Swift state initialiser.
    if let Some(first) = default.exercises.first() {
        default.active_exercise = Some(first.name.clone());
        default.active_set_index = Some(0);
    }

    Mutex::new(default)
});

/// Return a list of exercise names for the current (in-memory) workout.
/// TODO: Replace with real DB/session-backed data model.
#[uniffi::export]
pub fn list_exercises() -> Vec<String> {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    w.exercises.iter().map(|e| e.name.clone()).collect()
}

/// Return the set labels for a named exercise (exact match). If not found, returns empty vector.
#[uniffi::export]
pub fn list_sets_for_exercise(exercise_name: &str) -> Vec<String> {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    w.exercises
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(exercise_name))
        .map(|e| e.sets.clone())
        .unwrap_or_default()
}

/// Get the currently active exercise name, if any.
#[uniffi::export]
pub fn get_active_exercise() -> Option<String> {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    w.active_exercise.clone()
}

/// Get the currently active set index (0-based). Returns -1 if none active.
#[uniffi::export]
pub fn get_active_set_index() -> i32 {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    match w.active_set_index {
        Some(i) => i as i32,
        None => -1,
    }
}

/// Set the active exercise by exact name (case-insensitive). Returns `true` if found and set.
/// TODO: Consider exposing an ID-based API instead.
#[uniffi::export]
pub fn set_active_exercise(name: &str) -> bool {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    if let Some(pos) = w
        .exercises
        .iter()
        .position(|e| e.name.eq_ignore_ascii_case(name))
    {
        w.active_exercise = Some(w.exercises[pos].name.clone());
        // Ensure an active set exists for the new exercise.
        if w.exercises[pos].sets.is_empty() {
            w.active_set_index = None;
        } else {
            // If previous active set index is out of range, set to first
            let idx = w.active_set_index.unwrap_or(0);
            if idx >= w.exercises[pos].sets.len() {
                w.active_set_index = Some(0);
            } else {
                w.active_set_index = Some(idx);
            }
        }
        true
    } else {
        false
    }
}

/// Set the active set for a named exercise by 0-based index. Returns true on success.
#[uniffi::export]
pub fn set_active_set(exercise_name: &str, index: i32) -> bool {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    if let Some(pos) = w
        .exercises
        .iter()
        .position(|e| e.name.eq_ignore_ascii_case(exercise_name))
    {
        let usize_index = if index < 0 {
            return false;
        } else {
            index as usize
        };
        if usize_index < w.exercises[pos].sets.len() {
            w.active_exercise = Some(w.exercises[pos].name.clone());
            w.active_set_index = Some(usize_index);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Start elapsed time tracking. This does not spawn a background thread — we record
/// the start instant and compute elapsed on demand.
#[uniffi::export]
pub fn start_elapsed_timer() {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    if w.elapsed_running_since.is_none() {
        w.elapsed_running_since = Some(Instant::now());
    }
}

/// Stop elapsed time tracking and accumulate the run duration.
#[uniffi::export]
pub fn stop_elapsed_timer() {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    if let Some(since) = w.elapsed_running_since.take() {
        let dur = Instant::now().saturating_duration_since(since);
        w.elapsed_accum_seconds += dur.as_secs() as i32;
    }
}

/// Reset the elapsed timer back to zero.
#[uniffi::export]
pub fn reset_elapsed_timer() {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    w.elapsed_accum_seconds = 0;
    w.elapsed_running_since = None;
}

/// Return elapsed seconds (integer). If the timer is running, compute current delta.
#[uniffi::export]
pub fn get_elapsed_seconds() -> i32 {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    let mut total = w.elapsed_accum_seconds;
    if let Some(since) = w.elapsed_running_since {
        let dur = Instant::now().saturating_duration_since(since);
        total += dur.as_secs() as i32;
    }
    total
}

/// Return a one-line heuristic summary of the current workout. The iOS UI uses
/// this to display a single-line "purpose" summary. This mirrors the placeholder
/// logic in `WorkoutPurposeSummaryView`.
#[uniffi::export]
pub fn get_workout_summary() -> String {
    let w = IN_MEMORY_WORKOUT.lock().unwrap();
    let names: Vec<String> = w.exercises.iter().map(|e| e.name.to_lowercase()).collect();

    let is_upper = names.iter().any(|n| {
        n.contains("bench")
            || n.contains("press")
            || n.contains("pull")
            || n.contains("row")
            || n.contains("dip")
    });
    let is_lower = names.iter().any(|n| {
        n.contains("squat") || n.contains("deadlift") || n.contains("lunge") || n.contains("leg")
    });
    let has_compounds = names.iter().any(|n| {
        n.contains("squat") || n.contains("deadlift") || n.contains("bench") || n.contains("press")
    });
    let volume_hint: usize = w.exercises.iter().map(|e| e.sets.len()).sum();

    let focus = match (is_upper, is_lower) {
        (true, true) => "full body",
        (true, false) => "upper body",
        (false, true) => "lower body",
        _ => "general fitness",
    };

    let goal = if has_compounds && volume_hint <= 10 {
        "strength-building"
    } else if volume_hint >= 15 {
        "hypertrophy-focused"
    } else {
        "performance-focused"
    };

    format!("{} {} workout", goal, focus)
}

/// Return a short list of suggestions for an exercise name. These are simple
/// heuristics to mirror the Swift `ExerciseSuggestionsView`.
#[uniffi::export]
pub fn get_exercise_suggestions(exercise_name: &str) -> Vec<String> {
    let lower = exercise_name.to_lowercase();
    let last_default = "Aim for same reps again".to_string();

    let mut items: Vec<String> = Vec::new();

    // Next set target
    items.push(format!("Next set target: {}", last_default));

    // Progression hint
    if lower.contains("bench") || lower.contains("press") {
        items.push("Progression hint: Consider +2.5 kg next session".into());
        items.push("Accessory: DB Flyes or Triceps Pressdowns".into());
    } else if lower.contains("deadlift") || lower.contains("squat") {
        items.push("Progression hint: Hold weight until reps stabilise".into());
        items.push("Accessory: Back Extensions or Hamstring Curls".into());
    } else if lower.contains("pull") {
        items.push("Progression hint: Focus on controlled negatives".into());
        items.push("Accessory: Bicep Curls or Face Pulls".into());
    } else {
        items.push("Progression hint: Maintain form and increase load gradually".into());
        items.push("Accessory: Add core or mobility work between sets".into());
    }

    // If exercise likely has fewer than 3 sets suggest adding one
    // (best-effort heuristic: not perfect)
    items.push("Consider adding one more set for volume".into());

    items
}

/// Post a frontend-origin log entry into the backend logs. This allows the
/// Swift app to push FE logs into the same stream used by the developer console.
/// TODO: Eventually split FE/BE or provide richer typed logs.
#[uniffi::export]
pub fn post_frontend_log(message: &str) {
    // Tag and push to the same backend queue
    backend_log(&format!("[FE] {}", message));
}

/// A few tiny convenience functions for quick UI interactions while developing.

/// Add a new simple exercise with `name` and `sets` count (creates numbered labels).
/// Returns `true` on success.
#[uniffi::export]
pub fn add_exercise_with_sets(name: &str, sets: i32) -> bool {
    if sets <= 0 {
        return false;
    }
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    let mut set_labels = Vec::new();
    for i in 1..=sets {
        set_labels.push(format!("Set {}: ?", i));
    }
    w.exercises.push(ExerciseData {
        name: name.to_string(),
        sets: set_labels,
    });
    true
}

/// Remove all in-memory exercises (useful during UI iteration). Returns number removed.
#[uniffi::export]
pub fn clear_all_exercises() -> i32 {
    let mut w = IN_MEMORY_WORKOUT.lock().unwrap();
    let removed = w.exercises.len() as i32;
    w.exercises.clear();
    w.active_exercise = None;
    w.active_set_index = None;
    removed
}
