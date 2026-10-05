---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The Swift harness: a generated project, the framework its own run built, one more read call, an engine-written fixture, and a measured seam

## Context and Problem Statement

ADR-335 chose a SwiftUI app over the engine through FFI and left its outcome to the iOS spike. The
spike's Rust half (SPEC-336, ADR-345) builds the adapter as an XCFramework and typechecks the Swift
bindings against it, on the one macOS runner the workflow hardening test admits to
`xcframework.yml`. Its Swift half, SPEC-339, is a harness that opens a synthetic collection,
lists its decks, renders a card and answers it on the iPhone and iPad simulators, and measures the
app's size, cold start and memory.

ADR-335 settles the client, the one static library, the card's isolation layers and the internal
build lane's trigger (with ADR-344). It does not settle nine things the harness must decide: how
the Xcode project is produced and kept reviewable; how the harness links the framework built from
its own commit; how a card is rendered when no allowed call renders one; where the synthetic
collection comes from when no allowed call writes one; how Swift speaks the engine's protobuf;
how a Swift test is seen red when nothing Swift compiles off the runner; how each figure is
measured; how the Swift code's tests are proved able to fail; and what the tree carries for the
internal build lane to inherit.

## Decision Drivers

- One engine on every surface: no second implementation of rendering, scheduling or the
  collection's schema in Swift (ADR-335, CHARTER 1's parity rule).
- The allow-list is the client's boundary: it widens only for a call the client needs, with that
  call's round trip, and never for a test's convenience (SPEC-336 R2, section 5).
- What is measured must be what was built: the harness links the framework its own commit built,
  never an older one.
- A reviewer reads every committed file: no generated or binary file a review cannot read.
- Nothing Swift compiles off the macOS runner, and that runner costs more per minute than the
  Linux jobs.
- Nothing private enters the public tree: no team, no real app id, no profile, no key (SPEC-334
  R20).
- A test is seen red for its criterion's reason before its code exists.

## Considered Options (the alternatives it was chosen against)

### D1. How the Xcode project is produced

- A committed XcodeGen spec — chosen, because every committed line is then reviewable YAML or a
  plain settings file, two deliveries that add a file never conflict in a project file, and the
  files the ios-swift probe reads (property lists, privacy manifest, xcconfig) stay in the tree. The
  spec is `ios/project.yml`, from which a pinned, digest-checked XcodeGen generates the project in
  CI; the generated project is ignored. Settings sit in committed `.xcconfig` files, and the
  Info.plist and the privacy manifest are committed files the spec points at.
- A committed `project.pbxproj` — rejected, because it is an old-style property list of generated
  identifiers that a reviewer cannot read, that two deliveries adding files conflict in, and that
  only Xcode writes correctly.
- Tuist — rejected, because its manifests are Swift that compiles only on the runner, so the
  project's shape could not be read or checked off it, and it brings a toolchain of its own where a
  single binary suffices.
- A Swift package alone, with no Xcode project — rejected, because a package cannot declare an iOS
  application bundle with its Info.plist, privacy manifest and UI-test target for `xcodebuild`.
- XcodeGen generating the Info.plist from `info.properties` — rejected, because the property list
  would then exist only after generation, and the probe's app-scoped rows would read the tree as
  holding no app.

### D2. How the harness links the framework its own commit built

- Two jobs in `xcframework.yml`: `harness` needs `xcframework` and downloads the artifact that run
  uploaded, into a local Swift package with a binary target for the C module and a Swift target for
  the generated bindings — chosen, because a job's `needs` and an artifact of its own run tie the
  framework to the commit by construction, and the runner is already admitted to this file by name.
- A workflow of its own for the harness — rejected, because the hardening test would admit the macOS
  runner to a second file, and the harness would either rebuild both static libraries or download
  another run's artifact, whose commit can differ.
- A committed XCFramework — rejected, because it is a binary a review cannot read, and it would be
  measured from whatever commit last built it.
- A remote binary target by URL and checksum — rejected, because it needs a published release
  before every harness run, and a pull request's framework is never published.
- The generated bindings compiled into the app's own module — rejected, because generated code then
  shares the app's isolation defaults and language mode; the ios-swift pack keeps it in its own
  module.

### D3. How a card is rendered

- One more read call on the allow-list — chosen, because the engine renders Anki's templates, the
  harness shows exactly what the engine renders, and the call reads a card and writes nothing. The
  call is `CardRenderingService.RenderExistingCard`, added with its round trip in this delivery.
- Rendering the note's fields in Swift — rejected, because it is a second template engine, which
  ADR-335's one-engine rule forbids, and its output would not be Anki's.
- Waiting for the engine core's dispatcher (#623) — rejected, because the spike's question, whether
  the engine through FFI renders on the simulators, would wait on a delivery that does not need its
  answer.
- A Rust-only delivery that widens the table first — rejected, because the call's only client is
  this harness, and SPEC-336 section 5 asks the client that needs a call to widen the table with its
  round trip.

### D4. Where the synthetic collection comes from

- An engine-written fixture — chosen, because the engine of the same commit writes it, nothing
  binary is committed, and the Swift assertions read the same names the Rust ones do. An example of
  the adapter crate, `harness-fixture`, run in the `xcframework` job, writes the collection the
  round-trip tests build, from one builder the tests and the example share; the run uploads it with
  the framework, and the harness bundles it and copies it into its container before opening.
- A committed collection file — rejected, because it is a binary database a review cannot read,
  whose ids and timestamps change with every rewrite, written by whichever engine last wrote it.
- Allowing the calls that add a deck and a note — rejected, because it widens the client's boundary
  to writes for a test's fixture.
- Swift writing the collection's tables itself — rejected, because it is a second implementation of
  the collection's schema.

### D5. How Swift speaks the engine's protobuf

- A hand-written wire codec — chosen, because six calls need a few dozen lines, the package
  depends on nothing, and its tests run in seconds without a simulator. It is `ios/HarnessWire`,
  covering exactly the fields the six calls send and read, tested on the macOS host against literal
  bytes.
- Generated message types from the engine's protobuf files — rejected for the harness, because they
  add a library and a generator pinned to the engine's files for six calls; the app shell decides
  this for the whole client (#625).
- Typed functions per call in the adapter — rejected, because SPEC-336 R1 gives a client one entry
  point, `run(service, method, bytes)`, and each typed function would be FFI surface to keep.

### D6. How a Swift test is seen red

- The harness's shape is committed first: the views, the model and the session actor with bodies
  that do nothing and return nothing, so every test compiles. The tests are committed alone on top
  of it and pushed alone; the pull request's own `harness` run reads each test's status and first
  failure from the result bundle into its report; the code is pushed only after that run ends —
  chosen, because each criterion is then red by assertion, on the runner that decides it, quoted in
  the record.
- Counting a test that does not compile as red — rejected, because a compile error is not the
  criterion's reason.
- A dispatch of the workflow for each commit — rejected, because the pull request's own run already
  runs it, and a second run doubles the macOS minutes.
- Pushing the code while the red run is in flight — refused, because the workflow cancels a pull
  request's superseded run, and the red would never be read.

### D7. How each figure is measured

- XCTest's own metrics in UI tests on the Release build of each simulator: the launch metric for
  the first frame, a signpost metric over the harness's own interval from its first line to the
  deck names shown, and the memory metric over one launch, open, list, render and answer; the app's
  size from the Release simulator bundle; every figure read from the result bundle by
  `xcresulttool` — chosen, because XCTest discards a warm-up iteration, repeats the rest and stores
  each figure in the result bundle, so the report reads figures the runner recorded, not ones the
  app chose to print.
- The app printing its own timings and memory — rejected, because it measures only what the app
  reaches after it starts, and its numbers are whatever it decides to report.
- Instruments traces recorded on the runner — rejected, because each trace is a large bundle to
  export and parse for three figures XCTest records directly.
- The Debug build — rejected, because it is unoptimised, so its size and timings are not the app's.

### D8. How the Swift tests are proved able to fail

- A committed list of hand-written codec mutants — chosen, because the codec is the Swift that
  carries invariants, and `swift test` on the host proves a mutant in seconds. The list is
  `ios/HarnessWire/swift-mutants.json`, each entry with its file, an anchor that occurs once, its
  replacement and the one test that must fail, proved by every run of the `harness-wire` job: each
  killer passes first, selecting exactly one test, then fails with its mutant installed, and the
  file is restored and checked byte for byte.
- A Swift mutation tool that generates mutants — rejected for the spike, because it rebuilds per
  mutant across the whole app, its simulator support is the weak part, and it adds a tool to the
  stack for a few dozen lines.
- A Swift killer kind in the shared mutation rows — rejected for the spike, because it changes the
  reader every Rust and Python row goes through, and a killer only the macOS runner can run; the
  delivery that keeps Swift for good decides it (#650).
- No Swift mutants — rejected, because a test nobody has seen fail proves nothing.

### D9. The seam the internal build lane inherits

- A seam of build settings with a reserved placeholder — chosen, because a simulator build then
  needs nothing private, and the lane sets the dev app id, the team, the number and the version
  from its own environment without editing a committed file. The seam is a shared scheme,
  `Harness`; the bundle id from one build setting, `DS_APP_ID`, whose committed value is a
  reverse-DNS name under the reserved `.invalid` top-level domain (RFC 2606), which no developer
  account can register and no upload can carry; the build number and the marketing version from
  `CURRENT_PROJECT_VERSION` and `MARKETING_VERSION`; no development team in any committed file, with
  an optional include of a local settings file git ignores; and code signing turned off on the CI
  command line, never in a settings file.
- Committing the dev app id — rejected, because app ids come from the private deploy rail and never
  enter the tree (SPEC-334 R20).
- An app id only in an ignored file — rejected, because a CI simulator build would then have no
  bundle id at all.
- Code signing off in a committed settings file — rejected, because the lane's archive would then
  have to override the tree to sign.

## Decision Outcome

Proposed options: D1 a committed XcodeGen spec and a generated project; D2 the harness as a job of
`xcframework.yml` that needs the framework job and downloads its run's artifact into a local
package; D3 one more read call, `RenderExistingCard`, with its round trip; D4 an engine-written
fixture from the adapter's example; D5 a hand-written wire codec for six calls; D6 shape first,
tests alone, red read from the pull request's own run; D7 XCTest's metrics on Release builds,
read from the result bundle; D8 a hand-written Swift mutant list proved by every codec run; D9 a
seam of build settings with a reserved placeholder and nothing private. Together they keep one
engine behind one boundary, measure the framework the commit built, leave every committed file
readable, and hand the internal build lane a tree it can sign from its own environment.

### Consequences

- Good, because the harness renders and schedules only through the engine, so what it shows is
  Anki's behaviour.
- Good, because a reviewer reads a YAML spec and settings files, never a generated project.
- Good, because each figure in SPEC-339 section 7 is the runner's record, tied to one commit.
- Bad, because a pull request that changes only `ios/**` rebuilds both static libraries, since the
  harness links only what its own run built.
- Bad, because the allow-list grows by one call before the engine core's dispatcher takes it over.
- Bad, because XcodeGen is one more pinned tool on the runner, and its digest is one more pin to
  keep.
- Bad, because the codec is hand-written: a field number typed wrong is caught only by the literal
  bytes in its tests and by the simulator flow.
- Bad, because Swift mutants are proved per run and sit outside the shared mutation population
  until a later delivery decides that (#650).
- Bad, because simulator figures are the runner's; a device can differ, and the device session
  measures it (#629).

### Confirmation

SPEC-339 A1 round-trips the render call; A2 and A3 hold the codec to literal bytes; A4 to A7
open, list, render and answer on both simulators; A8 holds the web view's two observable layers;
A9 holds D2's link to the run's own artifact; A10 holds D9's seam and refuses planted signing
material by name. The band's row on the sixth pair's literal proves A1 observes it. The red-first
record quotes each criterion's red from the run that read it. The `harness-wire` job's sweep reports
each Swift mutant killed.

## What would make this wrong

- The engine's render output needs a filter only a front end completes, so a Basic card renders a
  replacement node; the harness then shows a refusal, and the render call needs a completing step.
- A simulator figure differs from a device's by more than the decision it informs can bear; the
  device session's figures then decide ADR-335's outcome instead.
- The hand codec grows past the six calls; generated message types then win (#625).

## More Information

- SPEC-339; SPEC-334 (rows 1.1 and 1.2; R3, R4, R6, R17, R20).
- ADR-335 (the client), ADR-342 (one universal app), ADR-344 (internal builds), ADR-345 (the adapter
  and its job).
- `proto/anki/card_rendering.proto` at the engine's pinned rev, for the render call's fields.
- RFC 2606, for the reserved `.invalid` top-level domain.
