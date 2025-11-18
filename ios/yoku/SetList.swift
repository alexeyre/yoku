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

    
    private let chartHeight: CGFloat = 132
    private let chartHorizontalPadding: CGFloat = 12
    private let chartVerticalPadding: CGFloat = 6


    var body: some View {
        let activeExercise = workoutState.activeExercise
        GeometryReader { geometry in
            ScrollView {
                LazyVStack(spacing: 0) {
                    ExerciseListView(
                        seenSetIDs: $seenSetIDs,
                        previousSetCounts: $previousSetCounts,
                        availableWidth: geometry.size.width
                    )
                    
                    CommandInputBar()
                        .environmentObject(workoutState)
                        .padding(.top, 4)
                }
                .font(.appBody)
            }
        }
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
        let availableWidth: CGFloat
        
        var body: some View {
            ForEach(workoutState.exercises) { exercise in
                // Exercise row (tappable)
                ExerciseRowView(
                    exercise: exercise,
                    previousSetCount: previousSetCounts[exercise.id],
                    availableWidth: availableWidth
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
                            SetInformationView(set: set, availableWidth: availableWidth)
                        }
                        .buttonStyle(.plain)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.leading, 24)
                        .padding(.trailing, 12)
                        .background(
                            set.id == workoutState.activeSetID
                            ? Color.accentColor.opacity(0.06)
                            : Color.clear
                        )
                        .glowOnSetEvent(setID: set.backendID)
                        .gesture(
                            DragGesture(minimumDistance: 50)
                                .onEnded { value in
                                    if value.translation.width < -100 {
                                        if let backendID = set.backendID {
                                            Task {
                                                try? await workoutState.deleteSet(id: backendID)
                                            }
                                        }
                                    }
                                }
                        )
                        .onAppear {
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
        let availableWidth: CGFloat
        
        var body: some View {
            Button {
                workoutState.setActiveExercise(exercise)
                workoutState.toggle(expansionFor: exercise, expandIfCollapsed: true)
            } label: {
                HStack(spacing: 6) {
                    Text(workoutState.isExpanded(exercise) ? "▼" : "▶")
                        .font(.appBody)
                        .foregroundStyle(.secondary)

                    Text(exercise.name)
                        .font(.appBody)
                        .lineLimit(1)

                    if exercise.id == workoutState.activeExerciseID {
                        Spacer(minLength: 0)
                        Text("ACTIVE")
                            .font(.appBody)
                            .foregroundStyle(.tint)
                            .padding(.horizontal, 4)
                            .background(Color.accentColor.opacity(0.12))
                            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .frame(maxWidth: .infinity, alignment: .trailing)
            .padding(.horizontal, 12)
            .background(
                exercise.id == workoutState.activeExerciseID
                ? Color.accentColor.opacity(0.06)
                : Color.clear
            )
            .glowOnExerciseEvent(exerciseID: exercise.backendID)
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
        let availableWidth: CGFloat
        @State var weightText: String
        @State var repsText: String

        init(set: ExerciseSetModel, availableWidth: CGFloat) {
            self.set = set
            self.availableWidth = availableWidth
            self.weightText = String(format: "%.1f", set.weight)
            self.repsText = String(format: "%lld", set.reps)
        }
        
        private var setLabelWidth: CGFloat {
            50
        }
        
        private var weightFieldWidth: CGFloat {
            60
        }
        
        private var repsFieldWidth: CGFloat {
            35
        }
        
        private func syncStateFromSet() {
            weightText = String(format: "%.1f", set.weight)
            repsText = String(format: "%lld", set.reps)
        }

        func propagateUpdates() async {
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
            }
            weightText = String(format: "%.1f", setWeight)
            repsText = String(format: "%lld", setReps)
        }

        var body: some View {
            HStack(spacing: 0) {
                Text(set.label)
                    .font(.appBody)
                    .lineLimit(1)
                    .frame(width: setLabelWidth, alignment: .leading)
                
                HStack(spacing: 0) {
                    
                    TextField("Set weight", text: $weightText)
                        .onSubmit {
                            Task { await propagateUpdates() }
                        }
                        .font(.appBody)
                        .frame(width: weightFieldWidth, alignment: .trailing)
                        .multilineTextAlignment(.trailing)
                        .textFieldStyle(.plain)
                        .padding(0)
                    
                    Text("kg")
                        .font(.appBody)
                        .foregroundStyle(.secondary)
                        .frame(width: 20, alignment: .leading)
                        .padding(0)
                }
                HStack(spacing: 0) {
                    TextField("reps", text: $repsText)
                        .onSubmit {
                            Task { await propagateUpdates() }
                        }
                        .font(.appBody)
                        .frame(width: repsFieldWidth, alignment: .leading)
                        .multilineTextAlignment(.trailing)
                        .lineLimit(1)
                        .textFieldStyle(.plain)
                        .padding(0)
                    Text("reps")
                        .font(.appBody)
                        .foregroundStyle(.secondary)
                        .frame(width: 35, alignment: .leading)
                        .padding(0)
                }

                if let rpe = set.rpe {
                    Text(String(format: "@%.1f", rpe))
                        .font(.appBody)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                
                if set.id == workoutState.activeSetID {
                    Spacer(minLength: 0)
                    Text("●")
                        .font(.appBody)
                        .foregroundStyle(.tint)
                        .accessibilityLabel("Active set")
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .onChange(of: set.weight) { oldValue, newValue in
                if abs(Double(weightText) ?? 0 - newValue) > 0.01 {
                    syncStateFromSet()
                }
            }
            .onChange(of: set.reps) { oldValue, newValue in
                if let textValue = Int64(repsText), textValue != newValue {
                    syncStateFromSet()
                } else if Int64(repsText) == nil {
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
