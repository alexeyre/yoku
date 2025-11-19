import SwiftUI
import YokuUniffi

struct WorkoutPurposeSummaryView: View {
    @EnvironmentObject var workoutState: Session
    @State private var summary: String = "Analyzing workout…"
    @State private var displayedText: String = ""
    @State private var visible: Bool = true
    @State private var showGlow: Bool = false
    @State private var typingTask: Task<Void, Never>?

    var body: some View {
        if visible {
            HStack(spacing: 6) {
                Text("✨")
                    .font(.appBody)
                    .foregroundStyle(.yellow)
                Text(displayedText)
                    .font(.appBody)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 0)
            }
            .frame(height: 19)
            .padding(.horizontal, 12)
            .padding(.vertical, 0)
            .clipped()
            .transition(.move(edge: .top).combined(with: .opacity))
            .animation(.spring(response: 0.4, dampingFraction: 0.8), value: visible)
            .localGlowEffect(isActive: $showGlow)
            .onAppear { refreshSummary() }
            .onChange(of: workoutState.activeExerciseID) { _, _ in
                refreshSummary()
            }
            .onChange(of: workoutState.exercises) { _, _ in
                refreshSummary()
            }
            .onChange(of: workoutState.intention) { _, _ in
                refreshSummary()
            }
            .onDisappear {
                typingTask?.cancel()
                typingTask = nil
            }
        }
    }

    private func refreshSummary() {
        guard workoutState.activeWorkoutSession != nil else {
            if visible {
                typingTask?.cancel()
                typingTask = nil
                withAnimation(.easeOut(duration: 0.2)) {
                    visible = false
                    showGlow = false
                }
                displayedText = ""
            }
            return
        }
        
        // Check if there's a user-specified intention or cached summary
        if let workoutSession = workoutState.activeWorkoutSession,
           let intention = workoutSession.intention(),
           !intention.isEmpty {
            // Use the intention (either user-specified or LLM-generated cached summary)
            let wasVisible = visible
            summary = intention
            if !wasVisible {
                // Cancel any existing typing task
                typingTask?.cancel()
                displayedText = ""
                
                // Animate view appearance with expansion
                withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                    visible = true
                }
                
                // Start glow effect
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    withAnimation(.easeInOut(duration: 0.3)) {
                        showGlow = true
                    }
                }
                
                // Start typing animation
                typingTask = Task { @MainActor in
                    await typeText(intention)
                }
            } else if displayedText != intention {
                // If already visible but text changed, type it out
                typingTask?.cancel()
                typingTask = Task { @MainActor in
                    await typeText(intention)
                }
            }
        } else {
            // No intention yet, generate LLM summary (which will cache it)
            // Only generate if we have exercises
            if !workoutState.exercises.isEmpty {
                typingTask?.cancel()
                typingTask = Task { @MainActor in
                    do {
                        // Force regenerate when exercises change to get updated summary
                        // This will regenerate if intention is empty, but preserve user-specified intentions
                        let generatedSummary = try await workoutState.getWorkoutSummary(forceRegenerate: true)
                        // Refresh the workout session to get the updated intention
                        try? await workoutState.refreshActiveWorkoutSession()
                        
                        // Update displayed text
                        let wasVisible = visible
                        summary = generatedSummary
                        
                        if !wasVisible {
                            displayedText = ""
                            withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                                visible = true
                            }
                            
                            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                                withAnimation(.easeInOut(duration: 0.3)) {
                                    showGlow = true
                                }
                            }
                            
                            await typeText(generatedSummary)
                        } else if displayedText != generatedSummary {
                            await typeText(generatedSummary)
                        }
                    } catch {
                        // If generation fails, hide the view
                        if visible {
                            withAnimation(.easeOut(duration: 0.2)) {
                                visible = false
                                showGlow = false
                            }
                            displayedText = ""
                        }
                    }
                }
            } else {
                // No exercises yet, hide the view
                if visible {
                    typingTask?.cancel()
                    typingTask = nil
                    withAnimation(.easeOut(duration: 0.2)) {
                        visible = false
                        showGlow = false
                    }
                    displayedText = ""
                }
            }
        }
    }
    
    @MainActor
    private func typeText(_ text: String) async {
        displayedText = ""
        let characters = Array(text)
        
        for (index, character) in characters.enumerated() {
            // Check if task was cancelled
            if Task.isCancelled {
                return
            }
            
            displayedText += String(character)
            
            // Vary typing speed slightly for more natural feel
            // Faster for spaces and punctuation, normal for letters
            let delay: UInt64
            if character == " " {
                delay = 30_000_000 // 0.03 seconds
            } else if ",.!?".contains(character) {
                delay = 80_000_000 // 0.08 seconds
            } else {
                delay = 50_000_000 // 0.05 seconds
            }
            
            try? await Task.sleep(nanoseconds: delay)
        }
        
        // Auto-hide glow after typing completes
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            withAnimation(.easeOut(duration: 0.3)) {
                showGlow = false
            }
        }
    }
}

#Preview {
    WorkoutPurposeSummaryView()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
