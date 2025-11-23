DROP INDEX IF EXISTS idx_request_strings_user_id;

DROP INDEX IF EXISTS idx_workout_sessions_date;
DROP INDEX IF EXISTS idx_workout_sessions_user_date;
DROP INDEX IF EXISTS idx_workout_sessions_status;
DROP INDEX IF EXISTS idx_workout_sessions_user_id;

DROP INDEX IF EXISTS idx_workout_sets_status;
DROP INDEX IF EXISTS idx_workout_sets_created_at;
DROP INDEX IF EXISTS idx_workout_sets_session_exercise;
DROP INDEX IF EXISTS idx_workout_sets_exercise_id;
DROP INDEX IF EXISTS idx_workout_sets_session_id;

DROP INDEX IF EXISTS idx_exercise_equipment_equipment_id;
DROP INDEX IF EXISTS idx_exercise_equipment_exercise_id;

DROP INDEX IF EXISTS idx_exercise_muscles_relation_type;
DROP INDEX IF EXISTS idx_exercise_muscles_muscle_id;
DROP INDEX IF EXISTS idx_exercise_muscles_exercise_id;

DROP TABLE IF EXISTS workout_sets;
DROP TABLE IF EXISTS workout_sessions;
DROP TABLE IF EXISTS request_strings;
DROP TABLE IF EXISTS exercise_equipment;
DROP TABLE IF EXISTS exercise_muscles;
DROP TABLE IF EXISTS exercises;
DROP TABLE IF EXISTS equipment;
DROP TABLE IF EXISTS muscles;
DROP TABLE IF EXISTS users;

-- Drop migrations tracking table last
DROP TABLE IF EXISTS _migrations;
