import Foundation
import Combine

@MainActor
final class TimerStore: ObservableObject {
    @Published var elapsedTime: TimeInterval = 0
    @Published var isPaused: Bool = false
    @Published var startTime: Date?
    
    private var timerCancellable: AnyCancellable?
    private var saveHandler: ((TimeInterval) -> Void)?
    
    var isRunning: Bool {
        timerCancellable != nil && !isPaused
    }
    
    func setSaveHandler(_ handler: @escaping (TimeInterval) -> Void) {
        self.saveHandler = handler
    }
    
    func start(from initialElapsed: TimeInterval? = nil, startTime: Date? = nil) {
        if let initial = initialElapsed {
            self.elapsedTime = initial
            self.startTime = startTime ?? Date().addingTimeInterval(-initial)
        } else if self.startTime == nil {
            self.startTime = Date().addingTimeInterval(-elapsedTime)
        }
        
        guard timerCancellable == nil else { return }
        
        isPaused = false
        
        timerCancellable = Timer.publish(every: 1.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                guard let self else { return }
                self.elapsedTime += 1
                if Int(self.elapsedTime) % 10 == 0 {
                    self.saveHandler?(self.elapsedTime)
                }
            }
    }
    
    func pause() {
        timerCancellable?.cancel()
        timerCancellable = nil
        isPaused = true
        saveHandler?(elapsedTime)
    }
    
    func resume() {
        guard isPaused else { return }
        start()
    }
    
    func stop() {
        timerCancellable?.cancel()
        timerCancellable = nil
        isPaused = false
        startTime = nil
        elapsedTime = 0
    }
}

