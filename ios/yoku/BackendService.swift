import Foundation
import YokuUniffi

actor BackendService {
    static let shared = BackendService()
    
    private var session: YokuUniffi.Session?
    private var databasePath: String?
    private var model: String?
    
    private init() {}
    
    enum ServiceError: Error {
        case notInitialized
    }
    
    func initialize(dbPath: String, model: String) async throws {
        if let _ = session, self.databasePath == dbPath, self.model == model {
            return
        }
        
        let created = try await YokuUniffi.createSession(dbPath: dbPath, model: model)
        self.session = created
        self.databasePath = dbPath
        self.model = model
    }
    
    func getSession() throws -> YokuUniffi.Session {
        guard let s = session else {
            throw ServiceError.notInitialized
        }
        return s
    }

    func getActiveWorkoutState() async throws -> ActiveWorkoutState {
        let s = try getSession()
        return try await YokuUniffi.getActiveWorkoutState(session: s)
    }
    
    func getAllWorkoutSessions() async throws -> [WorkoutSession] {
        let s = try getSession()
        return try await YokuUniffi.getAllWorkoutSessions(session: s)
    }

    func classifyAndProcessInput(
        input: String,
        selectedSetBackendID: Int64?,
        visibleSetBackendIDs: [Int64]
    ) async throws -> [YokuUniffi.Modification] {
        let s = try getSession()
        return try await YokuUniffi.classifyAndProcessInput(
            session: s,
            input: input,
            selectedSetBackendId: selectedSetBackendID,
            visibleSetBackendIds: visibleSetBackendIDs
        )
    }
    
    func updateWorkoutSet(id: Int64, weight: Double, reps: Int64) async throws -> UpdateWorkoutSetResult {
        let s = try getSession()
        return try await YokuUniffi.updateWorkoutSet(session: s, setId: id, reps: reps, weight: weight)
    }
    
    func deleteWorkoutSet(id: Int64) async throws -> [YokuUniffi.Modification] {
        let s = try getSession()
        return try await YokuUniffi.deleteWorkoutSet(session: s, id: id)
    }
    
    func createBlankWorkoutSession() async throws -> Bool {
        let s = try getSession()
        return try await YokuUniffi.createBlankWorkoutSession(session: s)
    }
    
    func completeWorkoutSession(durationSeconds: Int64) async throws {
        let s = try getSession()
        try await YokuUniffi.completeWorkoutSession(session: s, durationSeconds: durationSeconds)
    }
    
    func getInProgressWorkoutSession() async throws -> YokuUniffi.WorkoutSession? {
        let s = try getSession()
        return try? await YokuUniffi.getInProgressWorkoutSession(session: s)
    }
    
    func updateWorkoutElapsedTime(elapsedSeconds: Int64) async throws {
        let s = try getSession()
        try await YokuUniffi.updateWorkoutElapsedTime(session: s, elapsedSeconds: elapsedSeconds)
    }
    
    func setActiveWorkoutSessionId(_ id: Int64) async throws {
        let s = try getSession()
        try await YokuUniffi.setSessionWorkoutSessionId(session: s, id: id)
    }
    
    func deleteWorkoutSession(id: Int64) async throws {
        let s = try getSession()
        try await YokuUniffi.deleteWorkoutSession(session: s, id: id)
    }
    
    func getLiftsForExercise(_ id: Int64) async throws -> [Double] {
        let s = try getSession()
        return try await YokuUniffi.getLiftsForExercise(session: s, exerciseId: id, limit: 100)
    }
    
    func resetDatabase() async throws {
        let s = try getSession()
        try await YokuUniffi.resetDatabase(session: s)
    }
}

