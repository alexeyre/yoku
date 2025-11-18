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
                            .font(.appBody)
                        Text(error.localizedDescription)
                            .font(.appCaption2)
                            .foregroundStyle(.secondary)
                        ProgressView().opacity(0)  // keep layout consistent
                    }
                    .padding()
                } else {
                    // Lightweight splash/loading while DB initializes
                    VStack(spacing: 12) {
                        ProgressView("Preparing database…")
                        Text("Please wait")
                            .font(.appBody)
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

    // Settings navigation
    @State private var showSettings = false

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Header with buttons
                HStack(spacing: 16) {
                    Button {
                        Task {
                            await createNewWorkoutAndNavigate()
                        }
                    } label: {
                        if isPerformingSelection {
                            ProgressView()
                                .scaleEffect(0.8)
                        } else {
                        Text("[ NEW ]")
                            .font(.appButton)
                                .foregroundStyle(.primary)
                        }
                    }
                    .buttonStyle(.plain)
                    .disabled(isPerformingSelection)

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
                        Text("[ RESET ]")
                            .font(.appButton)
                            .foregroundStyle(.red)
                    }
                    .buttonStyle(.plain)

                    Spacer()

                    Button {
                        showSettings = true
                    } label: {
                        Text("[ SETTINGS ]")
                            .font(.appButton)
                            .foregroundStyle(.primary)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Settings")
                }
                .padding(.horizontal, 12)
                .padding(.top, 6)
                .padding(.bottom, 4)

                // Minimal divider
                Rectangle()
                    .fill(Color.primary.opacity(0.08))
                    .frame(height: 0.5)

                // Workouts list - minimal styling
                List {
                    if let error = loadError {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("ERROR: Failed to load workouts")
                                .font(.appBody)
                            Text(error.localizedDescription)
                                .font(.appCaption2)
                                .foregroundStyle(.secondary)
                        }
                        .padding(.vertical, 4)
                        .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                        .listRowSeparator(.hidden)
                        .listRowBackground(Color.clear)
                    }

                    if isLoading && workoutSessionList.isEmpty {
                        HStack(spacing: 8) {
                            ProgressView()
                                .scaleEffect(0.8)
                            Text("Loading…")
                                .font(.appBody)
                        }
                        .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                        .listRowSeparator(.hidden)
                        .listRowBackground(Color.clear)
                    } else if workoutSessionList.isEmpty {
                        Text("No workouts")
                            .font(.appBody)
                            .foregroundStyle(.secondary)
                            .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                            .listRowSeparator(.hidden)
                            .listRowBackground(Color.clear)
                    } else {
                        ForEach(workoutSessionList.indices, id: \.self) { i in
                            let ws = workoutSessionList[i]
                            Button {
                                Task {
                                    await selectExistingWorkoutAndNavigate(ws)
                                }
                            } label: {
                                HStack(spacing: 6) {
                                    Text(workoutTitle(from: ws))
                                        .font(.appBody)
                                        .lineLimit(1)
                                    Spacer()
                                    Text(ws.date())
                                        .font(.appCaption2)
                                        .foregroundStyle(.secondary)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.vertical, 2)
                            }
                            .buttonStyle(.plain)
                            .disabled(isPerformingSelection)
                            .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                            .listRowSeparator(.hidden)
                            .listRowBackground(Color.clear)
                            .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                                Button(role: .destructive) {
                                    deleteWorkout(ws)
                                } label: {
                                    Text("DEL")
                                        .font(.appCaption)
                                }
                                .tint(.red.opacity(0.8))
                            }
                        }
                    }

                    if let selErr = selectionError {
                        Text("ERROR: \(selErr.localizedDescription)")
                            .font(.appBody)
                            .foregroundStyle(.red)
                            .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                            .listRowSeparator(.hidden)
                            .listRowBackground(Color.clear)
                    }
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
                .refreshable {
                    // Avoid FFI in previews
                    if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] != "1" {
                        await loadWorkouts()
                    }
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    Text("WORKOUTS")
                        .font(.appBody)
                }
            }
            .sheet(isPresented: $showSettings) {
                NavigationStack {
                    SettingsView()
                }
            }
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
            try await session.setActiveWorkoutSessionId(ws.id())
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

    // MARK: - Delete handler

    @MainActor
    private func deleteWorkouts(at offsets: IndexSet) {
        Task {
            for index in offsets {
                guard index < workoutSessionList.count else { continue }
                let workout = workoutSessionList[index]
                do {
                    _ = try await session.deleteWorkoutSession(id: workout.id())
                } catch {
                    loadError = error
                }
            }
            // Reload the list once after all deletions to reflect current state
            await loadWorkouts()
        }
    }
    
    // Helper for swipe delete action
    @MainActor
    private func deleteWorkout(_ workout: YokuUniffi.WorkoutSession) {
        Task {
            do {
                _ = try await session.deleteWorkoutSession(id: workout.id())
                await loadWorkouts()
            } catch {
                loadError = error
            }
        }
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
