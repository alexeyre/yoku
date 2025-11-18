import SwiftUI

struct CommandInputBar: View {
    @State private var inputText: String = ""
    @State private var isProcessing: Bool = false
    @EnvironmentObject var session: Session

    var body: some View {
        ZStack(alignment: .topLeading) {
            // Multiline text input with left padding for ">" indicator
            TextEditor(text: $inputText)
                .font(.appBody)
                .scrollContentBackground(.hidden)
                .background(Color.clear)
                .frame(minHeight: 20)
                .disabled(isProcessing)
                .padding(.leading, 12)
                .onSubmit(runCommand)
            
            // Terminal prompt indicator on first line
            HStack(alignment: .top, spacing: 6) {
                Text(">")
                    .font(.appBody)
                    .foregroundStyle(.secondary)
                    .padding(.top, 8) // Align with TextEditor text baseline
                
                // Placeholder when empty
                if inputText.isEmpty {
                    Text("cmd")
                        .font(.appBody)
                        .foregroundStyle(.secondary.opacity(0.6))
                        .padding(.top, 8)
                        .allowsHitTesting(false)
                }
                
                Spacer()
                
                if isProcessing {
                    SpinnerView()
                        .transition(.opacity)
                        .padding(.top, 4)
                }
            }
            .padding(.leading, 4) // Match TextEditor's internal padding
            .allowsHitTesting(false) // Allow taps to pass through to TextEditor
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
    }

    @MainActor
    private func runCommand() {
        guard !inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }

        isProcessing = true
        let cmd = inputText

        Task {
            // Use the command-style interface to process user input
            // The LLM analyzes the input with full workout context and returns commands to execute
            do {
                try await session.classifyAndProcessInput(input: cmd)
            } catch {
                // Error is stored in session.lastError and can be displayed to user
                print("Error processing command: \(error)")
            }
            await MainActor.run {
                inputText = ""
                isProcessing = false
            }
        }
    }
}
