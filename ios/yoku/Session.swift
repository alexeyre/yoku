import Combine
import Foundation
import SwiftUI
import YokuUniffi

extension YokuUniffi.Session: @unchecked Sendable {}
extension YokuUniffi.WorkoutSession: @unchecked Sendable {}
extension YokuUniffi.Exercise: @unchecked Sendable {}
extension YokuUniffi.WorkoutSet: @unchecked Sendable {}

struct ExerciseModel: Identifiable, Hashable {
    let id: UUID
    let backendID: Int?
    var name: String
    var sets: [ExerciseSetModel]

    init(id: UUID = UUID(), backendID: Int? = nil, name: String, sets: [ExerciseSetModel]) {
        self.id = id
        self.backendID = backendID
        self.name = name
        self.sets = sets
    }
}

struct ExerciseSetModel: Identifiable, Hashable {
    let id: UUID
    let backendID: Int?
    var label: String

    init(id: UUID = UUID(), backendID: Int? = nil, label: String) {
        self.id = id
        self.backendID = backendID
        self.label = label
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

@MainActor
final class Session: ObservableObject {
    @Published var exercises: [ExerciseModel] = []
    @Published var expanded: Set<UUID> = []
    @Published var activeExerciseID: UUID?
    @Published var activeSetID: UUID?
    @Published private(set) var session: YokuUniffi.Session?
    @Published private(set) var activeWorkoutSession: YokuUniffi.WorkoutSession?
    @Published var elapsedTime: TimeInterval = 0
    @Published var lastError: Error?

    private let backend = BackendSessionCoordinator()
    private var timerCancellable: AnyCancellable?
    private var exerciseIDMap: [Int: UUID] = [:]
    private var setIDMap: [Int: UUID] = [:]

    init() {}

    func setup(dbPath: String, model: String) async throws {
        do {
            let snapshot = try await backend.setup(dbPath: dbPath, model: model)
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw error
        }
    }

    func addSetFromString(input: String) async throws {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        do {
            let snapshot = try await backend.addSetFromString(trimmed)
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func setActiveWorkoutSessionId(_ id: Int) async throws {
        do {
            let snapshot = try await backend.setActiveWorkoutSessionId(Int32(id))
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func createBlankWorkoutSession() async throws {
        do {
            let snapshot = try await backend.createBlankWorkoutSession()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func resetDatabase() async throws {
        do {
            let snapshot = try await backend.resetDatabase()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func refreshActiveWorkoutSession() async throws {
        do {
            let snapshot = try await backend.snapshot()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            if case BackendSessionCoordinator.CoordinatorError.missingSession = error {
                resetLocalState()
                throw SessionError.backendNotInitialized
            } else {
                lastError = error
                throw error
            }
        }
    }

    func refreshSets() async throws {
        do {
            let snapshot = try await backend.snapshot()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func fetchAllWorkoutSessions() async throws -> [YokuUniffi.WorkoutSession] {
        do {
            return try await backend.fetchWorkoutSessions()
        } catch {
            throw mapCoordinatorError(error)
        }
    }

    // MARK: - Timer control

    func startTimer() {
        guard timerCancellable == nil else { return }
        timerCancellable = Timer.publish(every: 1.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                guard let self else { return }
                self.elapsedTime += 1
            }
    }

    func stopTimer() {
        timerCancellable?.cancel()
        timerCancellable = nil
    }

    var isTimerRunning: Bool {
        timerCancellable != nil
    }

    // MARK: - Accessors

    var activeExercise: ExerciseModel? {
        guard let id = activeExerciseID else { return nil }
        return exercises.first { $0.id == id }
    }

    func indexOfActiveSet(in exercise: ExerciseModel?) -> Int? {
        guard let exercise, let active = activeSetID else { return nil }
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
        let availableSetIDs = Set(exercise.sets.map { $0.id })
        if let current = activeSetID, !availableSetIDs.contains(current) {
            activeSetID = exercise.sets.first?.id
        } else if activeSetID == nil {
            activeSetID = exercise.sets.first?.id
        }
    }

    func initializeActiveSelectionIfNeeded() {
        if activeExerciseID == nil, let first = exercises.first {
            activeExerciseID = first.id
            activeSetID = first.sets.first?.id
            expanded.insert(first.id)
        } else if let exerciseID = activeExerciseID,
            let exercise = exercises.first(where: { $0.id == exerciseID }),
            activeSetID == nil
        {
            activeSetID = exercise.sets.first?.id
        }
    }

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

    // MARK: - Internal helpers

    private func apply(snapshot: BackendSessionCoordinator.Snapshot) {
        session = snapshot.session
        activeWorkoutSession = snapshot.workout
        updateExercises(with: snapshot.exercises, sets: snapshot.sets)
    }

    private func updateExercises(
        with backendExercises: [YokuUniffi.Exercise],
        sets backendSets: [YokuUniffi.WorkoutSet]
    ) {
        let sortedExercises = backendExercises.sorted {
            $0.name().localizedCaseInsensitiveCompare($1.name()) == .orderedAscending
        }

        let setsGrouped = Dictionary(grouping: backendSets, by: { Int($0.exerciseId()) })
        var nextExerciseMap: [Int: UUID] = [:]
        var nextSetMap: [Int: UUID] = [:]
        var models: [ExerciseModel] = []

        for exercise in sortedExercises {
            let backendID = Int(exercise.id())
            let exerciseUUID = exerciseIDMap[backendID] ?? UUID()
            nextExerciseMap[backendID] = exerciseUUID

            let sets = setsGrouped[backendID] ?? []
            let setModels = sets.map { backendSet -> ExerciseSetModel in
                let backendSetID = Int(backendSet.id())
                let setUUID = setIDMap[backendSetID] ?? UUID()
                nextSetMap[backendSetID] = setUUID
                let label = "Set \(backendSet.id())"
                return ExerciseSetModel(id: setUUID, backendID: backendSetID, label: label)
            }

            let model = ExerciseModel(
                id: exerciseUUID,
                backendID: backendID,
                name: exercise.name(),
                sets: setModels
            )
            models.append(model)
        }

        exercises = models
        exerciseIDMap = nextExerciseMap
        setIDMap = nextSetMap
        reconcileSelectionAfterUpdate()
    }

    private func reconcileSelectionAfterUpdate() {
        let validExerciseIDs = Set(exercises.map { $0.id })
        expanded = Set(expanded.filter { validExerciseIDs.contains($0) })

        if let currentExercise = activeExerciseID, !validExerciseIDs.contains(currentExercise) {
            activeExerciseID = nil
        }

        let validSetIDs = Set(exercises.flatMap { $0.sets }.map { $0.id })
        if let currentSet = activeSetID, !validSetIDs.contains(currentSet) {
            activeSetID = nil
        }

        initializeActiveSelectionIfNeeded()
    }

    private func resetLocalState() {
        exercises = []
        expanded = []
        activeExerciseID = nil
        activeSetID = nil
        activeWorkoutSession = nil
        session = nil
        exerciseIDMap = [:]
        setIDMap = [:]
    }

    private func mapCoordinatorError(_ error: Error) -> Error {
        if let coordinatorError = error as? BackendSessionCoordinator.CoordinatorError {
            switch coordinatorError {
            case .missingSession:
                return SessionError.backendNotInitialized
            }
        }
        return error
    }
}

// MARK: - Preview support

extension Session {
    static var preview: Session {
        let session = Session()
        session.exercises = [
            ExerciseModel(
                name: "Bench Press",
                sets: [
                    ExerciseSetModel(label: "8 reps"),
                    ExerciseSetModel(label: "8 reps"),
                    ExerciseSetModel(label: "6 reps"),
                ]),
            ExerciseModel(
                name: "Squat",
                sets: [
                    ExerciseSetModel(label: "5 reps"),
                    ExerciseSetModel(label: "5 reps"),
                    ExerciseSetModel(label: "5 reps"),
                ]),
            ExerciseModel(
                name: "Pull Ups",
                sets: [
                    ExerciseSetModel(label: "10 reps"),
                    ExerciseSetModel(label: "8 reps"),
                    ExerciseSetModel(label: "6 reps"),
                ]),
        ]
        if let first = session.exercises.first {
            session.activeExerciseID = first.id
            session.activeSetID = first.sets.first?.id
            session.expanded.insert(first.id)
        }
        return session
    }
}

// MARK: - Backend coordinator

private actor BackendSessionCoordinator {
    struct Snapshot {
        let session: YokuUniffi.Session
        let workout: YokuUniffi.WorkoutSession?
        let exercises: [YokuUniffi.Exercise]
        let sets: [YokuUniffi.WorkoutSet]
    }

    enum CoordinatorError: LocalizedError {
        case missingSession

        var errorDescription: String? {
            switch self {
            case .missingSession:
                return "No backend session is currently available."
            }
        }
    }

    private var session: YokuUniffi.Session?
    private var databasePath: String?
    private var model: String?

    func setup(dbPath: String, model: String) async throws -> Snapshot {
        if let current = session,
            databasePath == dbPath,
            self.model == model
        {
            return try await snapshot(for: current)
        }

        let created = try await YokuUniffi.createSession(dbPath: dbPath, model: model)
        session = created
        databasePath = dbPath
        self.model = model
        return try await snapshot(for: created)
    }

    func snapshot() async throws -> Snapshot {
        guard let session else { throw CoordinatorError.missingSession }
        return try await snapshot(for: session)
    }

    func addSetFromString(_ request: String) async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.addSetFromString(session: session, requestString: request)
        return try await snapshot(for: session)
    }

    func setActiveWorkoutSessionId(_ id: Int32) async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.setSessionWorkoutSessionId(session: session, id: id)
        return try await snapshot(for: session)
    }

    func createBlankWorkoutSession() async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.createBlankWorkoutSession(session: session)
        return try await snapshot(for: session)
    }

    func resetDatabase() async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.resetDatabase(session: session)
        return try await snapshot(for: session)
    }

    func fetchWorkoutSessions() async throws -> [YokuUniffi.WorkoutSession] {
        let session = try requireSession()
        return try await YokuUniffi.getAllWorkoutSessions(session: session)
    }

    private func snapshot(for session: YokuUniffi.Session) async throws -> Snapshot {
        let workout = try? await YokuUniffi.getSessionWorkoutSession(session: session)

        let fetchedExercises = try? await YokuUniffi.getAllExercises(session: session)
        let exercises = fetchedExercises?.map { $0 } ?? []

        let fetchedSets = try? await YokuUniffi.getAllSets(session: session)
        let sets = fetchedSets?.map { $0 } ?? []

        return Snapshot(
            session: session,
            workout: workout,
            exercises: exercises,
            sets: sets
        )
    }

    private func requireSession() throws -> YokuUniffi.Session {
        guard let session else { throw CoordinatorError.missingSession }
        return session
    }
}
