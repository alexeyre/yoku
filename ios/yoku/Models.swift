import Foundation
import YokuUniffi

struct ExerciseModel: Identifiable, Hashable {
    let id: UUID
    let backendID: Int64?
    var name: String
    var sets: [ExerciseSetModel]

    init(id: UUID = UUID(), backendID: Int64? = nil, name: String, sets: [ExerciseSetModel]) {
        self.id = id
        self.backendID = backendID
        self.name = name
        self.sets = sets
    }
}

struct ExerciseSetModel: Identifiable, Hashable {
    let id: UUID
    let backendID: Int64?
    var label: String
    var weight: Double
    var reps: Int64
    var rpe: Double?

    init(id: UUID = UUID(), backendID: Int64? = nil, label: String, weight: Double, reps: Int64, rpe: Double?) {
        self.id = id
        self.backendID = backendID
        self.label = label
        self.weight = weight
        self.reps = reps
        self.rpe = rpe
    }
}

struct WorkoutSessionModel: Identifiable, Hashable {
    let id: UUID
    var name: String?
    var date: Date

    init(id: UUID = UUID(), name: String?, date: Date) {
        self.id = id
        self.name = name
        self.date = date
    }
}

enum SessionError: LocalizedError {
    case backendNotInitialized
    case operationFailed(String)

    var errorDescription: String? {
        switch self {
        case .backendNotInitialized:
            return "The workout session backend is not initialized yet."
        case .operationFailed(let message):
            return message
        }
    }
}

