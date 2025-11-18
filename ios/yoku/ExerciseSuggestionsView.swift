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
