import SwiftUI
import Charts

struct SetList: View {
    @EnvironmentObject var workoutState: WorkoutStore
    
    @State private var seenSetIDs: Set<UUID> = []
    @State private var previousSetCounts: [UUID: Int] = [:]

    
    private let chartHeight: CGFloat = 132
    private let chartHorizontalPadding: CGFloat = 12
    private let chartVerticalPadding: CGFloat = 6

    var body: some View {
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
            seenSetIDs = Set(workoutState.exercises.flatMap { $0.sets.map { $0.id } })
            previousSetCounts = Dictionary(uniqueKeysWithValues: workoutState.exercises.map { ($0.id, $0.sets.count) })
        }
        .onChange(of: workoutState.exercises) { _, _ in
            let currentSetIDs = Set(workoutState.exercises.flatMap { $0.sets.map { $0.id } })
            seenSetIDs.formUnion(currentSetIDs)
            for exercise in workoutState.exercises {
                if previousSetCounts[exercise.id] == nil {
                    previousSetCounts[exercise.id] = exercise.sets.count
                }
            }
        }
    }

    private struct ExerciseListView: View {
        @EnvironmentObject var workoutState: WorkoutStore
        @Binding var seenSetIDs: Set<UUID>
        @Binding var previousSetCounts: [UUID: Int]
        let availableWidth: CGFloat
        
        private var exerciseIds: [UUID] {
            workoutState.exercises.map { $0.id }
        }
        
        var body: some View {
            Group {
                ForEach(workoutState.exercises) { exercise in
                    ExerciseSectionView(
                        exercise: exercise,
                        seenSetIDs: $seenSetIDs,
                        previousSetCounts: $previousSetCounts,
                        availableWidth: availableWidth
                    )
                }
            }
            .animation(.spring(response: 0.3, dampingFraction: 0.8), value: exerciseIds)
        }
    }

    private struct ExerciseSectionView: View {
        @EnvironmentObject var workoutState: WorkoutStore
        let exercise: ExerciseModel
        @Binding var seenSetIDs: Set<UUID>
        @Binding var previousSetCounts: [UUID: Int]
        let availableWidth: CGFloat

        var body: some View {
            let prevCount: Int? = previousSetCounts[exercise.id]

            let content = VStack(spacing: 0) {
                ExerciseRowView(
                    exercise: exercise,
                    previousSetCount: prevCount,
                    availableWidth: availableWidth
                )
                .transition(.opacity.combined(with: .move(edge: .top)))
                .onAppear {
                    if previousSetCounts[exercise.id] == nil {
                        previousSetCounts[exercise.id] = exercise.sets.count
                    }
                }
                .onChange(of: exercise.sets.count) { _, newValue in
                    previousSetCounts[exercise.id] = newValue
                }

                if workoutState.isExpanded(exercise) {
                    ExerciseSetsList(
                        exercise: exercise,
                        seenSetIDs: $seenSetIDs,
                        availableWidth: availableWidth
                    )
                }
            }
            
            content
        }
    }

    private struct ExerciseSetsList: View {
        @EnvironmentObject var workoutState: WorkoutStore
        let exercise: ExerciseModel
        @Binding var seenSetIDs: Set<UUID>
        let availableWidth: CGFloat

        var body: some View {
            ForEach(exercise.sets) { set in
                makeSetButton(for: set)
            }
        }

        private func makeSetButton(for set: ExerciseSetModel) -> some View {
            Button {
                withAnimation(.easeInOut(duration: 0.2)) {
                    workoutState.setActiveExercise(exercise)
                    workoutState.activeSetID = set.id
                }
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
            .animation(.easeInOut(duration: 0.2), value: set.id == workoutState.activeSetID)
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
            .transition(.opacity.combined(with: .move(edge: .top)))
        }
    }

    private struct ExerciseRowView: View {
        @EnvironmentObject var workoutState: WorkoutStore
        var exercise: ExerciseModel
        var previousSetCount: Int?
        let availableWidth: CGFloat
        
        var body: some View {
            baseRow
        }

        private var baseRow: some View {
            Button {
                withAnimation(.easeInOut(duration: 0.2)) {
                    workoutState.setActiveExercise(exercise)
                }
                withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                    workoutState.toggle(expansionFor: exercise, expandIfCollapsed: true)
                }
            } label: {
                HStack(spacing: 6) {
                    Text(workoutState.isExpanded(exercise) ? "▼" : "▶")
                        .font(.appBody)
                        .foregroundStyle(.secondary)
                        .animation(.easeInOut(duration: 0.2), value: workoutState.isExpanded(exercise))

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
                            .transition(.scale.combined(with: .opacity))
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
            .animation(.easeInOut(duration: 0.2), value: exercise.id == workoutState.activeExerciseID)
        }
    }

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
        @EnvironmentObject var workoutState: WorkoutStore
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
            guard let backendID = set.backendID else { return }
            var setWeight = set.weight
            var setReps = set.reps
            if let newSetWeight = Double(weightText) {
                setWeight = newSetWeight
            }
            if let newSetReps = Int64(repsText) {
                setReps = newSetReps
            }
            try? await workoutState.updateWorkoutSet(id: backendID, weight: setWeight, reps: setReps)
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
                    Text(repsText == "1" ? "rep" : "reps")
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
                        .transition(.scale.combined(with: .opacity))
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .onChange(of: set.weight) { _, newValue in
                let currentTextVal = Double(weightText) ?? 0
                if abs(currentTextVal - newValue) > 0.01 {
                    syncStateFromSet()
                }
            }
            .onChange(of: set.reps) { _, newValue in
                if let textValue = Int64(repsText), textValue != newValue {
                    syncStateFromSet()
                } else if Int64(repsText) == nil {
                    syncStateFromSet()
                }
            }
        }
    }

    private struct SetChartView: View {
        @EnvironmentObject var workoutState: WorkoutStore
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
        .environmentObject(WorkoutStore.preview)
}
