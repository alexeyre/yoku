import SwiftUI
import YokuUniffi

struct ContentView: View {
    var body: some View {
        VStack(spacing: 0) {
            InformationHeader()
                .background(Color(.systemBackground)) // optional: keeps header visible if content scrolls under
            ScrollView {
                VStack(spacing: 16) {
                    
                }
                .padding()
            }
        }
    }
}

#Preview {
    ContentView()
}
