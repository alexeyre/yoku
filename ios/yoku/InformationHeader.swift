import SwiftUI
import YokuUniffi

struct InformationHeader: View {
    @EnvironmentObject var workoutState: WorkoutStore
    @ObservedObject var timerStore: TimerStore
    
    @Environment(\.dismiss) private var dismiss
    @State private var showCompleteConfirmation = false

    var totalVolume: String {
        let volume = workoutState.exercises.reduce(0.0) { total, exercise in
            let exerciseVolume = exercise.sets.reduce(0.0) { sum, set in
                sum + (set.weight * Double(set.reps))
            }
            return total + exerciseVolume
        }
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
        let date = Date()
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }

    var elapsedString: String {
        let seconds = Int(timerStore.elapsedTime)
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
                        Task {
                            do {
                                try await workoutState.completeWorkout()
                                dismiss()
                            } catch {
                                print("Error completing workout: \(error)")
                            }
                        }
                    } label: {
                        Text("[ STOP ]")
                            .font(.appButton)
                            .foregroundStyle(.red)
                    }
                    .buttonStyle(.plain)

                    Button {
                        if timerStore.isRunning && !timerStore.isPaused {
                            timerStore.pause()
                        } else if timerStore.isPaused {
                            timerStore.resume()
                        }
                    } label: {
                        Text(timerStore.isRunning && !timerStore.isPaused ? "[ PAUSE ]" : "[ RESUME ]")
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
                HStack(spacing: 12) {
                    labeledValue("ELAPSED", elapsedString)
                    Spacer(minLength: 0)
                    Divider().frame(height: 12).opacity(0.15)
                    labeledValue("DATE", dateString, mirrored: true)
                }

                HStack(spacing: 12) {
                    labeledValue("VOLUME", totalVolume)
                    Divider().frame(height: 12).opacity(0.15)
                    Spacer(minLength: 0)
                    labeledValue("CURRENT", currentExercise, mirrored: true)
                }

                HStack(spacing: 12) {
                    labeledValue("EXERCISES", "\(totalExercises)")
                    Spacer(minLength: 0)
                    Divider().frame(height: 12).opacity(0.15)
                    labeledValue("SETS", "\(totalSets)", mirrored: true)
                }

                Rectangle()
                    .fill(Color.primary.opacity(0.08))
                    .frame(height: 0.5)
            }
            .font(.appBody)
            .padding(.horizontal, 12)
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
    InformationHeader(timerStore: TimerStore())
        .preferredColorScheme(.dark)
        .environmentObject(WorkoutStore.preview)
}
