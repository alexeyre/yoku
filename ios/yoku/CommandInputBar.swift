import SwiftUI

struct CommandInputBar: View {
    @State private var inputText: String = ""
    @State private var isProcessing: Bool = false
    @EnvironmentObject var session: Session

    private var statusEmoji: String? {
        guard !inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        let lower = inputText.lowercased()
        if lower.contains("add") { return "➕" }
        if lower.contains("modify") { return "✏️" }
        return "❓"
    }

    var body: some View {
        HStack(spacing: 8) {
            TextField("cmd >", text: $inputText)
                .font(.system(.footnote, design: .monospaced))
                .textFieldStyle(.plain)
                .onSubmit { runCommand() }
                .disabled(isProcessing)

            if isProcessing {
                SpinnerView()
                    .transition(.opacity)
            } else if let emoji = statusEmoji {
                Text(emoji)
                    .font(.system(size: 16))
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
            _ = try? await session.addSetFromString(input: cmd)
            await MainActor.run {
                inputText = ""
                isProcessing = false
            }
        }
    }
}
