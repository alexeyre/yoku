import SwiftUI
import YokuUniffi

struct ContentView: View {
    @StateObject private var workoutState = WorkoutState()
    var body: some View {
        ZStack {
            // Middle content: keep List as-is, just inset it to avoid header/input bar
            SetList()
                .padding(.top, headerHeight + 22)
                .padding(.bottom, inputBarHeight)

            // Pinned header at top
            VStack(spacing: 0) {
                InformationHeader()
                    .background(Color(.systemBackground))
                Spacer()
            }
            .frame(maxHeight: .infinity, alignment: .top)

            // Pinned input bar at bottom (moves with keyboard)
            VStack(spacing: 0) {
                Spacer()
                CommandInputBar()
                    .background(.ultraThinMaterial)
            }
            .frame(maxHeight: .infinity, alignment: .bottom)
        }
        .environmentObject(workoutState)
    }

    // Tune these constants to match your actual header/input sizes if needed.
    private var headerHeight: CGFloat { 60 }     // approximate InformationHeader height
    private var inputBarHeight: CGFloat { 56 }   // approximate CommandInputBar height including padding
}

#Preview {
    ContentView()
}
