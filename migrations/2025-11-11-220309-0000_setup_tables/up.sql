PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS muscle_groups (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS muscles (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    group_id INTEGER NOT NULL REFERENCES muscle_groups(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS exercises (
    id INTEGER NOT NULL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS equipment (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS exercise_equipment (
    exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    equipment_id INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    PRIMARY KEY (exercise_id, equipment_id)
);

CREATE TABLE IF NOT EXISTS exercise_muscles (
    exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    muscle_id INTEGER NOT NULL REFERENCES muscles(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    strength REAL NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    PRIMARY KEY (exercise_id, muscle_id)
);

CREATE TABLE IF NOT EXISTS users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS request_strings (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    string TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS workout_sessions (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    name TEXT,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    summary TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK(status IN ('in_progress', 'completed')),
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE TABLE IF NOT EXISTS workout_sets (
    id INTEGER NOT NULL PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    request_string_id INTEGER NOT NULL REFERENCES request_strings(id) ON DELETE CASCADE,
    weight REAL NOT NULL,
    reps INTEGER NOT NULL,
    set_index INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'planned' CHECK(status IN ('planned', 'completed')),
    rpe REAL,
    notes TEXT,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);

CREATE INDEX IF NOT EXISTS idx_exercise_muscles_exercise_id ON exercise_muscles(exercise_id);
CREATE INDEX IF NOT EXISTS idx_exercise_muscles_muscle_id ON exercise_muscles(muscle_id);
CREATE INDEX IF NOT EXISTS idx_exercise_muscles_relation_type ON exercise_muscles(relation_type);

CREATE INDEX IF NOT EXISTS idx_exercise_equipment_exercise_id ON exercise_equipment(exercise_id);
CREATE INDEX IF NOT EXISTS idx_exercise_equipment_equipment_id ON exercise_equipment(equipment_id);

CREATE INDEX IF NOT EXISTS idx_workout_sets_session_id ON workout_sets(session_id);
CREATE INDEX IF NOT EXISTS idx_workout_sets_exercise_id ON workout_sets(exercise_id);
CREATE INDEX IF NOT EXISTS idx_workout_sets_session_exercise ON workout_sets(session_id, exercise_id);
CREATE INDEX IF NOT EXISTS idx_workout_sets_created_at ON workout_sets(created_at);
CREATE INDEX IF NOT EXISTS idx_workout_sets_status ON workout_sets(status);

CREATE INDEX IF NOT EXISTS idx_workout_sessions_user_id ON workout_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_workout_sessions_status ON workout_sessions(status);
CREATE INDEX IF NOT EXISTS idx_workout_sessions_user_datetime ON workout_sessions(user_id, datetime);
CREATE INDEX IF NOT EXISTS idx_workout_sessions_datetime ON workout_sessions(datetime);

CREATE INDEX IF NOT EXISTS idx_request_strings_user_id ON request_strings(user_id);
