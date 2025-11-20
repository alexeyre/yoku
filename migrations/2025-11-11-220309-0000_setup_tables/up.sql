-- SQLite migration converted from Postgres
-- - UUIDs are stored as 16-byte INTEGERs; the application must supply 16-byte UUID values on INSERT
-- - Timestamps stored as INTEGER epoch seconds (UTC). Use CAST(strftime('%s','now') AS INTEGER) for defaults.
-- - Removed Postgres-specific extensions and PL/pgSQL trigger/function. updated_at should be set by the application on UPDATE.
PRAGMA foreign_keys = ON;

-- Muscles
CREATE TABLE IF NOT EXISTS muscles (
    id INTEGER NOT NULL PRIMARY KEY, -- 16-byte UUID (application supplied)
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Exercises
CREATE TABLE IF NOT EXISTS exercises (
    id INTEGER NOT NULL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Join table between exercises and muscles
CREATE TABLE IF NOT EXISTS exercise_muscles (
    exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    muscle_id INTEGER NOT NULL REFERENCES muscles(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    PRIMARY KEY (exercise_id, muscle_id)
);

-- Users
CREATE TABLE IF NOT EXISTS users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Request strings (raw user input)
CREATE TABLE IF NOT EXISTS request_strings (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    string TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Workout sessions
-- - `user_id` is nullable so sessions can be created without a user context
-- - `date` defaults to today (YYYY-MM-DD)
-- - `status` indicates if workout is 'in_progress' or 'completed'
CREATE TABLE IF NOT EXISTS workout_sessions (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    name TEXT,
    date TEXT NOT NULL DEFAULT (DATE('now')),
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    summary TEXT,
    intention TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK(status IN ('in_progress', 'completed')),
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Individual sets performed (one row per set)
CREATE TABLE IF NOT EXISTS workout_sets (
    id INTEGER NOT NULL PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    request_string_id INTEGER NOT NULL REFERENCES request_strings(id) ON DELETE CASCADE,
    weight REAL NOT NULL,
    reps INTEGER NOT NULL,
    set_index INTEGER NOT NULL,     -- 1-based index of the set within the exercise in that session
    rpe REAL,
    notes TEXT,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

-- Note: original Postgres trigger/function to auto-update updated_at removed.
-- Recommendation: set `updated_at = CAST(strftime('%s','now') AS INTEGER)` from the application when performing UPDATEs.

-- Indexes
CREATE INDEX IF NOT EXISTS idx_workout_sessions_user_date ON workout_sessions (user_id, date);
CREATE INDEX IF NOT EXISTS idx_workout_sessions_status ON workout_sessions (status);
CREATE INDEX IF NOT EXISTS idx_request_strings_user_id ON request_strings (user_id);
CREATE INDEX IF NOT EXISTS idx_exercise_muscles_exercise_id ON exercise_muscles (exercise_id);
CREATE INDEX IF NOT EXISTS idx_exercise_muscles_muscle_id ON exercise_muscles (muscle_id);
CREATE INDEX IF NOT EXISTS idx_workout_sets_session_id ON workout_sets (session_id);
CREATE INDEX IF NOT EXISTS idx_workout_sets_exercise_id ON workout_sets (exercise_id);

-- Keep unique indexes declared explicitly where helpful (some already created via UNIQUE constraints)
CREATE UNIQUE INDEX IF NOT EXISTS ux_exercises_slug ON exercises (slug);
CREATE UNIQUE INDEX IF NOT EXISTS ux_users_username ON users (username);

-- Workout suggestion cache
CREATE TABLE IF NOT EXISTS workout_suggestion_cache (
    session_id INTEGER NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    cache_key TEXT NOT NULL,
    suggestions_json TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    expires_at INTEGER NOT NULL,
    PRIMARY KEY (session_id, cache_key)
);

CREATE INDEX IF NOT EXISTS idx_workout_suggestion_cache_session_id ON workout_suggestion_cache (session_id);
CREATE INDEX IF NOT EXISTS idx_workout_suggestion_cache_expires_at ON workout_suggestion_cache (expires_at);

-- Update existing rows to have status 'completed' (for data migration)
UPDATE workout_sessions SET status = 'completed' WHERE status IS NULL;
