-- Enable pgcrypto for gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Trigger function to auto-update updated_at
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Muscles
CREATE TABLE muscles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Exercises
CREATE TABLE exercises (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slug text NOT NULL UNIQUE,
  name text NOT NULL,
  description text,
  created_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Join table between exercises and muscles
CREATE TABLE exercise_muscles (
    exercise_id uuid NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    muscle_id uuid NOT NULL REFERENCES muscles(id) ON DELETE CASCADE,
    relation_type text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY (exercise_id, muscle_id)
);

-- Users
CREATE TABLE users (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  username text NOT NULL UNIQUE,
  created_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Request strings (raw user input)
CREATE TABLE request_strings (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    string text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Plans table removed (plans feature deferred)

-- Workout sessions (compatible with CLI flows)
-- - `user_id` is nullable so sessions can be created without a user context
-- - `name` added to store an optional session title
-- - `date` gets a default of today to preserve existing create flows
-- - `duration_seconds` added for simple numeric duration usage
CREATE TABLE workout_sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid REFERENCES users(id) ON DELETE CASCADE,
    name text,
    date date NOT NULL DEFAULT CURRENT_DATE,
    duration_seconds int NOT NULL DEFAULT 0,
    notes text,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Individual sets performed (one row per set)
CREATE TABLE workout_sets (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id uuid NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    exercise_id uuid NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    request_string_id uuid NOT NULL REFERENCES request_strings(id) ON DELETE CASCADE,
    weight real NOT NULL,
    reps int NOT NULL,
    set_index int NOT NULL,     -- 1-based index of the set within the exercise in that session
    rpe real,
    notes text,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

-- Triggers to maintain updated_at for tables that have it
CREATE TRIGGER trg_muscles_set_updated_at
BEFORE UPDATE ON muscles
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_exercises_set_updated_at
BEFORE UPDATE ON exercises
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_exercise_muscles_set_updated_at
BEFORE UPDATE ON exercise_muscles
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_request_strings_set_updated_at
BEFORE UPDATE ON request_strings
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- plans triggers removed (plans table removed)

CREATE TRIGGER trg_workout_sessions_set_updated_at
BEFORE UPDATE ON workout_sessions
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_workout_sets_set_updated_at
BEFORE UPDATE ON workout_sets
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Indexes
CREATE INDEX idx_workout_sessions_user_date ON workout_sessions (user_id, date);
CREATE INDEX idx_request_strings_user_id ON request_strings (user_id);
CREATE INDEX idx_exercise_muscles_exercise_id ON exercise_muscles (exercise_id);
CREATE INDEX idx_exercise_muscles_muscle_id ON exercise_muscles (muscle_id);
CREATE INDEX idx_workout_sets_session_id ON workout_sets (session_id);
CREATE INDEX idx_workout_sets_exercise_id ON workout_sets (exercise_id);

-- Keep unique indexes declared explicitly where helpful (some already created via UNIQUE constraints)
CREATE UNIQUE INDEX ux_exercises_slug ON exercises (slug);
CREATE UNIQUE INDEX ux_users_username ON users (username);
