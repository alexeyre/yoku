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

    // Chart layout constants
    private let chartHeight: CGFloat = 132
    private let chartHorizontalPadding: CGFloat = 12
    private let chartVerticalPadding: CGFloat = 6


    var body: some View {
        let activeExercise = workoutState.activeExercise
        let activeSetIndex = workoutState.indexOfActiveSet(in: activeExercise)

        List {
            ForEach(workoutState.exercises) { exercise in
                // Exercise row (tappable)
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
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }

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

        // dynamic width states for measured content
        @State private var weightFieldWidth: CGFloat = 0

        // Adjust this to add some breathing room for caret/padding
        private let textFieldHorizontalPadding: CGFloat = 8

        init(set: ExerciseSetModel) {
            self.set = set
            self.weightText = String(format: "%.1f", set.weight)
            self.repsText = String(format: "%d", set.reps)
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
                // TODO
            }
            weightText = String(format: "%.1f", setWeight)
            repsText = String(format: "%d", setReps)
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

                ZStack(alignment: .leading) {
                    // Hidden text to measure current weight string width
                    MeasuredText(
                        text: weightText,
                        font: .appBody,
                        onWidthChange: { w in
                            // Clamp to a minimum so empty string still shows a small field
                            let minWidth: CGFloat = 16
                            weightFieldWidth = max(minWidth, w + textFieldHorizontalPadding)
                        }
                    )
                    .hidden()

                    TextField("Set weight", text: $weightText)
                        .onSubmit {
                            Task { await propagateUpdates() }
                        }
                        .font(.appBody)
                        .frame(width: weightFieldWidth, alignment: .leading)
                        .multilineTextAlignment(.leading)
                        .textFieldStyle(.plain)
                }

                Text(String("x"))
                    .font(.appBody)
                    .lineLimit(1)
                
                TextField("reps", text: $repsText)
                    .onSubmit {
                        Task { await propagateUpdates() }
                    }
                    .font(.appBody)
                    .fixedSize() // let reps grow to content
                    .lineLimit(1)

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
        }
    }

    // Helper to measure text width for a given font via preferences
    private struct WidthPreferenceKey: PreferenceKey {
        static var defaultValue: CGFloat = 0
        static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
            value = nextValue()
        }
    }

    private struct MeasuredText: View {
        let text: String
        let font: Font
        var onWidthChange: (CGFloat) -> Void = { _ in }

        init(text: String, font: Font, onWidthChange: @escaping (CGFloat) -> Void = { _ in }) {
            self.text = text
            self.font = font
            self.onWidthChange = onWidthChange
        }

        var body: some View {
            Text(text.isEmpty ? " " : text)
                .font(font)
                .background(
                    GeometryReader { proxy in
                        Color.clear
                            .preference(key: WidthPreferenceKey.self, value: proxy.size.width)
                    }
                )
                .onPreferenceChange(WidthPreferenceKey.self, perform: onWidthChange)
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
