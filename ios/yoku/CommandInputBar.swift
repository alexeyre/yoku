import SwiftUI

struct CommandInputBar: View {
    @State private var inputText: String = ""
    @State private var isProcessing: Bool = false
    @EnvironmentObject var session: Session

    var body: some View {
        HStack(spacing: 8) {
            TextField("cmd >", text: $inputText)
                .font(.appBody)
                .textFieldStyle(.plain)
                .onSubmit { runCommand() }
                .disabled(isProcessing)

            if isProcessing {
                SpinnerView()
                    .transition(.opacity)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(Color.black.opacity(0.1))
    }

    @MainActor
    private func runCommand() {
        guard !inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }

        isProcessing = true
        let cmd = inputText

        Task {
            // Use the backend to classify and process the input intelligently
            _ = try? await session.classifyAndProcessInput(input: cmd)
            await MainActor.run {
                inputText = ""
                isProcessing = false
            }
        }
    }
}
