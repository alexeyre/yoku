import SwiftUI

struct CommandInputBar: View {
    @State private var inputText: String = ""

    private var statusEmoji: String? {
        let trimmed = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        let lower = trimmed.lowercased()
        if lower.contains("add") {
            return "➕"
        } else if lower.contains("modify") {
            return "✏️"
        } else {
            return "❓"
        }
    }

    var body: some View {
        HStack(spacing: 8) {
            TextField("Type a command (e.g. \"add set\", \"modify set\")", text: $inputText)
                .textFieldStyle(.roundedBorder)
                .font(.system(.footnote, design: .monospaced))

            if let emoji = statusEmoji {
                Text(emoji)
                    .font(.system(size: 18))
                    .transition(.opacity)
                    .accessibilityLabel("Command status")
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(.ultraThinMaterial)
    }
}

#Preview {
    VStack {
        Spacer()
        CommandInputBar()
    }
    .preferredColorScheme(.dark)
}
