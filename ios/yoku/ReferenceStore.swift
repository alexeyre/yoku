import Foundation
import Combine
import YokuUniffi

@MainActor
final class ReferenceStore: ObservableObject {
    @Published var recentLifts: [Int64: [Double]] = [:]
    @Published var exerciseRecords: [Int64: Double] = [:] // max weight per exercise
    
    private let backend = BackendService.shared
    
    init() {}
    
    func fetchLifts(for exerciseIDs: [Int64]) async {
        for id in exerciseIDs {
            // Check if we already have data? 
            // For now, simple fetch.
            if let lifts = try? await backend.getLiftsForExercise(id) {
                recentLifts[id] = lifts
                if let max = lifts.max() {
                    exerciseRecords[id] = max
                }
            }
        }
    }
    
    // Accessor for charts
    func lifts(for exerciseID: Int64?) -> [Double] {
        guard let id = exerciseID else { return [] }
        return recentLifts[id] ?? []
    }
}

