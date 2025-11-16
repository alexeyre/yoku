import SwiftUI
import Combine
import YokuUniffi
// Shared models used by views


struct ExerciseModel: Identifiable, Hashable {
    let id: UUID = UUID()
    var name: String
    var sets: [ExerciseSetModel]
}
struct ExerciseSetModel: Identifiable, Hashable {
    let id: UUID = UUID()
    var label: String
}

final class Session: ObservableObject {
    // Published application state
    @Published var exercises: [ExerciseModel] = [];

    @Published var expanded: Set<UUID> = []
    @Published var activeExerciseID: UUID?
    @Published var activeSetID: UUID?
    
    // Make the backing session optional until async setup completes
    @Published var session: YokuUniffi.Session?

    // Simple elapsed time tracking (seconds)
    @Published var elapsedTime: TimeInterval = 0

    private var timerCancellable: AnyCancellable?

    // Keep initializer synchronous and non-throwing for @StateObject and previews
    init() {
        initializeActiveSelectionIfNeeded()
    }

    // Call this from a View .task to initialize the backend session
    @MainActor
    func setup(dbPath: String, model: String) async throws {
        let created = try await YokuUniffi.createSession(dbPath: dbPath, model: model)
        self.session = created
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
    var activeExercise: ExerciseModel? {
        guard let id = activeExerciseID else { return nil }
        return exercises.first { $0.id == id }
    }

    func indexOfActiveSet(in exercise: ExerciseModel?) -> Int? {
        guard let exercise = exercise, let active = activeSetID else { return nil }
        return exercise.sets.firstIndex { $0.id == active }
    }

    // MARK: - Expansion and selection helpers
    func isExpanded(_ exercise: ExerciseModel) -> Bool {
        expanded.contains(exercise.id)
    }

    func toggle(expansionFor exercise: ExerciseModel, expandIfCollapsed: Bool = false) {
        if expanded.contains(exercise.id) {
            expanded.remove(exercise.id)
        } else if expandIfCollapsed {
            expanded.insert(exercise.id)
        }
    }

    func setActiveExercise(_ exercise: ExerciseModel) {
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
    func dataSeries(for exercise: ExerciseModel?) -> [Int] {
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
