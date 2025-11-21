import SwiftUI
import YokuUniffi

struct ContentView: View {
    @EnvironmentObject var workoutStore: WorkoutStore

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                InformationHeader(timerStore: workoutStore.timerStore)
                    .background(Color(.systemBackground))
                WorkoutPurposeSummaryView()
                    .background(Color(.systemBackground))
            }

            SetList()
                .environmentObject(workoutStore)
        }
        .environmentObject(workoutStore)
        .onAppear {
            if workoutStore.activeWorkoutSession != nil && !workoutStore.isTimerRunning && workoutStore.workoutStartTime == nil {
                workoutStore.workoutStartTime = Date()
                workoutStore.startTimer()
            }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(WorkoutStore.preview)
}
