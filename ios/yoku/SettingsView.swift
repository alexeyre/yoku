import SwiftUI

struct SettingsView: View {
    @AppStorage(LoadingSettings.appStorageKey) private var styleRaw: String = LoadingStyle.normal.rawValue

    private var selectionBinding: Binding<LoadingStyle> {
        Binding(
            get: { LoadingStyle(rawValue: styleRaw) ?? .normal },
            set: { styleRaw = $0.rawValue }
        )
    }
    
    var body: some View {
        Form {
            Section("Loading Indicator") {
                Picker("Style", selection: selectionBinding) {
                    ForEach(LoadingStyle.allCases) { style in
                        Text(style.displayName).tag(style)
                    }
                }
                .pickerStyle(.segmented)

                HStack {
                    Text("Preview")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    Spacer()
                    SpinnerView(style: selectionBinding.wrappedValue)
                }
            }
        }
        .navigationTitle("Settings")
    }
}

#Preview {
    NavigationStack {
        SettingsView()
    }
}
