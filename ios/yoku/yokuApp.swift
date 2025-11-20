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

    @StateObject private var workoutStore = WorkoutStore()
    @StateObject private var historyStore = HistoryStore()
    @StateObject private var referenceStore = ReferenceStore()

    // Keep the last-known db path so we can “restart” by re-running setup.
    @State private var lastDBPath: String?

    var body: some Scene {
        WindowGroup {
            Group {
                if isDatabaseReady {
                    RootView()
                        .environmentObject(workoutStore)
                        .environmentObject(historyStore)
                        .environmentObject(referenceStore)
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
                    Button {
                        Task { @MainActor in
                            do {
                                // Re-init logic if needed, or just retry setup
                                if let path = lastDBPath {
                                    try await workoutStore.setup(dbPath: path, model: "gpt-5-mini")
                                    isDatabaseReady = true
                                }
                            } catch {
                                setupError = error
                            }
                        }
                    } label: {
                        Text("[ RESET ]")
                            .font(.appButton)
                            .foregroundStyle(.red)
                    }
                    .buttonStyle(.plain)
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

                    try await workoutStore.setup(dbPath: dbPath, model: "gpt-5-mini")
                    // Pass session to history store (handled by singleton now)
                    isDatabaseReady = true
                } catch {
                    setupError = error
                }
            }
        }
    }
}

private struct RootView: View {
    @EnvironmentObject private var workoutStore: WorkoutStore
    @EnvironmentObject private var historyStore: HistoryStore
    @State private var navigateToWorkout = false

    @State private var isLoading = false
    @State private var loadError: Error?

    // Interaction state
    @State private var isPerformingSelection = false
    @State private var selectionError: Error?
    @State private var showWorkoutOverwriteWarning = false

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
                            await historyStore.resetDatabase()
                            // Also need to reset workout store state if active
                            try? await workoutStore.setup(dbPath: "", model: "") // This is hacky, maybe add reset to workoutStore
                            // Actually workoutStore.setup handles reset implicitly if we just call it? No.
                            // Reset logic was: call backend reset.
                            // historyStore.resetDatabase calls backend reset.
                            // We should reload workouts.
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

                // Workouts list - terminal styling
                List {
                    if let error = historyStore.error {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("ERROR: Failed to load workouts")
                                .font(.appBody)
                            Text(error.localizedDescription)
                                .font(.appBody)
                                .foregroundStyle(.secondary)
                        }
                        .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                        .listRowSeparator(.hidden)
                        .listRowBackground(Color.clear)
                    }

                    if historyStore.isLoading && historyStore.workouts.isEmpty {
                        HStack(spacing: 6) {
                            ProgressView()
                                .scaleEffect(0.8)
                            Text("Loading…")
                                .font(.appBody)
                        }
                        .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                        .listRowSeparator(.hidden)
                        .listRowBackground(Color.clear)
                    } else if historyStore.workouts.isEmpty {
                        Text("No workouts")
                            .font(.appBody)
                            .foregroundStyle(.secondary)
                            .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
                            .listRowSeparator(.hidden)
                            .listRowBackground(Color.clear)
                    } else {
                        ForEach(historyStore.workouts.indices, id: \.self) { i in
                            let ws = historyStore.workouts[i]
                            Button {
                                Task {
                                    await selectExistingWorkoutAndNavigate(ws)
                                }
                            } label: {
                                HStack(spacing: 6) {
                                    Text(workoutTitle(from: ws))
                                        .font(.appBody)
                                        .lineLimit(1)
                                    Spacer(minLength: 0)
                                    Text(ws.date())
                                        .font(.appBody)
                                        .foregroundStyle(.secondary)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
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
                                        .font(.appBody)
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
                .environment(\.defaultMinListRowHeight, 0)
                .font(.appBody)
                .refreshable {
                    // Avoid FFI in previews
                    if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] != "1" {
                        await historyStore.fetchWorkouts()
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
            .alert("Workout Overwritten", isPresented: $showWorkoutOverwriteWarning) {
                Button("OK", role: .cancel) {}
            } message: {
                Text("A workout was already in progress. It has been completed and saved.")
            }
            .navigationDestination(isPresented: $navigateToWorkout) {
                ContentView()
                    .environmentObject(workoutStore)
                    .navigationBarBackButtonHidden(true)
            }
            .task {
                if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
                    return
                }
                await historyStore.fetchWorkouts()
                // Auto-navigate to in-progress workout if it exists
                if workoutStore.activeWorkoutSession != nil {
                    navigateToWorkout = true
                }
            }
            .onChange(of: workoutStore.activeWorkoutSession) { oldValue, newValue in
                // Auto-navigate when an in-progress workout is detected (but not if we're already navigating)
                if newValue != nil && oldValue == nil && !navigateToWorkout {
                    // Use a small delay to ensure this happens after initial setup
                    Task { @MainActor in
                        try? await Task.sleep(nanoseconds: 50_000_000) // 0.05 seconds
                        if !navigateToWorkout {
                            navigateToWorkout = true
                        }
                    }
                }
            }
        }
    }
    
    // MARK: - Selection handlers

    @MainActor
    private func selectExistingWorkoutAndNavigate(_ ws: YokuUniffi.WorkoutSession) async {
        selectionError = nil
        isPerformingSelection = true
        do {
            try await workoutStore.setActiveWorkoutSessionId(ws.id())
            // Navigate to workout view
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
            let hadExisting = try await workoutStore.createBlankWorkoutSession()
            await historyStore.fetchWorkouts()
            if hadExisting {
                showWorkoutOverwriteWarning = true
            }
            navigateToWorkout = true
        } catch {
            selectionError = error
        }
        isPerformingSelection = false
    }
    
    // Helper for swipe delete action
    @MainActor
    private func deleteWorkout(_ workout: YokuUniffi.WorkoutSession) {
        Task {
            await historyStore.deleteWorkout(id: workout.id())
        }
    }

    // MARK: - Placeholder formatting helpers

    private func workoutTitle(from ws: YokuUniffi.WorkoutSession) -> String {
        return ws.name() ?? "Unnamed workout"
    }
}

#Preview {
    // Use a preview Session with dummy data and no FFI
    RootView()
        .environmentObject(WorkoutStore.preview)
        .environmentObject(HistoryStore())
}
