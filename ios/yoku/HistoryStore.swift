import Foundation
import YokuUniffi
import SwiftUI
import Combine

@MainActor
final class HistoryStore: ObservableObject {
    @Published var workouts: [YokuUniffi.WorkoutSession] = []
    @Published var isLoading = false
    @Published var error: Error?
    
    private let backend = BackendService.shared
    
    init() {}
    
    // Deprecated: Session is managed by BackendService
    func setSession(_ session: YokuUniffi.Session) {
        // no-op
    }
    
    func fetchWorkouts() async {
        isLoading = true
        error = nil
        do {
            workouts = try await backend.getAllWorkoutSessions()
        } catch {
            self.error = error
        }
        isLoading = false
    }
    
    func deleteWorkout(id: Int64) async {
        do {
            try await backend.deleteWorkoutSession(id: id)
            await fetchWorkouts()
        } catch {
            self.error = error
        }
    }
    
    func resetDatabase() async {
        do {
            try await backend.resetDatabase()
            await fetchWorkouts()
        } catch {
            self.error = error
        }
    }
}

