import SwiftUI
import YokuUniffi

struct ContentView: View {
    @EnvironmentObject var session: Session

    var body: some View {
        VStack(spacing: 0) {
            // Pinned header (top)
            VStack(spacing: 0) {
                InformationHeader()
                    .background(Color(.systemBackground))
                WorkoutPurposeSummaryView()
                    .background(Color(.systemBackground))
            }

            // Scrollable middle content with command bar at bottom
            SetList()
                .environmentObject(session)
        }
        .environmentObject(session)
    }
}

#Preview {
    ContentView()
        .environmentObject(Session.preview)
}
