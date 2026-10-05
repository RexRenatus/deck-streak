### Added

- A SwiftUI harness for the iPhone and the iPad over the engine, through the FFI (SPEC-339, #616):
  it opens a collection, lists its decks, renders the next card in an isolated web view and
  answers it, through six engine calls and no others. It is a spike, not the app (ADR-335).
- The harness's own wire codec, a small Swift package with its request and response tests, so the
  bytes the harness sends and reads are judged without a simulator.
- The `harness-wire` and `harness` jobs in the `xcframework` workflow: the codec's tests and its
  mutants on the macOS runner, then the harness's tests on two simulators and section 7's
  measurements, in Release.
