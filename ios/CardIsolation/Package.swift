// swift-tools-version: 5.9
// The card view's isolation (SPEC-349 R1 to R4, ADR-360 D2): the one factory every card web view
// comes from, its navigation gate, its rule list and its window refusal. It depends on nothing,
// and its tests run on the macOS host with `swift test`, no simulator.
import PackageDescription

let package = Package(
    name: "CardIsolation",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "CardIsolation", targets: ["CardIsolation"]),
    ],
    targets: [
        .target(name: "CardIsolation"),
        .testTarget(name: "CardIsolationTests", dependencies: ["CardIsolation"]),
    ]
)
