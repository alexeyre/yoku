//
//  yokuApp.swift
//  yoku
//
//  Created by Alex Holder on 12/11/2025.
//

import SwiftUI
import YokuUniffi

@main
struct yokuApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .task {
                    let docsDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
                    let dbPath = docsDir.appendingPathComponent("app.db").path
                    await setupDatabase(path: dbPath);
                }
        }
    }
}
