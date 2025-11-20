import SwiftUI

struct CommandInputBar: View {
    @State private var inputText: String = ""
    @State private var isProcessing: Bool = false
    @EnvironmentObject var workoutStore: WorkoutStore

    var body: some View {
        HStack(alignment: .center, spacing: 6) {
            Text(">")
                .font(.appBody)
                .foregroundStyle(.secondary)
            
            TextField("cmd", text: $inputText)
                .font(.appBody)
                .textFieldStyle(.plain)
                .disabled(isProcessing)
                .onSubmit(runCommand)
            
            if isProcessing {
                SpinnerView()
                .transition(.opacity)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
    }

    @MainActor
    private func runCommand() {
        isProcessing = true
        let cmd = inputText

        Task {
            do {
                try await workoutStore.classifyAndProcessInput(input: cmd)
            } catch {
                print("Error processing command: \(error)")
            }
            await MainActor.run {
                inputText = ""
                isProcessing = false
            }
        }
    }
}
