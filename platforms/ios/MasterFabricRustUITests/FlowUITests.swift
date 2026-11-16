import XCTest

/// End-to-end navigation checks. The stack lives in Rust, so these verify the
/// SwiftUI ⇄ Rust round trip (push via dispatch, pop via Back → PopTo).
final class FlowUITests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    private func back(_ app: XCUIApplication) {
        app.navigationBars.buttons.element(boundBy: 0).tap()
    }

    func testHomeDetailBenchmarkInfoAndBack() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["wordmark"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.descendants(matching: .any)["hero-equation"].exists)

        // Home → Detail → back
        app.staticTexts["Shared Rust Core"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["More in this category"].waitForExistence(timeout: 3))
        back(app)
        XCTAssertTrue(app.buttons["open-benchmark"].waitForExistence(timeout: 3))

        // Home → Benchmark → back
        app.buttons["open-benchmark"].tap()
        XCTAssertTrue(app.buttons["run-all"].waitForExistence(timeout: 3))
        back(app)
        XCTAssertTrue(app.buttons["open-info"].waitForExistence(timeout: 3))

        // Home → Info → back
        app.buttons["open-info"].tap()
        XCTAssertTrue(app.navigationBars["Rust guide"].waitForExistence(timeout: 3))
        back(app)
        XCTAssertTrue(app.staticTexts["wordmark"].waitForExistence(timeout: 3))
    }

    func testDetailFavoriteToggleRoundTrip() {
        let app = XCUIApplication()
        app.launchArguments += ["-startScreen", "detail", "-detailId", "1"]
        app.launch()
        let add = app.buttons["Add to favorites"]
        let remove = app.buttons["Remove from favorites"]
        XCTAssertTrue(add.waitForExistence(timeout: 5) || remove.exists)
        let wasFavorite = remove.exists
        (wasFavorite ? remove : add).tap()
        XCTAssertTrue((wasFavorite ? add : remove).waitForExistence(timeout: 3))
        (wasFavorite ? add : remove).tap() // restore
    }

    /// Leaves the app, pages the springboard until the app icon is visible and pauses there
    /// so `simctl io screenshot` can capture it (ICON_PAUSE seconds via TEST_RUNNER_ICON_PAUSE).
    func testSpringboardShowsAppIcon() {
        XCUIApplication().launch()
        XCUIDevice.shared.press(.home)
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let icon = springboard.icons["MasterFabric Rust"]
        XCTAssertTrue(icon.waitForExistence(timeout: 5))
        var tries = 0
        while !icon.isHittable && tries < 4 {
            springboard.swipeLeft()
            tries += 1
            sleep(1)
        }
        XCTAssertTrue(icon.isHittable, "app icon not visible on any home screen page")
        let pause = UInt32(ProcessInfo.processInfo.environment["ICON_PAUSE"] ?? "0") ?? 0
        if pause > 0 { sleep(pause) }
    }
}
