//
//  SetList.swift
//  yoku
//
//  Created by Alex Holder on 13/11/2025.
//

import SwiftUI
import Charts

struct SetList: View {
    // Use the shared WorkoutState instead of local state
    @EnvironmentObject var workoutState: Session
    
    // Track seen set IDs to detect new sets
    @State private var seenSetIDs: Set<UUID> = []
    // Track previous set counts per exercise to detect new sets
    @State private var previousSetCounts: [UUID: Int] = [:]

    // Chart layout constants
    private let chartHeight: CGFloat = 132
    private let chartHorizontalPadding: CGFloat = 12
    private let chartVerticalPadding: CGFloat = 6


    var body: some View {
        let activeExercise = workoutState.activeExercise
        let activeSetIndex = workoutState.indexOfActiveSet(in: activeExercise)

        List {
            ExerciseListView(
                seenSetIDs: $seenSetIDs,
                previousSetCounts: $previousSetCounts
            )

            // Chart row below the set list
            Section {
                VStack(spacing: 0) {
                    Divider().opacity(0.15)
                    SetChartView(
                        exerciseName: activeExercise?.name,
                        dataPoints: workoutState.dataSeries(for: activeExercise),
                        activeSetIndex: activeSetIndex
                    )
                    .padding(.horizontal, chartHorizontalPadding)
                    .padding(.vertical, chartVerticalPadding)
                    .frame(height: chartHeight)
                    .listRowInsets(EdgeInsets(top: 0, leading: 0, bottom: 0, trailing: 0))
                }
                .listRowBackground(Color.clear)
                .listRowSeparator(.hidden)
            }

            // Suggestions section
            Section {
                ExerciseSuggestionsView()
                    .environmentObject(workoutState)
                    .listRowInsets(EdgeInsets(top: 0, leading: 6, bottom: 0, trailing: 6))
                    .listRowBackground(Color.clear)
            }
        }
        .listStyle(.plain)
        .environment(\.defaultMinListRowHeight, 12)
        .scrollContentBackground(.hidden)
        .onAppear {
            workoutState.initializeActiveSelectionIfNeeded()
            // Initialize seen set IDs with current sets
            seenSetIDs = Set(workoutState.exercises.flatMap { $0.sets.map { $0.id } })
            // Initialize previous set counts
            previousSetCounts = Dictionary(uniqueKeysWithValues: workoutState.exercises.map { ($0.id, $0.sets.count) })
        }
        .onChange(of: workoutState.exercises) { _, _ in
            // Update seen set IDs when exercises change
            let currentSetIDs = Set(workoutState.exercises.flatMap { $0.sets.map { $0.id } })
            // Keep track of all sets we've seen (don't remove deleted ones immediately)
            seenSetIDs.formUnion(currentSetIDs)
            // Update previous set counts for existing exercises
            for exercise in workoutState.exercises {
                if previousSetCounts[exercise.id] == nil {
                    previousSetCounts[exercise.id] = exercise.sets.count
                }
            }
        }
    }
    
    // MARK: - Exercise List View
    
    private struct ExerciseListView: View {
        @EnvironmentObject var workoutState: Session
        @Binding var seenSetIDs: Set<UUID>
        @Binding var previousSetCounts: [UUID: Int]
        
        var body: some View {
            ForEach(workoutState.exercises) { exercise in
                // Exercise row (tappable)
                ExerciseRowView(
                    exercise: exercise,
                    previousSetCount: previousSetCounts[exercise.id]
                )
                .onAppear {
                    // Initialize if not set
                    if previousSetCounts[exercise.id] == nil {
                        previousSetCounts[exercise.id] = exercise.sets.count
                    }
                }
                .onChange(of: exercise.sets.count) { oldValue, newValue in
                    // Update previous set count after checking for changes
                    // The ExerciseRowView will handle triggering the glow
                    previousSetCounts[exercise.id] = newValue
                }

                // Sets (expanded)
                if workoutState.isExpanded(exercise) {
                    ForEach(exercise.sets) { set in
                        Button {
                            workoutState.setActiveExercise(exercise)
                            workoutState.activeSetID = set.id
                        } label: {
                            SetInformationView(set: set)
                        }
                        .buttonStyle(.plain)
                        .listRowInsets(EdgeInsets(top: 0, leading: 14, bottom: 0, trailing: 6))
                        .listRowSeparator(.hidden)
                        .listRowBackground(
                            set.id == workoutState.activeSetID
                            ? Color.accentColor.opacity(0.06)
                            : Color.clear
                        )
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                            Button(role: .destructive) {
                                if let backendID = set.backendID {
                                    Task {
                                        try? await workoutState.deleteSet(id: backendID)
                                    }
                                }
                            } label: {
                                Text("DEL")
                                    .font(.appCaption)
                            }
                            .tint(.red.opacity(0.8))
                        }
                        .onAppear {
                            let wasNew = !seenSetIDs.contains(set.id)
                            seenSetIDs.insert(set.id)
                        }
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
        }
    }
    
    // MARK: - Exercise Row View
    
    private struct ExerciseRowView: View {
        @EnvironmentObject var workoutState: Session
        var exercise: ExerciseModel
        var previousSetCount: Int?
        @State private var showGlow: Bool = false
        @State private var lastSetCount: Int = 0
        
        var body: some View {
            Button {
                workoutState.setActiveExercise(exercise)
                workoutState.toggle(expansionFor: exercise, expandIfCollapsed: true)
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: workoutState.isExpanded(exercise) ? "chevron.down" : "chevron.right")
                        .font(.appIcon)
                        .foregroundStyle(.secondary)

                    Text(exercise.name)
                        .font(.appBody)
                        .lineLimit(1)

                    if exercise.id == workoutState.activeExerciseID {
                        Spacer(minLength: 4)
                        Text("ACTIVE")
                            .font(.appCaption2)
                            .foregroundStyle(.tint)
                            .padding(.horizontal, 4)
                            .padding(.vertical, 2)
                            .background(Color.accentColor.opacity(0.12))
                            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 2) // very tight
                .padding(.horizontal, 4)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .listRowInsets(EdgeInsets(top: 0, leading: 6, bottom: 0, trailing: 6))
            .listRowSeparator(.hidden)
            .listRowBackground(
                exercise.id == workoutState.activeExerciseID
                ? Color.accentColor.opacity(0.06)
                : Color.clear
            )
            .localGlowEffect(isActive: $showGlow)
            .onAppear {
                lastSetCount = exercise.sets.count
            }
            .onChange(of: exercise.sets.count) { oldValue, newValue in
                // Trigger glow if set count increased (new sets added)
                if newValue > oldValue {
                    withAnimation(.easeIn(duration: 0.2)) {
                        showGlow = true
                    }
                }
                lastSetCount = newValue
            }
        }
    }

    // MARK: - Delete handler

    @MainActor
    private func deleteSets(for exercise: ExerciseModel, at offsets: IndexSet) {
        Task {
            for index in offsets {
                guard index < exercise.sets.count else { continue }
                let set = exercise.sets[index]
                guard let backendID = set.backendID else { continue }
                do {
                    _ = try await workoutState.deleteSet(id: backendID)
                } catch {
                    print("Failed to delete set: \(error)")
                }
            }
        }
    }

    private struct SetInformationView: View {
        @EnvironmentObject var workoutState: Session
        var set: ExerciseSetModel
        @State var weightText: String
        @State var repsText: String
        @State private var showWeightGlow: Bool = false
        @State private var showRepsGlow: Bool = false
        @State private var previousWeight: Double
        @State private var previousReps: Int64
        @State private var isUserEditing: Bool = false  // Track if user is actively editing

        // Fixed widths for alignment across all sets
        private let weightFieldWidth: CGFloat = 50  // Enough for "999.9"
        private let repsFieldWidth: CGFloat = 35    // Enough for "999"

        init(set: ExerciseSetModel) {
            self.set = set
            self.weightText = String(format: "%.1f", set.weight)
            self.repsText = String(format: "%lld", set.reps)
            self._previousWeight = State(initialValue: set.weight)
            self._previousReps = State(initialValue: set.reps)
        }
        
        // Update state when set changes (e.g., after backend update)
        private func syncStateFromSet() {
            weightText = String(format: "%.1f", set.weight)
            repsText = String(format: "%lld", set.reps)
        }

        func propagateUpdates() async {
            // Mark that this is a user-initiated update
            isUserEditing = true
            
            var setWeight = set.weight
            var setReps = set.reps
            if let newSetWeight = Double(weightText) {
                setWeight = newSetWeight
            }
            if let newSetReps = Int64(repsText) {
                setReps = newSetReps
            }
            do {
                try await workoutState.updateWorkoutSet(id: self.set.backendID!, weight: setWeight, reps: setReps)
            } catch {
                // TODO
            }
            weightText = String(format: "%.1f", setWeight)
            repsText = String(format: "%lld", setReps)
            
            // Update previous values to match what we just set
            previousWeight = setWeight
            previousReps = setReps
            
            // Reset the flag after a short delay to allow backend update to complete
            try? await Task.sleep(nanoseconds: 100_000_000) // 0.1 seconds
            isUserEditing = false
        }

        var body: some View {
            HStack(spacing: 6) {
                Rectangle()
                    .frame(width: 2)
                    .opacity(0.25)
                    .accessibilityHidden(true)

                Text(set.label)
                    .font(.appBody)
                    .lineLimit(1)
                    .frame(width: 50, alignment: .leading)  // Fixed width for "Set X"

                TextField("Set weight", text: $weightText)
                    .onSubmit {
                        Task { await propagateUpdates() }
                    }
                    .font(.appBody)
                    .frame(width: weightFieldWidth, alignment: .trailing)  // Right-align numbers
                    .multilineTextAlignment(.trailing)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .localGlowEffect(isActive: $showWeightGlow)

                Text(String("x"))
                    .font(.appBody)
                    .lineLimit(1)
                
                TextField("reps", text: $repsText)
                    .onSubmit {
                        Task { await propagateUpdates() }
                    }
                    .font(.appBody)
                    .frame(width: repsFieldWidth, alignment: .trailing)  // Right-align numbers
                    .multilineTextAlignment(.trailing)
                    .lineLimit(1)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .localGlowEffect(isActive: $showRepsGlow)

                if let rpe = set.rpe {
                    Text(String(format: "@%.1f", rpe))
                        .font(.appBody)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                
                if set.id == workoutState.activeSetID {
                    Spacer(minLength: 4)
                    Image(systemName: "largecircle.fill.circle")
                        .font(.appIcon)
                        .foregroundStyle(.tint)
                        .accessibilityLabel("Active set")
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 1)
            .contentShape(Rectangle())
            .onChange(of: set.weight) { oldValue, newValue in
                // Sync state when set is updated from backend
                if abs(Double(weightText) ?? 0 - newValue) > 0.01 {
                    syncStateFromSet()
                    // Only trigger glow if this is NOT a user-initiated edit
                    // and the value actually changed from what we last saw
                    if !isUserEditing && abs(newValue - previousWeight) > 0.01 {
                        withAnimation(.easeIn(duration: 0.2)) {
                            showWeightGlow = true
                        }
                    }
                    previousWeight = newValue
                }
            }
            .onChange(of: set.reps) { oldValue, newValue in
                // Sync state when set is updated from backend
                if let textValue = Int64(repsText), textValue != newValue {
                    syncStateFromSet()
                    // Only trigger glow if this is NOT a user-initiated edit
                    // and the value actually changed from what we last saw
                    if !isUserEditing && newValue != previousReps {
                        withAnimation(.easeIn(duration: 0.2)) {
                            showRepsGlow = true
                        }
                    }
                    previousReps = newValue
                } else if Int64(repsText) == nil {
                    // Text doesn't parse to Int64, sync from set
                    syncStateFromSet()
                }
            }
        }
    }


    // MARK: - Extracted Chart Component

    private struct SetChartView: View {
        @EnvironmentObject var workoutState: Session
        let exerciseName: String?
        let dataPoints: [(Date, Double)]
        let activeSetIndex: Int?

        var body: some View {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    Text("CURRENT SET")
                        .font(.appChart)
                        .opacity(0.7)

                    if let name = exerciseName {
                        Text(name)
                            .font(.appChart)
                            .lineLimit(1)
                    } else {
                        Text("None")
                            .font(.appChart)
                            .opacity(0.7)
                    }

                    Spacer()

                    if let idx = activeSetIndex {
                        Text("Set \(idx + 1)")
                            .font(.appChart)
                            .foregroundStyle(.secondary)
                    }
                }

                Chart {
                    ForEach(Array(dataPoints.enumerated()), id: \.offset) { idx, point in
                        let date = point.0
                        let value = point.1
                        LineMark(
                            x: .value("Date", date, unit: .day),
                            y: .value("Weight", value)
                        )
                        .interpolationMethod(.catmullRom)

                        PointMark(
                            x: .value("Date", date, unit: .day),
                            y: .value("Weight", value)
                        )
                        .foregroundStyle(
                            idx == activeSetIndex ? Color.accentColor : Color.secondary
                        )
                        .symbolSize(idx == activeSetIndex ? 80 : 40)
                    }
                    if let idx = activeSetIndex, idx < dataPoints.count {
                        let (date, _) = dataPoints[idx]
                        RuleMark(x: .value("Active", date))
                            .foregroundStyle(Color.accentColor.opacity(0.25))
                    }
                }
                .chartYAxis {
                    AxisMarks(position: .leading)
                }
                .frame(height: 120)
            }
        }
    }
}

#Preview {
    SetList()
        .preferredColorScheme(.dark)
        .environmentObject(Session())
}
