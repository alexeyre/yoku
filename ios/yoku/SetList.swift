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
    @EnvironmentObject var workoutState: WorkoutState

    // Chart layout constants
    private let chartHeight: CGFloat = 132
    private let chartHorizontalPadding: CGFloat = 12
    private let chartVerticalPadding: CGFloat = 6

    // Use the global shared LogCenter so it matches postFrontendLog(_:)
    // Remove the local @StateObject to avoid a separate instance.
    private var logCenter: LogCenter { sharedLogCenter }

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
                            .font(.system(size: 10, weight: .semibold, design: .monospaced))
                            .foregroundStyle(.secondary)

                        Text(exercise.name)
                            .font(.system(.footnote, design: .monospaced))
                            .lineLimit(1)

                        if exercise.id == workoutState.activeExerciseID {
                            Spacer(minLength: 4)
                            Text("ACTIVE")
                                .font(.system(.caption2, design: .monospaced))
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
                            HStack(spacing: 6) {
                                Rectangle()
                                    .frame(width: 2)
                                    .opacity(0.25)
                                    .accessibilityHidden(true)

                                Text(set.label)
                                    .font(.system(.footnote, design: .monospaced)) // match exercise font
                                    .lineLimit(1)

                                if set.id == workoutState.activeSetID {
                                    Spacer(minLength: 4)
                                    Image(systemName: "largecircle.fill.circle")
                                        .font(.system(size: 10, weight: .semibold))
                                        .foregroundStyle(.tint)
                                        .accessibilityLabel("Active set")
                                }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 1) // ultra-compact
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .listRowInsets(EdgeInsets(top: 0, leading: 14, bottom: 0, trailing: 6))
                        .listRowSeparator(.hidden)
                        .listRowBackground(
                            set.id == workoutState.activeSetID
                            ? Color.accentColor.opacity(0.06)
                            : Color.clear
                        )
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

            // Developer log section
            Section {
                DevActivityLogView(logCenter: logCenter)
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

    // MARK: - Extracted Chart Component

    private struct SetChartView: View {
        let exerciseName: String?
        let dataPoints: [Int]
        let activeSetIndex: Int?

        var body: some View {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    Text("CURRENT SET")
                        .font(.system(.caption, design: .monospaced))
                        .opacity(0.7)

                    if let name = exerciseName {
                        Text(name)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                    } else {
                        Text("None")
                            .font(.system(.caption, design: .monospaced))
                            .opacity(0.7)
                    }

                    Spacer()

                    if let idx = activeSetIndex {
                        Text("Set \(idx + 1)")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                }

                Chart {
                    ForEach(Array(dataPoints.enumerated()), id: \.offset) { idx, value in
                        LineMark(
                            x: .value("Set", idx + 1),
                            y: .value("Reps", value)
                        )
                        .interpolationMethod(.catmullRom)

                        PointMark(
                            x: .value("Set", idx + 1),
                            y: .value("Reps", value)
                        )
                        .foregroundStyle(
                            idx == activeSetIndex ? Color.accentColor : Color.secondary
                        )
                        .symbolSize(idx == activeSetIndex ? 80 : 40)
                    }

                    if let idx = activeSetIndex, idx < dataPoints.count {
                        RuleMark(x: .value("Active", idx + 1))
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
        .environmentObject(WorkoutState())
}
