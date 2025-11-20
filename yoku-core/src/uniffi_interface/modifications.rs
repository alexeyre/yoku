#[derive(Debug, Clone, uniffi::Record)]
pub struct Modification {
    pub modification_type: ModificationType,
    pub set_id: Option<i64>,
    pub set_ids: Vec<i64>,
    pub exercise_id: Option<i64>,
    pub set: Option<std::sync::Arc<crate::uniffi_interface::objects::WorkoutSet>>,
    pub sets: Option<Vec<std::sync::Arc<crate::uniffi_interface::objects::WorkoutSet>>>,
    pub exercise: Option<std::sync::Arc<crate::uniffi_interface::objects::Exercise>>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum ModificationType {
    SetAdded,
    SetModified,
    SetRemoved,
    ExerciseAdded,
}

#[derive(Clone, uniffi::Record)]
pub struct UpdateWorkoutSetResult {
    pub set: std::sync::Arc<crate::uniffi_interface::objects::WorkoutSet>,
    pub modifications: Vec<Modification>,
}
