// @generated automatically by Diesel CLI.

diesel::table! {
    exercises (id) {
        id -> Int4,
        name -> Text,
        equipment -> Nullable<Text>,
        primary_muscle -> Nullable<Text>,
        secondary_muscle -> Nullable<Text>,
        description -> Nullable<Text>,
    }
}

diesel::table! {
    sets (id) {
        id -> Int4,
        exercise_id -> Int4,
        workout_id -> Int4,
        weight -> Numeric,
        reps -> Int4,
        rpe -> Nullable<Numeric>,
        set_number -> Nullable<Int4>,
    }
}

diesel::table! {
    settags (id) {
        id -> Int4,
        set_id -> Int4,
        tag_id -> Int4,
    }
}

diesel::table! {
    tags (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::table! {
    workouts (id) {
        id -> Int4,
        name -> Nullable<Text>,
        performed_at -> Nullable<Timestamp>,
        notes -> Nullable<Text>,
    }
}

diesel::joinable!(sets -> exercises (exercise_id));
diesel::joinable!(sets -> workouts (workout_id));
diesel::joinable!(settags -> sets (set_id));
diesel::joinable!(settags -> tags (tag_id));

diesel::allow_tables_to_appear_in_same_query!(exercises, sets, settags, tags, workouts,);
