import SwiftUI

enum LoadingStyle: String, CaseIterable, Identifiable {
    case normal
    case glitchy
    case orbiting
    case arrows

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .normal: return "Normal"
        case .glitchy: return "Glitchy"
        case .orbiting: return "Orbiting"
        case .arrows: return "Arrows"
        }
    }

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
                "⠁", 
                "⠉", 
                "⠈", 
                "⠘", 
                "⠐", 
                "⠰", 
                "⠤", 
                "⠄" 
            ]
            
        case .arrows:
            return [
                "[====]",
                "[>===]",
                "[>>==]",
                "[=>>=]",
                "[==>>]",
                "[===>]",
                "[====]",
                "[===<]",
                "[==<<]",
                "[=<<=]",
                "[<<==]",
                "[<===]",
            ]

        case .glitchy:
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
    var loadingStyle: LoadingStyle {
        get { self[LoadingStyleKey.self] }
        set { self[LoadingStyleKey.self] = newValue }
    }

    private struct LoadingStyleKey: EnvironmentKey {
        static let defaultValue: LoadingStyle = LoadingStyle(rawValue: UserDefaults.standard.string(forKey: LoadingSettings.appStorageKey) ?? "") ?? .arrows
    }
}
