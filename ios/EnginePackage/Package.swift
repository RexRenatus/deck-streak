// swift-tools-version: 5.9
// The engine as a Swift package (SPEC-339 R2, ADR-350): the XCFramework the same workflow run built,
// as a binary target for the C module, and the generated bindings in a module of their own. CI
// places both here from that run's `xcframework` artifact; git ignores them.
import PackageDescription

let package = Package(
    name: "EnginePackage",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "DeckStreakFFI", targets: ["DeckStreakFFI"]),
    ],
    targets: [
        .binaryTarget(name: "deck_streak_ffiFFI", path: "DeckStreakFFI.xcframework"),
        .target(
            name: "DeckStreakFFI",
            dependencies: ["deck_streak_ffiFFI"],
            linkerSettings: [
                // A static library carries no link directives, so the package names what the
                // engine's dependencies link on Apple targets: core-foundation-sys's framework and
                // the libc crate's iconv.
                .linkedFramework("CoreFoundation"),
                .linkedLibrary("iconv"),
            ]
        ),
    ]
)
