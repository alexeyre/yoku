import SwiftUI

extension Font {
    private static let customFontName = "Fira Code"
    
    private static func customFont(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        if UIFont(name: customFontName, size: size) != nil {
            return .custom(customFontName, size: size)
        } else {
            return .system(size: size, weight: weight, design: .monospaced)
        }
    }
    
    static var appBody: Font {
        customFont(size: 14)
    }
    
    static var appCaption: Font {
        customFont(size: 12)
    }
    
    static var appCaption2: Font {
        customFont(size: 11)
    }
    
    static var appIcon: Font {
        customFont(size: 10, weight: .semibold)
    }
    
    static var appChart: Font {
        customFont(size: 11)
    }
    
    static var appButton: Font {
        customFont(size: 14)
    }
}

