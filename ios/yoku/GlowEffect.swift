import SwiftUI

// Local glow effect for individual elements
struct LocalGlowEffect: ViewModifier {
    @Binding var isActive: Bool
    @State private var gradientStops: [Gradient.Stop] = []
    @State private var timer: Timer?
    
    func body(content: Content) -> some View {
        ZStack {
            content
            
            if isActive {
                ZStack {
                    LocalEffectNoBlur(gradientStops: gradientStops, cornerRadius: 8)
                    LocalEffect(gradientStops: gradientStops, cornerRadius: 8, blur: 4)
                    LocalEffect(gradientStops: gradientStops, cornerRadius: 8, blur: 8)
                }
                .allowsHitTesting(false)
                .transition(.opacity)
            }
        }
        .onChange(of: isActive) { _, newValue in
            if newValue {
                startGlow()
            } else {
                stopGlow()
            }
        }
        .onAppear {
            if isActive {
                startGlow()
            }
        }
        .onDisappear {
            stopGlow()
        }
    }
    
    private func startGlow() {
        gradientStops = LocalGlowEffect.generateGradientStops()
        
        timer = Timer.scheduledTimer(withTimeInterval: 0.4, repeats: true) { _ in
            withAnimation(.easeInOut(duration: 0.5)) {
                gradientStops = LocalGlowEffect.generateGradientStops()
            }
        }
        
        // Auto-hide after 2 seconds
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            withAnimation(.easeOut(duration: 0.3)) {
                isActive = false
            }
        }
    }
    
    private func stopGlow() {
        timer?.invalidate()
        timer = nil
    }
    
    static func generateGradientStops() -> [Gradient.Stop] {
        [
            Gradient.Stop(color: Color(hex: "BC82F3"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "F5B9EA"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "8D9FFF"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "FF6778"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "FFBA71"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "C686FF"), location: Double.random(in: 0...1))
        ].sorted { $0.location < $1.location }
    }
}

extension View {
    func localGlowEffect(isActive: Binding<Bool>) -> some View {
        modifier(LocalGlowEffect(isActive: isActive))
    }
}

// Original full-screen glow effect (kept for compatibility)
struct GlowEffect: View {
    @State private var gradientStops: [Gradient.Stop] = GlowEffect.generateGradientStops()

    var body: some View {
        ZStack {
            EffectNoBlur(gradientStops: gradientStops, width: 6)
                .onAppear {
                    // Start a timer to update the gradient stops every second
                    Timer.scheduledTimer(withTimeInterval: 0.4, repeats: true) { _ in
                        withAnimation(.easeInOut(duration: 0.5)) {
                            gradientStops = GlowEffect.generateGradientStops()
                        }
                    }
                }
            Effect(gradientStops: gradientStops, width: 9, blur: 4)
                .onAppear {
                    // Start a timer to update the gradient stops every second
                    Timer.scheduledTimer(withTimeInterval: 0.4, repeats: true) { _ in
                        withAnimation(.easeInOut(duration: 0.6)) {
                            gradientStops = GlowEffect.generateGradientStops()
                        }
                    }
                }
            Effect(gradientStops: gradientStops, width: 11, blur: 12)
                .onAppear {
                    // Start a timer to update the gradient stops every second
                    Timer.scheduledTimer(withTimeInterval: 0.4, repeats: true) { _ in
                        withAnimation(.easeInOut(duration: 0.8)) {
                            gradientStops = GlowEffect.generateGradientStops()
                        }
                    }
                }
            Effect(gradientStops: gradientStops, width: 15, blur: 15)
                .onAppear {
                    // Start a timer to update the gradient stops every second
                    Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { _ in
                        withAnimation(.easeInOut(duration: 1)) {
                            gradientStops = GlowEffect.generateGradientStops()
                        }
                    }
                }
        }
    }
    
    // Function to generate random gradient stops
    static func generateGradientStops() -> [Gradient.Stop] {
        [
            Gradient.Stop(color: Color(hex: "BC82F3"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "F5B9EA"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "8D9FFF"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "FF6778"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "FFBA71"), location: Double.random(in: 0...1)),
            Gradient.Stop(color: Color(hex: "C686FF"), location: Double.random(in: 0...1))
        ].sorted { $0.location < $1.location }
    }
}

struct Effect: View {
    var gradientStops: [Gradient.Stop]
    var width: CGFloat
    var blur: CGFloat

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 55)
                .strokeBorder(
                    AngularGradient(
                        gradient: Gradient(stops: gradientStops),
                        center: .center
                    ),
                    lineWidth: width
                )
                .frame(
                    width: UIScreen.main.bounds.width,
                    height: UIScreen.main.bounds.height
                )
                .padding(.top, -17)
                .blur(radius: blur)
        }
    }
}

struct EffectNoBlur: View {
    var gradientStops: [Gradient.Stop]
    var width: CGFloat

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 55)
                .strokeBorder(
                    AngularGradient(
                        gradient: Gradient(stops: gradientStops),
                        center: .center
                    ),
                    lineWidth: width
                )
                .frame(
                    width: UIScreen.main.bounds.width,
                    height: UIScreen.main.bounds.height
                )
                .padding(.top, -26)
        }
    }
}

// Local effect components for individual elements
struct LocalEffect: View {
    var gradientStops: [Gradient.Stop]
    var cornerRadius: CGFloat
    var blur: CGFloat
    
    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius)
            .strokeBorder(
                AngularGradient(
                    gradient: Gradient(stops: gradientStops),
                    center: .center
                ),
                lineWidth: 3
            )
            .blur(radius: blur)
    }
}

struct LocalEffectNoBlur: View {
    var gradientStops: [Gradient.Stop]
    var cornerRadius: CGFloat
    
    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius)
            .strokeBorder(
                AngularGradient(
                    gradient: Gradient(stops: gradientStops),
                    center: .center
                ),
                lineWidth: 2
            )
    }
}

extension Color {
    init(hex: String) {
        let scanner = Scanner(string: hex)
        _ = scanner.scanString("#")
        
        var hexNumber: UInt64 = 0
        scanner.scanHexInt64(&hexNumber)
        
        let r = Double((hexNumber & 0xff0000) >> 16) / 255
        let g = Double((hexNumber & 0x00ff00) >> 8) / 255
        let b = Double(hexNumber & 0x0000ff) / 255
        
        self.init(red: r, green: g, blue: b)
    }
}

#Preview {
    GlowEffect()
}
