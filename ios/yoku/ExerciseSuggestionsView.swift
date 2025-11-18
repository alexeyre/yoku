import SwiftUI
import YokuUniffi

struct ExerciseSuggestion: Identifiable, Hashable {
    let id = UUID()
    let title: String
    let subtitle: String?
    let icon: String
    let accent: Color
}

struct ExerciseSuggestionsView: View {
    @EnvironmentObject var workoutState: Session
    @State private var llmSuggestions: [YokuUniffi.WorkoutSuggestion] = []
    @State private var isLoadingSuggestions = false
    @State private var suggestionError: Error?
    
    @State private var isMinimised: Bool = false
    
    private var workoutIntention: String? {
        workoutState.activeWorkoutSession?.intention()
    }


    private func convertLLMSuggestion(_ suggestion: YokuUniffi.WorkoutSuggestion) -> ExerciseSuggestion {
        let (icon, accent) = iconForSuggestionType(suggestion.suggestionType())
        return ExerciseSuggestion(
            title: suggestion.title(),
            subtitle: suggestion.subtitle(),
            icon: icon,
            accent: accent
        )
    }

    private func iconForSuggestionType(_ type: String) -> (String, Color) {
        switch type.lowercased() {
        case "exercise":
            return ("dumbbell.fill", .orange)
        case "progression":
            return ("arrow.up.right.circle.fill", .green)
        case "volume":
            return ("plus.app.fill", .blue)
        case "accessory":
            return ("bolt.fill", .purple)
        case "completion":
            return ("checkmark.circle.fill", .green)
        default:
            return ("lightbulb.fill", .yellow)
        }
    }

    private func allSuggestions() -> [ExerciseSuggestion] {
        // Show suggestions if we have exercises OR if we have an intention set
        let hasExercises = !workoutState.exercises.isEmpty
        let hasIntention = (workoutIntention?.isEmpty == false)
        
        if !hasExercises && !hasIntention {
            return []
        }
        
        var suggestions: [ExerciseSuggestion] = []
        
        // Add LLM suggestions, prioritizing exercise-level recommendations
        let llmConverted = llmSuggestions.map { convertLLMSuggestion($0) }
        suggestions.append(contentsOf: llmConverted)
        
        // If no suggestions at all, show a placeholder
        if suggestions.isEmpty {
            if hasIntention {
                suggestions.append(
                    ExerciseSuggestion(
                        title: "Add exercises to get started",
                        subtitle: "Based on your workout intention",
                        icon: "hand.point.up.left.fill",
                        accent: .secondary
                    )
                )
            } else {
                suggestions.append(
                    ExerciseSuggestion(
                        title: "Pick an exercise",
                        subtitle: "Tap one above to get tailored suggestions",
                        icon: "hand.point.up.left.fill",
                        accent: .secondary
                    )
                )
            }
        }
        
        return suggestions
    }

    private func loadSuggestions() {
        guard !isLoadingSuggestions else { return }
        
        // Skip LLM call if workout is empty AND no intention is set
        let hasExercises = !workoutState.exercises.isEmpty
        let hasIntention = (workoutIntention?.isEmpty == false)
        
        if !hasExercises && !hasIntention {
            llmSuggestions = []
            return
        }
        
        isLoadingSuggestions = true
        suggestionError = nil

        Task {
            do {
                let suggestions = try await workoutState.getWorkoutSuggestions()
                await MainActor.run {
                    self.llmSuggestions = suggestions
                    self.isLoadingSuggestions = false
                }
            } catch {
                await MainActor.run {
                    self.suggestionError = error
                    self.isLoadingSuggestions = false
                    // On error, we'll just use local suggestions
                }
            }
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "lightbulb.fill")
                    .foregroundStyle(.yellow)
                Text("Suggestions")
                    .font(.appCaption)
                    .opacity(0.8)
                Spacer()
                if isLoadingSuggestions {
                    ProgressView()
                        .scaleEffect(0.7)
                }
            }.onTapGesture {
                self.isMinimised = !self.isMinimised
            }
            if !self.isMinimised, !isLoadingSuggestions {
                ForEach(allSuggestions()) { s in
                    HStack(alignment: .firstTextBaseline, spacing: 10) {
                        Image(systemName: s.icon)
                            .foregroundStyle(s.accent)
                            .frame(width: 18)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(s.title)
                                .font(.appBody)
                            if let sub = s.subtitle {
                                Text(sub)
                                    .font(.appCaption2)
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
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color.secondary.opacity(0.08))
        )
        .onAppear {
            loadSuggestions()
        }
        .onChange(of: workoutState.exercises) { _, _ in
            loadSuggestions()
        }
        .onChange(of: workoutState.activeExerciseID) { _, _ in
            loadSuggestions()
        }
        .onChange(of: workoutIntention) { _, _ in
            loadSuggestions()
        }
    }
}

#Preview {
    ExerciseSuggestionsView()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
