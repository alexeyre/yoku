import SwiftUI
import YokuUniffi

struct ContentView: View {
    @EnvironmentObject var session: Session

    var body: some View {
        VStack(spacing: 0) {
            // Pinned header (top)
            VStack(spacing: 0) {
                InformationHeader()
                    .background(Color(.systemBackground))
                WorkoutPurposeSummaryView()
                    .background(Color(.systemBackground))
            }

            // Scrollable middle content with command bar at bottom
            SetList()
                .environmentObject(session)
        }
        .environmentObject(session)
        .onAppear {
            // Auto-start timer if workout exists and timer not already running
            if session.activeWorkoutSession != nil && !session.isTimerRunning && session.workoutStartTime == nil {
                session.workoutStartTime = Date()
                session.startTimer()
            }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(Session.preview)
}
