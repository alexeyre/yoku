import SwiftUI

struct SpinnerView: View {
    var style: LoadingStyle?

    @AppStorage(LoadingSettings.appStorageKey) private var storedStyleRaw: String = LoadingStyle.normal.rawValue

    @State private var frameIndex: Int = 0
    @State private var timer: Timer? = nil

    private var activeStyle: LoadingStyle {
        style ?? LoadingStyle(rawValue: storedStyleRaw) ?? .normal
    }

    private var frames: [String] {
        let f = activeStyle.frames
        return f.isEmpty ? [""] : f
    }

    private var text: String {
        frames[frameIndex % frames.count]
    }

    var body: some View {
        Text(text)
            .font(.appBody)
            .foregroundColor(.secondary)
            .onAppear { start() }
            .onDisappear { stop() }
            .accessibilityLabel("Loading")
    }

    @MainActor
    private func start() {
        guard timer == nil else { return }
        timer = Timer.scheduledTimer(withTimeInterval: 0.12, repeats: true) { _ in
            frameIndex = (frameIndex + 1) % max(1, frames.count)
        }
        if let timer {
            RunLoop.main.add(timer, forMode: .common)
        }
    }

    @MainActor
    private func stop() {
        timer?.invalidate()
        timer = nil
        frameIndex = 0
    }
}


#Preview {
    VStack(alignment: .leading, spacing: 12) {
        ForEach(LoadingStyle.allCases) { style in
            HStack(spacing: 12) {
                Text(style.displayName)
                    .font(.appCaption)
                    .frame(width: 80, alignment: .leading)
                SpinnerView(style: style)
            }
        }
        .padding(.vertical, 8)
    }
    .padding()
    .previewLayout(.sizeThatFits)
}
