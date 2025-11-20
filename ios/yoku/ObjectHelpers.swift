//
//  ObjectHelpers.swift
//  yoku
//
//  Created by Alex Holder on 20/11/2025.
//

import Foundation
import YokuUniffi

extension YokuUniffi.WorkoutSession: @retroactive Equatable {
    public static func == (lhs: YokuUniffi.WorkoutSession, rhs: YokuUniffi.WorkoutSession) -> Bool {
        return lhs.id() == rhs.id()
    }
}
