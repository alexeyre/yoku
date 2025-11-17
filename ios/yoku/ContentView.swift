import SwiftUI
import YokuUniffi

// PreferenceKey to measure the input bar height
private struct InputBarHeightPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

struct ContentView: View {
    @EnvironmentObject var session: Session

    // Measured input bar height so we can pad the scrollable content
    @State private var inputBarHeight: CGFloat = 0

    var body: some View {
        VStack(spacing: 0) {
            // Pinned header (top)
            VStack(spacing: 0) {
                InformationHeader()
                    .background(Color(.systemBackground))
                WorkoutPurposeSummaryView()
                    .padding(.horizontal, 12)
                    .padding(.bottom, 6)
                    .background(Color(.systemBackground))
            }

            // Scrollable middle content
            // We pad the bottom by the measured input bar height so content isn’t obscured.
            SetList()
                .environmentObject(session)
                .padding(.bottom, inputBarHeight)
        }
        // Pin the input bar using safeAreaInset so it rides above the keyboard automatically.
        .safeAreaInset(edge: .bottom, spacing: 0) {
            CommandInputBar()
                .environmentObject(session)
                .background(.ultraThinMaterial)
                .background(
                    GeometryReader { proxy in
                        Color.clear
                            .preference(key: InputBarHeightPreferenceKey.self, value: proxy.size.height)
                    }
                )
                .shadow(color: Color.black.opacity(0.08), radius: 8, x: 0, y: -2)
        }
        .onPreferenceChange(InputBarHeightPreferenceKey.self) { inputBarHeight = $0 }
        // Let SwiftUI manage keyboard safe area for us; safeAreaInset handles lifting the bar.
        .ignoresSafeArea(.keyboard, edges: [])
        .environmentObject(session)
    }
}

#Preview {
    ContentView()
        .environmentObject(Session.preview)
}
