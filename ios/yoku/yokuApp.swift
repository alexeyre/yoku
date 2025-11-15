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
    @State private var isDatabaseReady = false
    @State private var setupError: Error?

    var body: some Scene {
        WindowGroup {
            Group {
                if isDatabaseReady {
                    ContentView()
                } else if let error = setupError {
                    VStack(spacing: 12) {
                        Text("Failed to set up database")
                            .font(.headline)
                        Text(error.localizedDescription)
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        ProgressView().opacity(0) // keep layout consistent
                    }
                    .padding()
                } else {
                    // Lightweight splash/loading while DB initializes
                    VStack(spacing: 12) {
                        ProgressView("Preparing database…")
                        Text("Please wait")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                    .padding()
                }
            }
            .task {
                // Build Application Support/yoku/app.db
                do {
                    let appSupport = try FileManager.default.url(
                        for: .applicationSupportDirectory,
                        in: .userDomainMask,
                        appropriateFor: nil,
                        create: true
                    )
                    let appDir = appSupport.appendingPathComponent("yoku", isDirectory: true)
                    try FileManager.default.createDirectory(at: appDir, withIntermediateDirectories: true)
                    let dbURL = appDir.appendingPathComponent("app.db")
                    let dbPath = dbURL.path

                    // Initialize the database before showing the main UI
                    await try setupDatabase(path: dbPath)

                    // Mark ready
                    isDatabaseReady = true
                } catch {
                    setupError = error
                }
            }
        }
    }
}
