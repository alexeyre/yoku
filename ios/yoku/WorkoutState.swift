import SwiftUI
import Combine

// Shared models used by views
struct Exercise: Identifiable, Hashable {
    let id: UUID = UUID()
    var name: String
    var sets: [ExerciseSet]
}

struct ExerciseSet: Identifiable, Hashable {
    let id: UUID = UUID()
    var label: String
}

final class WorkoutState: ObservableObject {
    // Published application state
    @Published var exercises: [Exercise] = [
        Exercise(name: "Deadlift", sets: [
            ExerciseSet(label: "Set 1: 5 reps"),
            ExerciseSet(label: "Set 2: 5 reps"),
            ExerciseSet(label: "Set 3: 5 reps")
        ]),
        Exercise(name: "Bench Press", sets: [
            ExerciseSet(label: "Set 1: 8 reps"),
            ExerciseSet(label: "Set 2: 8 reps"),
            ExerciseSet(label: "Set 3: 6 reps")
        ]),
        Exercise(name: "Pull-Up", sets: [
            ExerciseSet(label: "Set 1: 10 reps"),
            ExerciseSet(label: "Set 2: 8 reps"),
            ExerciseSet(label: "Set 3: 6 reps")
        ]),
        Exercise(name: "Squat", sets: [
            ExerciseSet(label: "Set 1: 5 reps"),
            ExerciseSet(label: "Set 2: 5 reps"),
            ExerciseSet(label: "Set 3: 5 reps")
        ]),
        Exercise(name: "Overhead Press", sets: [
            ExerciseSet(label: "Set 1: 8 reps"),
            ExerciseSet(label: "Set 2: 6 reps"),
            ExerciseSet(label: "Set 3: 6 reps")
        ])
    ]

    @Published var expanded: Set<UUID> = []
    @Published var activeExerciseID: UUID?
    @Published var activeSetID: UUID?

    // Simple elapsed time tracking (seconds)
    @Published var elapsedTime: TimeInterval = 0

    private var timerCancellable: AnyCancellable?

    init() {
        initializeActiveSelectionIfNeeded()
    }

    // MARK: - Timer control
    func startTimer() {
        guard timerCancellable == nil else { return }
        timerCancellable = Timer.publish(every: 1.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                guard let self = self else { return }
                self.elapsedTime += 1
            }
    }

    func stopTimer() {
        timerCancellable?.cancel()
        timerCancellable = nil
    }

    // MARK: - Convenience accessors
    var activeExercise: Exercise? {
        guard let id = activeExerciseID else { return nil }
        return exercises.first { $0.id == id }
    }

    func indexOfActiveSet(in exercise: Exercise?) -> Int? {
        guard let exercise = exercise, let active = activeSetID else { return nil }
        return exercise.sets.firstIndex { $0.id == active }
    }

    // MARK: - Expansion and selection helpers
    func isExpanded(_ exercise: Exercise) -> Bool {
        expanded.contains(exercise.id)
    }

    func toggle(expansionFor exercise: Exercise, expandIfCollapsed: Bool = false) {
        if expanded.contains(exercise.id) {
            expanded.remove(exercise.id)
        } else if expandIfCollapsed {
            expanded.insert(exercise.id)
        }
    }

    func setActiveExercise(_ exercise: Exercise) {
        activeExerciseID = exercise.id
        if !(exercise.sets.contains { $0.id == activeSetID }) {
            activeSetID = exercise.sets.first?.id
        }
    }

    func initializeActiveSelectionIfNeeded() {
        guard activeExerciseID == nil, activeSetID == nil else { return }
        if let first = exercises.first {
            activeExerciseID = first.id
            activeSetID = first.sets.first?.id
            expanded.insert(first.id)
        }
    }

    // MARK: - Dummy data series generator for charts
    // Returns a small series of integers representing e.g. reps or weights
    func dataSeries(for exercise: Exercise?) -> [Int] {
        guard let exercise else { return [] }
        let lower = exercise.name.lowercased()
        if lower.contains("deadlift") {
            return [5, 5, 5]
        }
        if lower.contains("bench") {
            return [8, 8, 6]
        }
        if lower.contains("pull") {
            return [10, 8, 6]
        }
        if lower.contains("squat") {
            return [5, 5, 5]
        }
        if lower.contains("press") || lower.contains("overhead") {
            return [8, 6, 6]
        }

        // Fallback: try to parse numbers from set labels
        let numbers = exercise.sets.compactMap { set -> Int? in
            let digits = set.label.compactMap { $0.isNumber ? Int(String($0)) : nil }
            guard !digits.isEmpty else { return nil }
            return digits.reduce(0) { $0 * 10 + $1 }
        }

        if numbers.isEmpty {
            return Array(1...max(1, exercise.sets.count))
        }
        return numbers
    }
}
