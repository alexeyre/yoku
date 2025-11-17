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

    @StateObject private var session = Session()

    // Keep the last-known db path so we can “restart” by re-running setup.
    @State private var lastDBPath: String?

    var body: some Scene {
        WindowGroup {
            Group {
                if isDatabaseReady {
                    RootView()
                        .environmentObject(session)
                } else if let error = setupError {
                    VStack(spacing: 12) {
                        Text("Failed to set up database")
                            .font(.headline)
                        Text(error.localizedDescription)
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        ProgressView().opacity(0)  // keep layout consistent
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
                // Skip heavy setup when running in Xcode previews
                if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
                    return
                }
                // Build Application Support/yoku/app.db
                do {
                    let _ = setenv("RUST_BACKTRACE", "1", 1)
                    YokuUniffi.setDebugLogLevel()

                    let appSupport = try FileManager.default.url(
                        for: .applicationSupportDirectory,
                        in: .userDomainMask,
                        appropriateFor: nil,
                        create: true
                    )
                    let appDir = appSupport.appendingPathComponent("yoku", isDirectory: true)
                    try FileManager.default.createDirectory(
                        at: appDir, withIntermediateDirectories: true)
                    let dbURL = appDir.appendingPathComponent("app.db")
                    let dbPath = dbURL.path
                    lastDBPath = dbPath

                    try await session.setup(dbPath: dbPath, model: "gpt-5-mini")
                    isDatabaseReady = true
                } catch {
                    setupError = error
                }
            }
        }
    }

    @MainActor
    private func resetViewStateForRestart() {
        // Clear visible state so UI goes back to loading screen
        isDatabaseReady = false
        setupError = nil
    }
}

private struct RootView: View {
    @EnvironmentObject private var session: Session
    @State private var navigateToWorkout = false

    @State private var workoutSessionList: [YokuUniffi.WorkoutSession] = []
    @State private var isLoading = false
    @State private var loadError: Error?

    // Interaction state
    @State private var isPerformingSelection = false
    @State private var selectionError: Error?

    var body: some View {
        NavigationStack {
            VStack {
                // Workouts list
                List {
                    if let error = loadError {
                        Section {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Failed to load workouts")
                                    .font(.headline)
                                Text(error.localizedDescription)
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }
                            .padding(.vertical, 4)
                        }
                    }

                    if isLoading && workoutSessionList.isEmpty {
                        Section {
                            ProgressView("Loading workouts…")
                        }
                    } else if workoutSessionList.isEmpty {
                        Section {
                            Text("No workouts yet")
                                .foregroundStyle(.secondary)
                        }
                    } else {
                        Section {
                            ForEach(workoutSessionList.indices, id: \.self) { i in
                                let ws = workoutSessionList[i]
                                Button {
                                    Task {
                                        await selectExistingWorkoutAndNavigate(ws)
                                    }
                                } label: {
                                    HStack {
                                        VStack(alignment: .leading) {
                                            Text(workoutTitle(from: ws))
                                                .font(.headline)
                                            if let subtitle = workoutSubtitle(from: ws) {
                                                Text(subtitle)
                                                    .font(.subheadline)
                                                    .foregroundStyle(.secondary)
                                            }
                                        }
                                        Spacer()
                                        Text(ws.date())
                                    }
                                }
                                .disabled(isPerformingSelection)
                            }
                        }
                        Button {
                            Task { @MainActor in
                                do {
                                    try await session.resetDatabase()
                                    await loadWorkouts()
                                } catch {
                                    loadError = error
                                }
                            }
                        } label: {
                            Text("Reset database")
                        }
                    }

                    if let selErr = selectionError {
                        Section {
                            Text("Selection failed: \(selErr.localizedDescription)")
                                .font(.footnote)
                                .foregroundStyle(.red)
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .refreshable {
                    // Avoid FFI in previews
                    if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] != "1" {
                        await loadWorkouts()
                    }
                }

                // Start New Workout button
                Button {
                    Task {
                        await createNewWorkoutAndNavigate()
                    }
                } label: {
                    if isPerformingSelection {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Start New Workout")
                            .frame(maxWidth: .infinity)
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(isPerformingSelection)
                .padding()
            }
            .navigationTitle("Workouts")
            .navigationDestination(isPresented: $navigateToWorkout) {
                ContentView()
                    .environmentObject(session)
                    .navigationBarBackButtonHidden(true)
            }
            .task {
                // Avoid FFI in previews
                if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
                    return
                }
                if workoutSessionList.isEmpty {
                    await loadWorkouts()
                }
            }
        }
    }

    // MARK: - Data loading

    @MainActor
    private func loadWorkouts() async {
        isLoading = true
        loadError = nil
        do {
            let sessions = try await session.fetchAllWorkoutSessions()
            workoutSessionList = sessions
        } catch SessionError.backendNotInitialized {
            // Backend session not ready yet
        } catch {
            loadError = error
        }
        isLoading = false
    }

    // MARK: - Selection handlers

    @MainActor
    private func selectExistingWorkoutAndNavigate(_ ws: YokuUniffi.WorkoutSession) async {
        selectionError = nil
        isPerformingSelection = true
        do {
            try await session.setActiveWorkoutSessionId(Int(ws.id()))
            navigateToWorkout = true
        } catch {
            selectionError = error
        }
        isPerformingSelection = false
    }

    @MainActor
    private func createNewWorkoutAndNavigate() async {
        selectionError = nil
        isPerformingSelection = true
        do {
            try await session.createBlankWorkoutSession()
            await loadWorkouts()
            navigateToWorkout = true
        } catch {
            selectionError = error
        }
        isPerformingSelection = false
    }

    // MARK: - Placeholder formatting helpers

    private func workoutTitle(from ws: YokuUniffi.WorkoutSession) -> String {
        return ws.name() ?? "Unnamed workout"
    }

    private func workoutSubtitle(from ws: YokuUniffi.WorkoutSession) -> String? {
        return nil
    }

}

#Preview {
    // Use a preview Session with dummy data and no FFI
    RootView()
        .environmentObject(Session.preview)
}
