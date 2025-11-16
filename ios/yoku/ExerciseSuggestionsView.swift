import SwiftUI

struct ExerciseSuggestion: Identifiable, Hashable {
    let id = UUID()
    let title: String
    let subtitle: String?
    let icon: String
    let accent: Color
}

struct ExerciseSuggestionsView: View {
    @EnvironmentObject var workoutState: Session

    private func suggestions() -> [ExerciseSuggestion] {
        guard let exercise = workoutState.activeExercise else {
            return [
                ExerciseSuggestion(title: "Pick an exercise", subtitle: "Tap one above to get tailored suggestions", icon: "hand.point.up.left.fill", accent: .secondary)
            ]
        }

        let series = workoutState.dataSeries(for: exercise)
        let last = series.last ?? 5
        let avg = series.isEmpty ? last : Int((Double(series.reduce(0, +)) / Double(series.count)).rounded())

        // Very simple heuristics as placeholders
        var items: [ExerciseSuggestion] = [
            ExerciseSuggestion(
                title: "Next set target",
                subtitle: "Aim for \(last) reps again to consolidate",
                icon: "target",
                accent: .accentColor
            ),
            ExerciseSuggestion(
                title: "Progression hint",
                subtitle: avg >= last ? "Consider +2.5 kg next session" : "Hold weight until reps stabilize",
                icon: "arrow.up.right.circle.fill",
                accent: .green
            ),
            ExerciseSuggestion(
                title: "Accessory idea",
                subtitle: accessorySuggestion(for: exercise.name),
                icon: "bolt.fill",
                accent: .orange
            )
        ]

        // If the current exercise has fewer than 3 sets, suggest adding one
        if (exercise.sets.count < 3) {
            items.append(
                ExerciseSuggestion(
                    title: "Add one more set",
                    subtitle: "Keep RIR 1–2 for quality",
                    icon: "plus.app.fill",
                    accent: .blue
                )
            )
        }

        return items
    }

    private func accessorySuggestion(for name: String) -> String {
        let lower = name.lowercased()
        if lower.contains("bench") { return "Add DB Flyes or Triceps Pressdowns" }
        if lower.contains("deadlift") { return "Add Back Extensions or Hamstring Curls" }
        if lower.contains("squat") { return "Add Leg Press or Bulgarian Split Squats" }
        if lower.contains("press") { return "Add Lateral Raises or Face Pulls" }
        if lower.contains("pull") { return "Add Bicep Curls or Face Pulls" }
        return "Add core or mobility work between sets"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "lightbulb.fill")
                    .foregroundStyle(.yellow)
                Text("Suggestions")
                    .font(.system(.caption, design: .monospaced))
                    .opacity(0.8)
                Spacer()
            }

            ForEach(suggestions()) { s in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Image(systemName: s.icon)
                        .foregroundStyle(s.accent)
                        .frame(width: 18)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(s.title)
                            .font(.system(.footnote, design: .monospaced))
                        if let sub = s.subtitle {
                            Text(sub)
                                .font(.system(.caption2, design: .monospaced))
                                .foregroundStyle(.secondary)
                        }
                    }
                    Spacer()
                }
                .padding(.vertical, 6)
                .padding(.horizontal, 10)
                .background(.thinMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color.secondary.opacity(0.08))
        )
    }
}

#Preview {
    ExerciseSuggestionsView()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
