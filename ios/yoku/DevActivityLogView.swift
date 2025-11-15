import Combine
import SwiftUI
import YokuUniffi

struct LogEntry: Identifiable, Hashable {
    enum Source: String {
        case frontend = "FE"
        case backend = "BE"
    }
    let id = UUID()
    let source: Source
    let message: String
    let timestamp: Date = Date()
}

final class LogCenter: ObservableObject {
    @Published var entries: [LogEntry] = []

    // Track the last backend log index we've pulled
    private var backendLastIndex: Int32 = 0

    // Poll cancellable when using Combine timers from outside
    private var pollCancellable: AnyCancellable?

    func post(_ entry: LogEntry) {
        entries.append(entry)
        // Keep it light
        if entries.count > 200 {
            entries.removeFirst(entries.count - 200)
        }
    }

    // Convenience
    func fe(_ message: String) { post(LogEntry(source: .frontend, message: message)) }
    func be(_ message: String) { post(LogEntry(source: .backend, message: message)) }

    // Start periodic polling of backend logs. Interval is in seconds.
    func startBackendPolling(interval: TimeInterval = 1.0) {
        // Avoid starting multiple pollers
        stopBackendPolling()

        pollCancellable = Timer.publish(every: interval, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                self?.pollOnce()
            }
    }

    func stopBackendPolling() {
        pollCancellable?.cancel()
        pollCancellable = nil
    }

    private func pollOnce() {
        // Call into the uniffi-exported Rust functions to fetch new backend logs.
        // Do the blocking call off the main thread and push results back to main.
        DispatchQueue.global(qos: .background).async { [weak self] in
            guard let self = self else { return }

            // `backend_logs_count()` and `backend_logs_since(_:)` are synchronous
            // uniffi bindings exported from the Rust library.
            let total = Int(YokuUniffi.backendLogsCount())
            if Int(self.backendLastIndex) >= total {
                return
            }

            // Fetch new logs since last index
            let newLogs = YokuUniffi.backendLogsSince(startIndex: self.backendLastIndex)

            if newLogs.isEmpty { return }

            DispatchQueue.main.async {
                for message in newLogs {
                    self.be(message)
                }
                self.backendLastIndex += Int32(newLogs.count)
            }
        }
    }
}

// Global/shared LogCenter instance accessible across the app
let sharedLogCenter = LogCenter()

// Global helper to post a front-end log message from anywhere in Swift
func postFrontendLog(_ message: String) {
    DispatchQueue.main.async {
        sharedLogCenter.fe(message)
    }
}

struct DevActivityLogView: View {
    @ObservedObject var logCenter: LogCenter

    // We keep a short timer for UI demo pulses. The backend polling is handled
    // by the LogCenter's internal poller started in onAppear.
    private let uiTimer = Timer.publish(every: 3.0, on: .main, in: .common).autoconnect()

    private func line(for entry: LogEntry) -> some View {
        // Console-style single line: [HH:mm:ss] [FE/BE] message
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text("[\(timestampString(entry.timestamp))]")
                .foregroundStyle(.secondary)
            Text("[\(entry.source.rawValue)]")
                .foregroundStyle(entry.source == .frontend ? .blue : .purple)
            Text(entry.message)
                .foregroundStyle(.primary)
            Spacer(minLength: 0)
        }
        .font(.system(.footnote, design: .monospaced))
        .textSelection(.enabled)
        .padding(.vertical, 2)
        .padding(.horizontal, 8)
        .background(Color.clear)
    }

    private func timestampString(_ date: Date) -> String {
        let f = DateFormatter()
        f.dateFormat = "HH:mm:ss"
        return f.string(from: date)
    }

    var body: some View {
        // No header; just the console
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(logCenter.entries) { entry in
                        line(for: entry)
                            .id(entry.id)
                        Divider()
                            .opacity(0.15)
                    }
                }
                .padding(.vertical, 6)
                .background(Color.black.opacity(0.08))  // subtle console tint that works in light/dark
            }
            .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(Color.secondary.opacity(0.2), lineWidth: 1)
            )
            .onChange(of: logCenter.entries.count) { _, _ in
                if let last = logCenter.entries.last {
                    withAnimation(.easeOut(duration: 0.25)) {
                        proxy.scrollTo(last.id, anchor: .bottom)
                    }
                }
            }
        }
        .padding(.horizontal, 0)
        .padding(.vertical, 0)
        .onAppear {
            // Start polling backend logs when the view appears
            logCenter.startBackendPolling()
        }
        .onDisappear {
            // Stop polling when the view disappears
            logCenter.stopBackendPolling()
        }
    }
}

#Preview {
    DevActivityLogView(logCenter: LogCenter())
        .preferredColorScheme(.dark)
}
