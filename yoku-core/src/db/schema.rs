// @generated automatically by Diesel CLI.

diesel::table! {
    exercise_muscles (exercise_id, muscle_id) {
        exercise_id -> Integer,
        muscle_id -> Integer,
        relation_type -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    exercises (id) {
        id -> Integer,
        slug -> Text,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    muscles (id) {
        id -> Integer,
        name -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    request_strings (id) {
        id -> Integer,
        user_id -> Integer,
        string -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    workout_sessions (id) {
        id -> Integer,
        user_id -> Nullable<Integer>,
        name -> Nullable<Text>,
        date -> Text,
        duration_seconds -> Integer,
        notes -> Nullable<Text>,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    workout_sets (id) {
        id -> Integer,
        session_id -> Integer,
        exercise_id -> Integer,
        request_string_id -> Integer,
        weight -> Float,
        reps -> Integer,
        set_index -> Integer,
        rpe -> Nullable<Float>,
        notes -> Nullable<Text>,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::joinable!(exercise_muscles -> exercises (exercise_id));
diesel::joinable!(exercise_muscles -> muscles (muscle_id));
diesel::joinable!(request_strings -> users (user_id));
diesel::joinable!(workout_sessions -> users (user_id));
diesel::joinable!(workout_sets -> exercises (exercise_id));
diesel::joinable!(workout_sets -> request_strings (request_string_id));
diesel::joinable!(workout_sets -> workout_sessions (session_id));

diesel::allow_tables_to_appear_in_same_query!(
    exercise_muscles,
    exercises,
    muscles,
    request_strings,
    users,
    workout_sessions,
    workout_sets,
);
