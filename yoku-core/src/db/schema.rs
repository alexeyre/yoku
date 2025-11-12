
diesel::table! {
    exercise_muscles (exercise_id, muscle_id) {
        exercise_id -> Uuid,
        muscle_id -> Uuid,
        relation_type -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    exercises (id) {
        id -> Uuid,
        slug -> Text,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    muscles (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    request_strings (id) {
        id -> Uuid,
        user_id -> Uuid,
        string -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workout_sessions (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        name -> Nullable<Text>,
        date -> Date,
        duration_seconds -> Int4,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workout_sets (id) {
        id -> Uuid,
        session_id -> Uuid,
        exercise_id -> Uuid,
        request_string_id -> Uuid,
        weight -> Float4,
        reps -> Int4,
        set_index -> Int4,
        rpe -> Nullable<Float4>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
