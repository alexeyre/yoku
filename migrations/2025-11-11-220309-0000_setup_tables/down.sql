-- Revert the changes applied in up.sql for migration 2025-11-11-220309-0000_setup_tables
-- This file drops triggers, indexes, tables, the trigger function, and the pgcrypto extension.
-- Operations are defensive (IF EXISTS) and ordered to avoid FK/trigger dependency issues.

-- 1) Drop triggers that reference the trigger function
DROP TRIGGER IF EXISTS trg_workout_sets_set_updated_at ON workout_sets;
DROP TRIGGER IF EXISTS trg_workout_sessions_set_updated_at ON workout_sessions;
DROP TRIGGER IF EXISTS trg_request_strings_set_updated_at ON request_strings;
DROP TRIGGER IF EXISTS trg_users_set_updated_at ON users;
DROP TRIGGER IF EXISTS trg_exercise_muscles_set_updated_at ON exercise_muscles;
DROP TRIGGER IF EXISTS trg_exercises_set_updated_at ON exercises;
DROP TRIGGER IF EXISTS trg_muscles_set_updated_at ON muscles;

-- 2) Drop explicit indexes (optional; dropping tables would remove them, but do it explicitly)
DROP INDEX IF EXISTS idx_workout_sets_exercise_id;
DROP INDEX IF EXISTS idx_workout_sets_session_id;
DROP INDEX IF EXISTS idx_exercise_muscles_muscle_id;
DROP INDEX IF EXISTS idx_exercise_muscles_exercise_id;
DROP INDEX IF EXISTS idx_request_strings_user_id;
DROP INDEX IF EXISTS idx_workout_sessions_user_date;

DROP INDEX IF EXISTS ux_exercises_slug;
DROP INDEX IF EXISTS ux_users_username;

-- 3) Drop tables in an order that respects foreign-key relationships
--    Use IF EXISTS to be idempotent and CASCADE to ensure dependent objects are removed.
DROP TABLE IF EXISTS workout_sets CASCADE;
DROP TABLE IF EXISTS workout_sessions CASCADE;
DROP TABLE IF EXISTS request_strings CASCADE;
DROP TABLE IF EXISTS exercise_muscles CASCADE;
DROP TABLE IF EXISTS exercises CASCADE;
DROP TABLE IF EXISTS muscles CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- 4) Drop the trigger function used to set updated_at
DROP FUNCTION IF EXISTS set_updated_at() CASCADE;

-- 5) Optionally remove the pgcrypto extension if installed by this migration
DROP EXTENSION IF EXISTS "pgcrypto";

-- End of down migration
