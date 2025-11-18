//
//  AppFont.swift
//  yoku
//
//  Centralized font configuration for the app
//

import SwiftUI

extension Font {
    // MARK: - Font Configuration
    // 
    // To change the font app-wide, update `customFontName` below.
    // Make sure the font file is:
    // 1. Added to the Xcode project
    // 2. Listed in Info.plist under UIAppFonts
    // 3. The name matches exactly (check with Font Book or UIFont.familyNames)
    //
    private static let customFontName = "Fira Code"
    
    // Fallback to system monospaced if custom font fails to load
    private static func customFont(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        // Check if font is available (works for both static and variable fonts)
        if UIFont(name: customFontName, size: size) != nil {
            return .custom(customFontName, size: size)
        } else {
            // Fallback to system monospaced if custom font not found
            return .system(size: size, weight: weight, design: .monospaced)
        }
    }
    
    // MARK: - App Font Styles
    
    /// Standard body text (terminal-style)
    static var appBody: Font {
        customFont(size: 14)
    }
    
    /// Caption text (smaller)
    static var appCaption: Font {
        customFont(size: 12)
    }
    
    /// Extra small caption
    static var appCaption2: Font {
        customFont(size: 11)
    }
    
    /// Small icon/chevron size
    static var appIcon: Font {
        customFont(size: 10, weight: .semibold)
    }
    
    /// Chart labels
    static var appChart: Font {
        customFont(size: 11)
    }
    
    /// Button text
    static var appButton: Font {
        customFont(size: 14)
    }
}

