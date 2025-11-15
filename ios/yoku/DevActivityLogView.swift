import SwiftUI
import Combine

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
}

struct DevActivityLogView: View {
    @ObservedObject var logCenter: LogCenter

    private let timer = Timer.publish(every: 3.0, on: .main, in: .common).autoconnect()

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
                .background(Color.black.opacity(0.08)) // subtle console tint that works in light/dark
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
        .onReceive(timer) { _ in
            // Demo pulses if no real logs yet
            if logCenter.entries.isEmpty {
                logCenter.fe("UI ready. Awaiting commands…")
                logCenter.be("Database connected.")
            }
        }
    }
}

#Preview {
    DevActivityLogView(logCenter: LogCenter())
        .preferredColorScheme(.dark)
}
