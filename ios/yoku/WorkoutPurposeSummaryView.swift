import SwiftUI
import YokuUniffi

struct WorkoutPurposeSummaryView: View {
    @EnvironmentObject var workoutState: Session
    @State private var summary: String = "Analyzing workout…"
    
    private var workoutIntention: String? {
        workoutState.activeWorkoutSession?.intention()
    }

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "sparkles")
                .foregroundStyle(.yellow)
            Text(summary)
                .font(.appBody)
                .lineLimit(1)
                .truncationMode(.tail)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .onAppear { refreshSummary() }
        .onChange(of: workoutState.activeExerciseID) { _, _ in
            refreshSummary()
        }
        .onChange(of: workoutState.exercises) { _, _ in
            refreshSummary()
        }
        .onChange(of: workoutIntention) { _, _ in
            refreshSummary()
        }
    }

    private func refreshSummary() {
        // Use the workout intention if it exists, otherwise fall back to heuristic
        if let workoutSession = workoutState.activeWorkoutSession,
           let intention = workoutSession.intention(),
           !intention.isEmpty {
            summary = intention
            return
        }
        
        // Fallback: infer a purpose from current exercise names
        let names = workoutState.exercises.map { $0.name.lowercased() }

        let isUpper = names.contains { $0.contains("bench") || $0.contains("press") || $0.contains("pull") || $0.contains("row") || $0.contains("dip") }
        let isLower = names.contains { $0.contains("squat") || $0.contains("deadlift") || $0.contains("lunge") || $0.contains("leg") }
        let hasCompounds = names.contains { $0.contains("squat") || $0.contains("deadlift") || $0.contains("bench") || $0.contains("press") }
        let volumeHint = workoutState.exercises.reduce(0) { $0 + $1.sets.count }

        let focus: String
        switch (isUpper, isLower) {
        case (true, true): focus = "full body"
        case (true, false): focus = "upper body"
        case (false, true): focus = "lower body"
        default: focus = "general fitness"
        }

        let goal: String
        if hasCompounds && volumeHint <= 10 {
            goal = "strength-building"
        } else if volumeHint >= 15 {
            goal = "hypertrophy-focused"
        } else {
            goal = "performance-focused"
        }

        summary = "\(goal) \(focus) workout"
    }
}

#Preview {
    WorkoutPurposeSummaryView()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
