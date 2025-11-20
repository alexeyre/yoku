//
//  InformationHeader.swift
//  yoku
//
//  Created by Alex Holder on 13/11/2025.
//

import SwiftUI
import YokuUniffi

struct InformationHeader: View {
    // Read values from the shared WorkoutState
    @EnvironmentObject var workoutState: Session
    
    @Environment(\.dismiss) private var dismiss
    @State private var showCompleteConfirmation = false

    var totalVolume: String {
        let volume = workoutState.exercises.reduce(0.0) { total, exercise in
            let exerciseVolume = exercise.sets.reduce(0.0) { sum, set in
                sum + (set.weight * Double(set.reps))
            }
            return total + exerciseVolume
        }
        // Format as kg with one decimal place, or show "0 kg" if no volume
        if volume > 0 {
            return String(format: "%.1f kg", volume)
        } else {
            return "0 kg"
        }
    }
    
    var dateString: String {
        if let workout = workoutState.activeWorkoutSession {
            return workout.date()
        }
        // Fallback to today's date if no workout
        let date = Date()
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }

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
        VStack {
                HStack(spacing: 16) {
                    Button {
                        showCompleteConfirmation = true
                    } label: {
                        Text("[ STOP ]")
                            .font(.appButton)
                            .foregroundStyle(.red)
                    }
                    .buttonStyle(.plain)
                    .confirmationDialog(
                        "Complete Workout?",
                        isPresented: $showCompleteConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Complete", role: .destructive) {
                            Task {
                                do {
                                    try await workoutState.completeWorkout()
                                    dismiss()
                                } catch {
                                    // Handle error - could show alert
                                    print("Error completing workout: \(error)")
                                }
                            }
                        }
                        Button("Cancel", role: .cancel) {}
                    }

                    Button {
                        if workoutState.isTimerRunning && !workoutState.isTimerPaused {
                            workoutState.pauseTimer()
                        } else if workoutState.isTimerPaused {
                            workoutState.resumeTimer()
                        }
                    } label: {
                        Text(workoutState.isTimerRunning && !workoutState.isTimerPaused ? "[ PAUSE ]" : "[ RESUME ]")
                            .font(.appButton)
                            .foregroundStyle(.primary)
                    }
                    .buttonStyle(.plain)

                    Spacer()

                    Button {
                        workoutState.nextSet()
                    } label: {
                        Text("[ NEXT ]")
                            .font(.appButton)
                            .foregroundStyle(.primary)
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 12)
                .padding(.top, 6)
            VStack(spacing: 4) {
                // First line: elapsed time and date
                HStack(spacing: 12) {
                    labeledValue("ELAPSED", elapsedString)
                    Spacer(minLength: 0)
                    Divider().frame(height: 12).opacity(0.15)
                    labeledValue("DATE", dateString, mirrored: true)
                }
                
                // Second line: total volume and current exercise
                HStack(spacing: 12) {
                    labeledValue("VOLUME", totalVolume)
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
                
                // Optional: a very subtle hairline instead of a full divider
                Rectangle()
                    .fill(Color.primary.opacity(0.08))
                    .frame(height: 0.5)
            }
            .font(.appBody)
            .padding(.horizontal, 12)
            // Reduced bottom padding to pull the summary closer
            .padding(.top, 6)
            .padding(.bottom, 2)
            .contentShape(Rectangle())
        }
    }

    private func labeledValue(_ label: String, _ value: String, mirrored: Bool = false) -> some View {
        HStack(spacing: 6) {
            if mirrored {
                Text(value)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    //.animation(.easeInOut(duration: 0.2), value: value)
                Text(label)
                    .opacity(0.7)
            } else {
                Text(label)
                    .opacity(0.7)
                Text(value)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    //.animation(.easeInOut(duration: 0.2), value: value)
            }
        }
    }
}

#Preview {
    InformationHeader()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
