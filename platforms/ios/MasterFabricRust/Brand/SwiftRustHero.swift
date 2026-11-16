import SwiftUI

/// Home hero: "Swift + Rust = 💥".
/// Swift logo = SF Symbol `swift`; Rust logo = official single-path SVG from
/// rust-lang/rust-artwork (CC-BY 4.0, unmodified paths), kept as a vector template image.
struct SwiftRustHero: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var pulse = false

    private static let swiftGradient = LinearGradient(
        colors: [Color(red: 0.98, green: 0.55, blue: 0.24), Color(red: 0.94, green: 0.32, blue: 0.22)],
        startPoint: .top, endPoint: .bottom)
    private static let rustGradient = LinearGradient(
        colors: [Color(red: 0.72, green: 0.27, blue: 0.13), Color(red: 0.36, green: 0.13, blue: 0.07)],
        startPoint: .topLeading, endPoint: .bottomTrailing)
    private static let boomGradient = LinearGradient(
        colors: [.yellow, .orange, .red], startPoint: .top, endPoint: .bottom)

    var body: some View {
        VStack(spacing: 14) {
            Wordmark(style: .header)
                .padding(.top, -4)

            HStack(spacing: 8) {
                tile(label: "Swift", fill: Self.swiftGradient) {
                    Image(systemName: "swift")
                        .font(.system(size: 30, weight: .semibold))
                        .foregroundStyle(.white)
                }

                op("plus")

                tile(label: "Rust", fill: Self.rustGradient) {
                    Image("RustLogo")
                        .renderingMode(.template)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 44, height: 44)
                        .foregroundStyle(.white)
                }

                op("equal")

                boom
            }
            .frame(maxWidth: .infinity)
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Swift plus Rust equals boom")
            .accessibilityIdentifier("hero-equation")

            Text("Native SwiftUI on a shared Rust core")
                .font(.subheadline.weight(.semibold))
                .multilineTextAlignment(.center)
            Text("State, navigation, data and business rules run in Rust. SwiftUI only renders.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 6)
        .onAppear {
            guard !reduceMotion else { return }
            withAnimation(.easeInOut(duration: 1.1).repeatForever(autoreverses: true)) { pulse = true }
        }
    }

    private var boom: some View {
        VStack(spacing: 6) {
            ZStack {
                Image(systemName: "burst.fill")
                    .font(.system(size: 66))
                    .foregroundStyle(Self.boomGradient)
                    .rotationEffect(.degrees(pulse ? 8 : -8))
                    .scaleEffect(pulse ? 1.06 : 0.94)
                    .shadow(color: .orange.opacity(0.45), radius: pulse ? 10 : 4)
                Image(systemName: "sparkles")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundStyle(.white)
                    .opacity(pulse ? 1 : 0.7)
            }
            .frame(width: 64, height: 64)
            Text("Boom").font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
        }
    }

    private func op(_ symbol: String) -> some View {
        Image(systemName: symbol)
            .font(.system(size: 18, weight: .heavy))
            .foregroundStyle(.secondary)
            .padding(.bottom, 20) // align with tile centers (labels sit below)
    }

    private func tile<C: View>(label: String, fill: LinearGradient, @ViewBuilder content: () -> C) -> some View {
        VStack(spacing: 6) {
            content()
                .frame(width: 64, height: 64)
                .background(fill, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                .shadow(color: .black.opacity(0.12), radius: 6, y: 3)
            Text(label).font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
        }
    }
}
