import Combine
import Foundation
import SwiftUI

enum GlowTarget: Hashable, CustomStringConvertible {
    case set(id: Int64)
    case exercise(id: Int64)
    case custom(String)
    case uuid(UUID)
    case allSets
    case allExercises
    case pattern(String)
    
    var description: String {
        switch self {
        case .set(let id): return "set:\(id)"
        case .exercise(let id): return "exercise:\(id)"
        case .custom(let str): return "custom:\(str)"
        case .uuid(let id): return "uuid:\(id.uuidString)"
        case .allSets: return "set:*"
        case .allExercises: return "exercise:*"
        case .pattern(let p): return p
        }
    }
}

struct GlowEvent: Identifiable {
    let id = UUID()
    let target: GlowTarget
    let duration: TimeInterval
    let timestamp: Date
    
    init(target: GlowTarget, duration: TimeInterval = 2.5) {
        self.target = target
        self.duration = duration
        self.timestamp = Date()
    }
    
    func matches(_ target: GlowTarget) -> Bool {
        switch (self.target, target) {
        case (.set(let id1), .set(let id2)): return id1 == id2
        case (.exercise(let id1), .exercise(let id2)): return id1 == id2
        case (.uuid(let id1), .uuid(let id2)): return id1 == id2
        case (.custom(let str1), .custom(let str2)): return str1 == str2
        case (.allSets, .set): return true
        case (.allExercises, .exercise): return true
        case (.pattern(let pattern), _): 
            return target.description.matches(pattern: pattern)
        default: return false
        }
    }
}

extension String {
    func matches(pattern: String) -> Bool {
        if pattern.hasSuffix("*") {
            let prefix = String(pattern.dropLast())
            return self.hasPrefix(prefix)
        }
        return self == pattern
    }
}

