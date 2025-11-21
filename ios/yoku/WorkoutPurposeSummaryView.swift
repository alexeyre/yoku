import SwiftUI
import YokuUniffi

struct WorkoutPurposeSummaryView: View {
    @EnvironmentObject var workoutState: WorkoutStore
    @State private var summaryMessage: String = "Analyzing workout…"
    @State private var summaryEmoji: String = "✨"
    @State private var displayedText: String = ""
    @State private var visible: Bool = false
    @State private var showGlow: Bool = false
    @State private var typingTask: Task<Void, Never>?
    @State private var isTyping: Bool = false
    
    var body: some View {
        Group {
            if visible {
                HStack(spacing: 6) {
                    Text(summaryEmoji)
                        .font(.appBody)
                        .foregroundStyle(.yellow)
                    TypingLineView(text: displayedText, isTyping: isTyping)
                    Spacer(minLength: 0)
                }
                .frame(height: 19)
                .padding(.horizontal, 12)
                .padding(.vertical, 0)
                .clipped()
                .transition(.move(edge: .top).combined(with: .opacity))
                .animation(.spring(response: 0.4, dampingFraction: 0.8), value: visible)
                .localGlowEffect(isActive: $showGlow)
            } else {
                EmptyView()
            }
        }
        .onAppear { updateFromSummary() }
        .onChange(of: workoutState.workoutSummary) { _, _ in
            updateFromSummary()
        }
        .onDisappear {
            typingTask?.cancel()
            typingTask = nil
        }
    }
    
    private func updateFromSummary() {
        guard workoutState.activeWorkoutSession != nil else {
            if visible {
                typingTask?.cancel()
                typingTask = nil
                withAnimation(.easeOut(duration: 0.2)) {
                    visible = false
                    showGlow = false
                    isTyping = false
                }
                displayedText = ""
            }
            return
        }

        guard !workoutState.exercises.isEmpty else {
            if visible {
                typingTask?.cancel()
                typingTask = nil
                withAnimation(.easeOut(duration: 0.2)) {
                    visible = false
                    showGlow = false
                    isTyping = false
                }
                displayedText = ""
            }
            return
        }

        guard let summaryJson = workoutState.workoutSummary,
              let data = summaryJson.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let message = json["message"] as? String,
              let emoji = json["emoji"] as? String else {
            if visible {
                typingTask?.cancel()
                typingTask = nil
                withAnimation(.easeOut(duration: 0.2)) {
                    visible = false
                    showGlow = false
                    isTyping = false
                }
                displayedText = ""
            }
            return
        }
        
        let trimmedMessage = message.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedEmoji = emoji.trimmingCharacters(in: .whitespacesAndNewlines)

        guard trimmedMessage != summaryMessage || trimmedEmoji != summaryEmoji else {
            return
        }
        
        summaryMessage = trimmedMessage.isEmpty ? "Analyzing workout…" : trimmedMessage
        summaryEmoji = trimmedEmoji.isEmpty ? "✨" : trimmedEmoji
        
        typingTask?.cancel()
        typingTask = Task { @MainActor in
            if !visible {
                displayedText = ""
                withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                    visible = true
                }
                try? await Task.sleep(nanoseconds: 50_000_000)
            }
            await typeText(summaryMessage)
        }
    }

    @MainActor
    private func typeText(_ text: String) async {
        displayedText = ""
        isTyping = true

        withAnimation(.easeInOut(duration: 0.2)) {
            showGlow = true
        }
        
        let characters = Array(text)

        for character in characters {
            if Task.isCancelled {
                withAnimation(.easeOut(duration: 0.2)) {
                    showGlow = false
                    isTyping = false
                }
                return
            }
            displayedText += String(character)
            try? await Task.sleep(nanoseconds: 30_000_000)
        }

        isTyping = false
        withAnimation(.easeOut(duration: 0.3)) {
            showGlow = false
        }
    }
}

struct TypingLineView: View {
    let text: String
    let isTyping: Bool
    @State private var cursorOpacity: Double = 1.0
    
    var body: some View {
        HStack(spacing: 0) {
            Text(text)
                .font(.appBody)
                .lineLimit(1)
                .truncationMode(.tail)
            
            if isTyping {
                Text("|")
                    .font(.appBody)
                    .foregroundStyle(.yellow)
                    .opacity(cursorOpacity)
                    .animation(.easeInOut(duration: 0.5).repeatForever(autoreverses: true), value: cursorOpacity)
                    .onAppear {
                        cursorOpacity = 0.0
                    }
            }
        }
    }
}

#Preview {
    WorkoutPurposeSummaryView()
        .preferredColorScheme(.dark)
        .environmentObject(WorkoutStore.preview)
}
