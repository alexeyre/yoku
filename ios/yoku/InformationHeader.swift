//
//  InformationHeader.swift
//  yoku
//
//  Created by Alex Holder on 13/11/2025.
//

import SwiftUI

struct InformationHeader: View {
    // Read values from the shared WorkoutState
    @EnvironmentObject var workoutState: WorkoutState

    // Statically-provided items (still local)
    let workoutName: String = "FULL BODY A"
    let dateString: String = "2025-11-13 18:42"

    var elapsedString: String {
        let seconds = Int(workoutState.elapsedTime)
        let hrs = seconds / 3600
        let mins = (seconds % 3600) / 60
        let secs = seconds % 60
        return String(format: "%02d:%02d:%02d", hrs, mins, secs)
    }

    var currentExercise: String {
        workoutState.activeExercise?.name ?? "None"
    }

    var totalExercises: Int {
        workoutState.exercises.count
    }

    var totalSets: Int {
        workoutState.exercises.reduce(0) { $0 + $1.sets.count }
    }

    var body: some View {
        VStack(spacing: 4) {
            // First line: workout name and date
            HStack(spacing: 12) {
                labeledValue("WORKOUT", workoutName)
                Spacer(minLength: 0)
                Divider().frame(height: 12).opacity(0.15)
                labeledValue("DATE", dateString, mirrored: true)
            }

            // Second line: elapsed and current exercise
            HStack(spacing: 12) {
                labeledValue("ELAPSED", elapsedString)
                Divider().frame(height: 12).opacity(0.15)
                Spacer(minLength: 0)
                labeledValue("CURRENT", currentExercise, mirrored: true)
            }

            // Third line: totals
            HStack(spacing: 12) {
                labeledValue("EXERCISES", "\(totalExercises)")
                Spacer(minLength: 0)
                Divider().frame(height: 12).opacity(0.15)
                labeledValue("SETS", "\(totalSets)", mirrored: true)
            }
            Divider().frame(height: 12).opacity(0.15)
        }
        .font(.system(.footnote, design: .monospaced))
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .contentShape(Rectangle())
    }

    private func labeledValue(_ label: String, _ value: String, mirrored: Bool = false) -> some View {
        HStack(spacing: 6) {
            if mirrored {
                Text(value)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Text(label)
                    .opacity(0.7)
            } else {
                Text(label)
                    .opacity(0.7)
                Text(value)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
        }
    }
}

#Preview {
    InformationHeader()
        .preferredColorScheme(.dark)
        .environmentObject(WorkoutState())
}
