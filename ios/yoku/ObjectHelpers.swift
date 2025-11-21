import Foundation
import YokuUniffi

extension YokuUniffi.WorkoutSession: @retroactive Equatable {
    public static func == (lhs: YokuUniffi.WorkoutSession, rhs: YokuUniffi.WorkoutSession) -> Bool {
        return lhs.id() == rhs.id()
    }
}
