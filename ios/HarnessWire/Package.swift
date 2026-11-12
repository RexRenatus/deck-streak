// swift-tools-version: 5.9
// The harness's protobuf wire codec (SPEC-339 R9, ADR-350): it depends on nothing, and its tests
// run on the macOS host with `swift test`, no simulator.
import PackageDescription

let package = Package(
    name: "HarnessWire",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "HarnessWire", targets: ["HarnessWire"]),
    ],
    targets: [
        .target(name: "HarnessWire"),
        .testTarget(name: "HarnessWireTests", dependencies: ["HarnessWire"]),
    ]
)
