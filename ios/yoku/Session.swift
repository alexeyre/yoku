import Combine
import Foundation
import SwiftUI
import YokuUniffi

//extension YokuUniffi.Session: @unchecked Sendable {}
//extension YokuUniffi.WorkoutSession: @unchecked Sendable {}
//extension YokuUniffi.Exercise: @unchecked Sendable {}
//extension YokuUniffi.WorkoutSet: @unchecked Sendable {}

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
    let weight: Double
    let reps: Int64
    let rpe: Double?
    let notes: String?

    init(id: UUID = UUID(), backendID: Int64? = nil, label: String, weight: Double, reps: Int64, rpe: Double? = nil, notes: String? = nil) {
        self.id = id
        self.backendID = backendID
        self.label = label
        self.weight = weight
        self.reps = reps
        self.rpe = rpe
        self.notes = notes
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

    // New: lifts map keyed by backend exercise id
    @Published private(set) var liftsByExerciseId: [Int64: [(Date, Double)]] = [:]

    private let backend = BackendSessionCoordinator()
    private var timerCancellable: AnyCancellable?
    private var exerciseIDMap: [Int64: UUID] = [:]
    private var setIDMap: [Int64: UUID] = [:]

    init() {}

    func setup(dbPath: String, model: String, fastModel: String) async throws {
        do {
            let snapshot = try await backend.setup(dbPath: dbPath, model: model, fastModel: fastModel)
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
    
    func updateWorkoutSet(id: Int64, weight: Double, reps: Int64) async throws {
        let snapshot = try await backend.updateWorkoutSet(id, weight: weight, reps: reps)
        apply(snapshot: snapshot)
    }
    
    func setWorkoutIntention(intention: String?) async throws {
        do {
            try await backend.setWorkoutIntention(intention: intention)
            // Refresh to get updated workout session
            let snapshot = try await backend.snapshot()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func getWorkoutSuggestions() async throws -> [YokuUniffi.WorkoutSuggestion] {
        do {
            return try await backend.getWorkoutSuggestions()
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }
    
    func deleteWorkout(id: Int64) async throws {
        let snapshot = try await backend.deleteWorkout(id)
        apply(snapshot: snapshot)
    }
    
    func deleteSet(id: Int64) async throws {
        let snapshot = try await backend.deleteSet(id)
        apply(snapshot: snapshot)
    }

    func classifyAndProcessInput(input: String) async throws {
        do {
            try await backend.classifyAndProcessInput(input: input)
            // Refresh to get updated state
            let snapshot = try await backend.snapshot()
            apply(snapshot: snapshot)
            lastError = nil
        } catch {
            lastError = error
            throw mapCoordinatorError(error)
        }
    }

    func setActiveWorkoutSessionId(_ id: Int64) async throws {
        do {
            let snapshot = try await backend.setActiveWorkoutSessionId(id)
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

    func dataSeries(for exercise: ExerciseModel?) -> [(Date, Double)] {
        guard let exercise else { return [] }
        // Use the fetched lifts by backend id, map to Ints for charting
        if let backendID = exercise.backendID,
            let lifts = liftsByExerciseId[backendID]
        {
            return lifts
        }
        return []
    }

    // MARK: - Internal helpers

    private func apply(snapshot: BackendSessionCoordinator.Snapshot) {
        session = snapshot.session
        activeWorkoutSession = snapshot.workout
        liftsByExerciseId = snapshot.liftsByExerciseId
        updateExercises(with: snapshot.exercises, sets: snapshot.sets)
    }

    private func updateExercises(
        with backendExercises: [YokuUniffi.Exercise],
        sets backendSets: [YokuUniffi.WorkoutSet]
    ) {
        let sortedExercises = backendExercises.sorted {
            $0.name().localizedCaseInsensitiveCompare($1.name()) == .orderedAscending
        }

        let setsGrouped = Dictionary(grouping: backendSets, by: { $0.exerciseId() })
        var nextExerciseMap: [Int64: UUID] = [:]
        var nextSetMap: [Int64: UUID] = [:]
        var models: [ExerciseModel] = []

        nextExerciseMap.reserveCapacity(sortedExercises.count)
        nextSetMap.reserveCapacity(backendSets.count)
        models.reserveCapacity(sortedExercises.count)

        for exercise in sortedExercises {
            let backendID = exercise.id()
            let sets = setsGrouped[backendID] ?? []
            guard !sets.isEmpty else {
                continue
            }

            let exerciseUUID = exerciseIDMap[backendID] ?? UUID()
            nextExerciseMap[backendID] = exerciseUUID

            let setModels = sets.map { backendSet -> ExerciseSetModel in
                let backendSetID = backendSet.id()
                let setUUID = setIDMap[backendSetID] ?? UUID()
                nextSetMap[backendSetID] = setUUID
                let label = "Set \(backendSet.id())"
                let weight = backendSet.weight()
                let reps = backendSet.reps()
                let rpe = backendSet.rpe()
                let notes = backendSet.notes()
                return ExerciseSetModel(
                    id: setUUID, backendID: backendSetID, label: label, weight: weight, reps: reps, rpe: rpe, notes: notes)
            }

            let model = ExerciseModel(
                id: exerciseUUID,
                backendID: backendID,
                name: exercise.name(),
                sets: setModels
            )
            models.append(model)
        }

        exerciseIDMap = nextExerciseMap
        setIDMap = nextSetMap

        guard models != exercises else {
            return
        }

        exercises = models
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
        liftsByExerciseId = [:]
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
                    ExerciseSetModel(label: "8 reps", weight: 10.0, reps: 5),
                    ExerciseSetModel(label: "8 reps", weight: 10.0, reps: 5),
                    ExerciseSetModel(label: "6 reps", weight: 10.0, reps: 6),
                ]),
            ExerciseModel(
                name: "Squat",
                sets: [
                    ExerciseSetModel(label: "5 reps", weight: 10.0, reps: 5),
                    ExerciseSetModel(label: "5 reps", weight: 10.0, reps: 5),
                    ExerciseSetModel(label: "5 reps", weight: 10.0, reps: 5),
                ]),
            ExerciseModel(
                name: "Pull Ups",
                sets: [
                    ExerciseSetModel(label: "10 reps", weight: 10.0, reps: 10),
                    ExerciseSetModel(label: "8 reps", weight: 10.0, reps: 8),
                    ExerciseSetModel(label: "6 reps", weight: 10.0, reps: 6),
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
        let liftsByExerciseId: [Int64: [(Date, Double)]]
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
    private var fastModel: String?

    func setup(dbPath: String, model: String, fastModel: String) async throws -> Snapshot {
        if let current = session,
            databasePath == dbPath,
            self.model == model,
            self.fastModel == fastModel
        {
            return try await snapshot(for: current)
        }

        let created = try await YokuUniffi.createSession(dbPath: dbPath, model: model, fastModel: fastModel)
        session = created
        databasePath = dbPath
        self.model = model
        self.fastModel = fastModel
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

    func setActiveWorkoutSessionId(_ id: Int64) async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.setSessionWorkoutSessionId(session: session, id: id)
        return try await snapshot(for: session)
    }

    func getLiftsForExercise(_ id: Int64) async throws -> [YokuUniffi.LiftDataPoint] {
        let session = try requireSession()
        return try await YokuUniffi.getLiftsForExercise(
            session: session, exerciseId: id, limit: 100)
    }
    
    func deleteWorkout(_ id: Int64) async throws -> Snapshot {
        let session = try requireSession()
        let _ = try await YokuUniffi.deleteWorkout(session: session, id: id)
        return try await snapshot(for: session)
    }
    
    func deleteSet(_ id: Int64) async throws -> Snapshot {
        let session = try requireSession()
        let _ = try await YokuUniffi.deleteSetFromWorkout(session: session, id: id)
        return try await snapshot(for: session)
    }
    
    func updateWorkoutSet(_ id: Int64, weight: Double, reps: Int64) async throws -> Snapshot {
        let session = try requireSession()
        let _ = try await YokuUniffi.updateWorkoutSet(session: session, setId: id, reps: reps, weight: weight)
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

    func setWorkoutIntention(intention: String?) async throws {
        let session = try requireSession()
        try await YokuUniffi.setWorkoutIntention(session: session, intention: intention)
    }

    func getWorkoutSuggestions() async throws -> [YokuUniffi.WorkoutSuggestion] {
        let session = try requireSession()
        return try await YokuUniffi.getWorkoutSuggestions(session: session)
    }

    func classifyAndProcessInput(input: String) async throws -> Snapshot {
        let session = try requireSession()
        try await YokuUniffi.classifyAndProcessInput(session: session, input: input)
        return try await snapshot(for: session)
    }

    private func snapshot(for session: YokuUniffi.Session) async throws -> Snapshot {
        async let workoutTask = YokuUniffi.getSessionWorkoutSession(session: session)
        async let exercisesTask = YokuUniffi.getAllExercises(session: session)
        async let setsTask = YokuUniffi.getAllSets(session: session)

        let workout = try? await workoutTask
        let exercises = (try? await exercisesTask) ?? []
        let sets = (try? await setsTask) ?? []

        // Concurrently fetch lifts for each exercise (best-effort)
        let liftsByExerciseId: [Int64: [(Date, Double)]] = await withTaskGroup(of: (Int64, [(Date, Double)]?).self) {
            group in
            for ex in exercises {
                let exId = ex.id()
                group.addTask {
                    do {
                        let liftDataPoints = try await YokuUniffi.getLiftsForExercise(
                            session: session, exerciseId: exId, limit: 100)
                        let lifts: [(Date, Double)] = liftDataPoints.map { (Date(timeIntervalSince1970: TimeInterval($0.timestamp())), $0.lift()) }
                        return (exId, lifts)
                    } catch {
                        return (exId, nil)
                    }
                }
            }

            var dict: [Int64: [(Date, Double)]] = [:]
            for await (id, lifts) in group {
                dict[id] = lifts ?? []
            }
            return dict
        }

        return Snapshot(
            session: session,
            workout: workout,
            exercises: exercises,
            sets: sets,
            liftsByExerciseId: liftsByExerciseId
        )
    }

    private func requireSession() throws -> YokuUniffi.Session {
        guard let session else { throw CoordinatorError.missingSession }
        return session
    }
}
