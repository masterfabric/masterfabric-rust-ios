import SwiftUI

/// MasterFabric textmark, reproduced the same way the MasterFabric ecomm app draws it:
/// a typographic wordmark (vector by nature) rather than a bitmap.
///   - `.header`: ecomm `BrandHeaderView` (SDUIComponentViews.swift): body/medium,
///     tracking 6, label color, centered full width, 16 pt horizontal + top padding.
///   - `.navigation`: ecomm `TopNavigationBar` (AppRootView.swift): subheadline/medium, tracking 4.
struct Wordmark: View {
    enum Style { case header, navigation }
    var style: Style = .header

    var body: some View {
        switch style {
        case .header:
            Text(verbatim: "MASTERFABRIC")
                .font(.body.weight(.medium))
                .tracking(6)
                .foregroundStyle(Color(uiColor: .label))
                .lineLimit(1)
                .minimumScaleFactor(0.6)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.horizontal, 16)
                .padding(.top, 16)
                .accessibilityIdentifier("wordmark")
                .accessibilityAddTraits(.isHeader)
        case .navigation:
            Text(verbatim: "MASTERFABRIC")
                .font(.subheadline.weight(.medium))
                .tracking(4)
                .foregroundStyle(Color(uiColor: .label))
                .lineLimit(1)
                .accessibilityIdentifier("wordmark-nav")
                .accessibilityAddTraits(.isHeader)
        }
    }
}
