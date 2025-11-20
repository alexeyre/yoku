import SwiftUI
import YokuUniffi

struct ContentView: View {
    @EnvironmentObject var workoutStore: WorkoutStore

    var body: some View {
        VStack(spacing: 0) {
            // Pinned header (top)
            VStack(spacing: 0) {
                InformationHeader(timerStore: workoutStore.timerStore)
                    .background(Color(.systemBackground))
                WorkoutPurposeSummaryView()
                    .background(Color(.systemBackground))
            }

            // Scrollable middle content with command bar at bottom
            SetList()
                .environmentObject(workoutStore)
        }
        .environmentObject(workoutStore)
        .onAppear {
            // Auto-start timer if workout exists and timer not already running
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
