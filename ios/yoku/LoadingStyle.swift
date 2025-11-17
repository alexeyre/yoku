import SwiftUI

enum LoadingStyle: String, CaseIterable, Identifiable {
    case normal
    case glitchy
    case orbiting

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .normal: return "Normal"
        case .glitchy: return "Glitchy"
        case .orbiting: return "Orbiting"
        }
    }

    // Frames for each style
    var frames: [String] {
        switch self {
        case .normal:
            return ["[   ]",
                    "[.  ]",
                    "[.. ]",
                    "[...]",
                    "[ ..]",
                    "[  .]",
                    "[   ]",
                    "[  .]",
                    "[ ..]",
                    "[...]",
                    "[.. ]",
                    "[.  ]"]
        case .orbiting:
            return [
                "⠁", // top
                "⠉", // top-right
                "⠈", // right
                "⠘", // bottom-right
                "⠐", // bottom
                "⠰", // bottom-left
                "⠤", // left
                "⠄"  // top-left
            ]

        case .glitchy:
            // Intentionally janky/glitchy feel: irregular shapes and order
            return ["[   ]",
                    "[¿  ]",
                    "[ ▒ ]",
                    "[░░ ]",
                    "[▓▓▓]",
                    "[░░ ]",
                    "[ ▒ ]",
                    "[¿  ]"]

        }
    }
}

struct LoadingSettings {
    static let appStorageKey = "LoadingStyleSelection"

    @AppStorage(appStorageKey) var styleRaw: String = LoadingStyle.normal.rawValue

    var style: LoadingStyle {
        get { LoadingStyle(rawValue: styleRaw) ?? .normal }
        nonmutating set { styleRaw = newValue.rawValue }
    }
}

extension EnvironmentValues {
    // Optional: allow injecting a style via environment if desired in the future
    var loadingStyle: LoadingStyle {
        get { self[LoadingStyleKey.self] }
        set { self[LoadingStyleKey.self] = newValue }
    }

    private struct LoadingStyleKey: EnvironmentKey {
        static let defaultValue: LoadingStyle = LoadingStyle(rawValue: UserDefaults.standard.string(forKey: LoadingSettings.appStorageKey) ?? "") ?? .normal
    }
}
