import Combine
import Foundation
import SwiftUI
import YokuUniffi


@MainActor
final class WorkoutStore: ObservableObject {
    @Published var exercises: [ExerciseModel] = []
    @Published var expanded: Set<UUID> = []
    @Published var activeExerciseID: UUID?
    @Published var activeSetID: UUID?

    @Published private(set) var session: YokuUniffi.Session?
    @Published private(set) var activeWorkoutSession: YokuUniffi.WorkoutSession?
    @Published var workoutSummary: String? = nil

    let timerStore = TimerStore()

    @Published private(set) var liftsByExerciseId: [Int64: [Double]] = [:]

    @Published var lastError: Error?

    private let backend = BackendService.shared
    private var exerciseIDMap: [Int64: UUID] = [:]
    private var setIDMap: [Int64: UUID] = [:]
    
    init() {
        timerStore.setSaveHandler { [weak self] elapsed in
            Task { try? await self?.backend.updateWorkoutElapsedTime(elapsedSeconds: Int64(elapsed)) }
        }
    }

    func setup(dbPath: String, model: String) async throws {
        do {
            try await backend.initialize(dbPath: dbPath, model: model)

            if let inProgressWorkout = try? await backend.getInProgressWorkoutSession() {
                let elapsedFromDB = inProgressWorkout.durationSeconds()
                timerStore.start(from: TimeInterval(elapsedFromDB))
                try await setActiveWorkoutSessionId(inProgressWorkout.id())
            } else {
                 activeWorkoutSession = nil
                 exercises = []
            }
            lastError = nil
        } catch {
            lastError = error
            throw error
        }
    }

    func setActiveWorkoutSessionId(_ id: Int64) async throws {
        do {
            try await backend.setActiveWorkoutSessionId(id)
            let state = try await backend.getActiveWorkoutState()
            apply(state: state)

            if let workout = activeWorkoutSession, workout.status() == "in_progress" {
                let elapsedFromDB = workout.durationSeconds()
                if !timerStore.isRunning {
                    timerStore.start(from: TimeInterval(elapsedFromDB))
                }
            }
            lastError = nil
        } catch {
            lastError = error
            throw error
        }
    }
    
    func createBlankWorkoutSession() async throws -> Bool {
        do {
            let hadExisting = try await backend.createBlankWorkoutSession()
            let state = try await backend.getActiveWorkoutState()
            apply(state: state)
            
            timerStore.stop()
            timerStore.start()
            lastError = nil
            return hadExisting
        } catch {
            lastError = error
            throw error
        }
    }

    func classifyAndProcessInput(input: String) async throws {
        let selectedSetBackendID: Int64? = {
            guard let activeSetID = activeSetID else { return nil }
            for exercise in exercises {
                if let set = exercise.sets.first(where: { $0.id == activeSetID }) {
                    return set.backendID
                }
            }
            return nil
        }()
        
        let visibleSetBackendIDs: [Int64] = exercises
            .filter { expanded.contains($0.id) }
            .flatMap { $0.sets }
            .compactMap { $0.backendID }
        
        let modifications = try await backend.classifyAndProcessInput(
            input: input,
            selectedSetBackendID: selectedSetBackendID,
            visibleSetBackendIDs: visibleSetBackendIDs
        )
        
        apply(modifications: modifications)
    }

    func updateWorkoutSet(id: Int64, weight: Double, reps: Int64) async throws {
        do {
            let result = try await backend.updateWorkoutSet(id: id, weight: weight, reps: reps)
            apply(modifications: result.modifications)
            lastError = nil
        } catch {
            lastError = error
            throw error
        }
    }
    
    func deleteSet(id: Int64) async throws {
        let modifications = try await backend.deleteWorkoutSet(id: id)
        apply(modifications: modifications)
    }
    
    func completeWorkout() async throws {
        let durationSeconds = Int64(timerStore.elapsedTime)
        try await backend.completeWorkoutSession(durationSeconds: durationSeconds)
        timerStore.stop()

        activeWorkoutSession = nil
        exercises = []
    }

    private func apply(state: ActiveWorkoutState) {
        self.activeWorkoutSession = state.workout
        self.workoutSummary = state.workout.summary()

        updateExercises(with: state.exercises, sets: state.sets)

        Task {
            await fetchLifts(for: state.exercises)
        }
    }
    
    private func apply(modifications: [YokuUniffi.Modification]) {
        for mod in modifications {
            handleModification(mod)
        }
    }

    private func handleModification(_ mod: YokuUniffi.Modification) {
        switch mod.modificationType {
        case .setAdded:
            if let sets = mod.sets {
                for set in sets {
                    addSet(set, to: mod.exercise)
                }
            } else if let set = mod.set {
                addSet(set, to: mod.exercise)
            }
        case .setModified:
            if let sets = mod.sets {
                for set in sets {
                    updateSet(set)
                }
            } else if let set = mod.set {
                updateSet(set)
            }
        case .setRemoved:
            if !mod.setIds.isEmpty {
                for id in mod.setIds {
                    removeSet(id: id)
                }
            } else if let setId = mod.setId {
                removeSet(id: setId)
            }
        case .exerciseAdded:
            if let exercise = mod.exercise {
                if let sets = mod.sets {
                    for set in sets {
                        addExercise(exercise, with: set)
                    }
                } else if let set = mod.set {
                     addExercise(exercise, with: set)
                }
            }
        }
    }

    private func addSet(_ backendSet: YokuUniffi.WorkoutSet, to backendExercise: YokuUniffi.Exercise?) {
        let exerciseID = backendSet.exerciseId()
        guard let index = exercises.firstIndex(where: { $0.backendID == exerciseID }) else {
            if let ex = backendExercise {
                addExercise(ex, with: backendSet)
            }
            return
        }
        
        var exercise = exercises[index]
        let setModel = mapSet(backendSet, indexInExercise: exercise.sets.count)
        exercise.sets.append(setModel)

        for (i, var s) in exercise.sets.enumerated() {
            s.label = "Set \(i + 1)"
            exercise.sets[i] = s
        }
        
        exercises[index] = exercise
    }
    
    private func updateSet(_ backendSet: YokuUniffi.WorkoutSet) {
        let exerciseID = backendSet.exerciseId()
        let setID = backendSet.id()
        
        guard let exIndex = exercises.firstIndex(where: { $0.backendID == exerciseID }),
              let setIndex = exercises[exIndex].sets.firstIndex(where: { $0.backendID == setID })
        else { return }
        
        var setModel = exercises[exIndex].sets[setIndex]
        setModel.weight = backendSet.weight()
        setModel.reps = backendSet.reps()
        setModel.rpe = backendSet.rpe()
        
        exercises[exIndex].sets[setIndex] = setModel
    }
    
    private func removeSet(id: Int64) {
        for (exIndex, exercise) in exercises.enumerated() {
            if let setIndex = exercise.sets.firstIndex(where: { $0.backendID == id }) {
                var updatedExercise = exercise
                updatedExercise.sets.remove(at: setIndex)
                
                if updatedExercise.sets.isEmpty {
                    exercises.remove(at: exIndex)
                } else {
                    for (i, var s) in updatedExercise.sets.enumerated() {
                        s.label = "Set \(i + 1)"
                        updatedExercise.sets[i] = s
                    }
                    exercises[exIndex] = updatedExercise
                }
                return
            }
        }
    }
    
    private func addExercise(_ backendExercise: YokuUniffi.Exercise, with backendSet: YokuUniffi.WorkoutSet) {
        let backendID = backendExercise.id()
        if exercises.contains(where: { $0.backendID == backendID }) {
            addSet(backendSet, to: backendExercise)
            return
        }
        
        let setModel = mapSet(backendSet, indexInExercise: 0)
        let exerciseModel = ExerciseModel(
            backendID: backendID,
            name: backendExercise.name(),
            sets: [setModel]
        )

        let index = exercises.firstIndex(where: { $0.name.localizedCaseInsensitiveCompare(exerciseModel.name) == .orderedDescending }) ?? exercises.count
        exercises.insert(exerciseModel, at: index)

        exerciseIDMap[backendID] = exerciseModel.id
    }
    
    private func mapSet(_ backendSet: YokuUniffi.WorkoutSet, indexInExercise: Int) -> ExerciseSetModel {
        let backendID = backendSet.id()
        let uuid = setIDMap[backendID] ?? UUID()
        setIDMap[backendID] = uuid
        
        return ExerciseSetModel(
            id: uuid,
            backendID: backendID,
            label: "Set \(indexInExercise + 1)",
            weight: backendSet.weight(),
            reps: backendSet.reps(),
            rpe: backendSet.rpe()
        )
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

            let setModels = sets.enumerated().map { (index, backendSet) -> ExerciseSetModel in
                let backendSetID = backendSet.id()
                let setUUID = setIDMap[backendSetID] ?? UUID()
                nextSetMap[backendSetID] = setUUID
                let label = "Set \(index + 1)"
                let weight = backendSet.weight()
                let reps = backendSet.reps()
                let rpe = backendSet.rpe()
                return ExerciseSetModel(
                    id: setUUID, backendID: backendSetID, label: label, weight: weight, reps: reps, rpe: rpe)
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

        if models != exercises {
             exercises = models
             reconcileSelectionAfterUpdate()
        }
    }

    func nextSet() {
    }

    var activeExercise: ExerciseModel? {
        guard let id = activeExerciseID else { return nil }
        return exercises.first { $0.id == id }
    }

    var activeWorkoutSessionId: Int64? {
        activeWorkoutSession?.id()
    }

    func indexOfActiveSet(in exercise: ExerciseModel?) -> Int? {
        guard let exercise, let active = activeSetID else { return nil }
        return exercise.sets.firstIndex { $0.id == active }
    }

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

    func startTimer() {
        timerStore.start()
    }
    
    func pauseTimer() {
        timerStore.pause()
    }

    func resumeTimer() {
        timerStore.resume()
    }

    func stopTimer() {
        timerStore.stop()
    }

    var isTimerRunning: Bool {
        timerStore.isRunning
    }
    
    var isTimerPaused: Bool {
        timerStore.isPaused
    }
    
    var workoutStartTime: Date? {
        get { timerStore.startTime }
        set { timerStore.startTime = newValue }
    }

    func dataSeries(for exercise: ExerciseModel?) -> [(Date, Double)] {
        guard let exercise, let backendID = exercise.backendID, let lifts = liftsByExerciseId[backendID] else { return [] }
        return lifts.map { (Date(), $0) }
    }
    
    private func fetchLifts(for exercises: [YokuUniffi.Exercise]) async {
        for ex in exercises {
            if let lifts = try? await backend.getLiftsForExercise(ex.id()) {
                liftsByExerciseId[ex.id()] = lifts
            }
        }
    }

    static var preview: WorkoutStore {
        let store = WorkoutStore()
        return store
    }
}
