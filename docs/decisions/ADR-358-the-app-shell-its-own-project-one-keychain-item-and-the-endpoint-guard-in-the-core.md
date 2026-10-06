---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The app shell: its own generated project, one Keychain item for the host key, sync settings as build settings, the endpoint guard in the core, one codec for the client, a lexical thin-Swift census, and an unsigned device archive

## Context and Problem Statement

#625 starts the universal app: a generated project for one iPhone and iPad app with a split view
on the iPad, the sync login held in the Keychain, the deck list over FFI, the dev app id syncing to
the staging sync user, and signing identifiers rendered from private configuration. ADR-335 put
the client behind one Rust entry point, `run(service, method, bytes)`, and the sync login in the
Keychain; ADR-342 chose one universal SwiftUI target; ADR-344 decided that internal builds carry
the dev app id and sync to the staging sync user; ADR-350 decided, for the harness, the generator,
the framework's route from its own run into a local package, a hand-written codec, and a signing
seam of build settings. None of them decides:

- where the app's targets live beside the harness's;
- what the login stores, under which key, and what the app may cache;
- how the app learns which server and which user it syncs to;
- what stops a login request from reaching a server that is not DeckStreak's: the engine sends a
  login with no endpoint to its built-in default server (SPEC-347 M13);
- whether the hand codec stays once it passes six calls, which ADR-350 names as the point where
  generated types would win and leaves to this issue;
- how "no logic in Swift" is measured, which #624's design leaves to this issue;
- how the app is shown archivable without the lane's signing.

## Decision Drivers

- One engine behind one boundary: Swift renders and stores; it decides nothing the engine or the
  core can decide (ADR-335).
- A credential never leaves the device it was typed on, and the password outlives the call by
  nothing.
- REL-01 as a property of the build, not a habit: a dev build cannot be pointed at real data by
  what someone types.
- Every rule a Linux box can check is checked there; the macOS runner proves only what needs it.
- No committed signing identifier, team id, built framework, second Rust library, remote package,
  environment pin or upload lane (SPEC-334 R20; ADR-350).

## Considered Options (the alternatives each was chosen against)

### D1. Where the app's targets live

- Its own XcodeGen spec, `ios/app.yml`, generating `ios/DeckStreak.xcodeproj` beside the harness's project — chosen, because
  the product is named for itself, the harness keeps its record and its commands unchanged, and the
  two specs share the local packages and the seam. The existing ignore rule `ios/*.xcodeproj/`
  covers the second project.
- A second application target in `ios/project.yml` — rejected, because the product would live in a
  project named `Harness`, and renaming that project rewrites SPEC-339's acceptance commands and
  the `harness` job's three `-project` lines for a name.
- Growing the harness into the app — rejected, because the harness is the spike's record: its
  synthetic collection, its measurement class and its card web view would ship, and its figures
  would lose the baseline they were taken against.
- A committed project file, Tuist, or a package alone — settled against by ADR-350 D1.

### D2. What the login stores

- One Keychain generic-password item: service the configured endpoint, account the configured
  user, value the host key; `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`; not
  synchronizable; the app's default access group. The password is held only for the call and
  cleared after it — chosen, because the host key is what the engine's later sync calls take, the
  password is needed by nothing after the login, and keying the item by the configuration means a
  build with another endpoint or user finds no item, with no comparison written in Swift.
- Storing the password too, to log in again silently — rejected, because it keeps the one secret
  that can mint new host keys on every device that ever signed in, where the host key alone serves.
- A file under Data Protection — rejected, because SPEC-334 R19 places the sync login in the
  Keychain, and a file is carried by backups to another device.
- A synchronizable item — rejected, because the credential would leave the device it was typed on,
  and a dev build's credential would follow its tester's account.
- Storing the endpoint inside the item and comparing it in Swift — rejected, because that is a
  decision in Swift and a stale item needs clean-up logic, where a configuration-keyed item simply
  is not found.
- `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` — rejected, because the sync that #633 runs at a
  session's start and end may run while the screen locks, and the after-first-unlock class is the
  one Apple documents for an item a background task reads.

### D3. How the app learns its server and its user

- Two build settings with reserved placeholders, `DS_SYNC_ENDPOINT` (an `https` URL on a host under `.invalid`) and `DS_SYNC_USER`, rendered into the Info.plist as `DSSyncEndpoint` and `DSSyncUser`, and read by one configuration file; the lane renders the real values on its command line, a developer through the ignored `Signing.local.xcconfig` — chosen, because
  the seam already exists (ADR-350 D9), the committed tree carries nothing private, and which data a
  build reaches is decided by the build, so a dev build cannot be pointed at real data from its own
  screen.
- The server and the user typed on the login screen — rejected, because REL-01 would then rest on
  what a tester types, not on how the build was made.
- The real values committed — rejected, because the server's address is private configuration, as
  the app ids are (SPEC-334 R20).
- A configuration file bundled beside the Info.plist — rejected, because it is a second rendering
  seam where one exists.
- A Swift source generated with the values — rejected, because it is generated code the census and
  a reviewer cannot read in the tree, and one more build phase.

### D4. What stops a login from reaching another server

- A guard in the engine core, on the login pair, before the engine sees it: it decodes the request,
  refuses an absent, empty, unparseable or credential-carrying endpoint and any scheme but `https`,
  admits `http` only to a loopback IP literal (the engine's own test server), parses with the URL
  parser the engine itself uses (a direct `url` edge for `deck-streak-engine-core`, a package the
  lockfile already holds), and refuses in the engine's own error shape, a `BackendError` of kind
  `INVALID_INPUT`, so the client reads one shape of refusal — chosen, because the hazard is the
  engine's default, the core is the one client-side holder of the engine, and every transport that
  later admits the pair inherits the guard.
- No guard — rejected, because a login with no endpoint sends the password to the engine's built-in
  default server, which is not DeckStreak's (SPEC-347 M13).
- The guard in Swift — rejected, because it is a decision in Swift (D6 refuses it), and the web
  client would need its own.
- The guard in the native adapter — rejected, because after #623 the adapter depends on the core
  alone and does not decode the engine's messages.
- A typed login function on the FFI surface — rejected, because SPEC-336 R1 gives the client one
  entry point, and a typed function is surface #624 would have to keep.
- A prefix test on the string — rejected, because `http://127.0.0.1@other.example/` begins like a
  loopback URL and is not one, and a prefix cannot tell a loopback literal from a name.
- `https` alone with no loopback exception — rejected, because the engine's test server speaks
  plain HTTP, so the login would have no network-free round trip; it was the option that needed no
  new edge.
- A new refusal variant across UniFFI — rejected, because every exhaustive Swift `switch` over
  `EngineRefusal`, the harness's included, stops compiling, and the client would read two shapes of
  refusal for one call.

### D5. How the client speaks the engine's protobuf, past six calls

- The hand-written codec stays the whole client's, at `ios/HarnessWire` under its name, and grows by the messages each delivery needs: here one request (the login) and two decoders (the host key and the engine's error message) — chosen, because
  all three messages are flat, each is pinned by literal bytes and by mutants the `harness-wire` job
  sweeps, and the codec still depends on nothing. ADR-350 named passing six calls as the point where
  generated types would win; this decision measures that point at seven and keeps the hand codec,
  with a new condition below.
- Generated message types now — rejected, because they need the generator's runtime library: as a
  remote package it breaks #624's rule that no package declares a remote dependency, and vendored
  as a local package it brings a large third-party source tree for three flat messages; and a
  generator pinned to the engine's files becomes one more tool on the runner.
- A codec in Rust, exported as typed records through UniFFI — rejected, because it amends SPEC-336
  R1's one entry point, and every message grows the umbrella's bindings.
- A second codec for the app — rejected, because two readers of one wire must then be kept equal.
- Renaming the package for the whole client now — rejected, because it moves the `harness-wire`
  job's paths, the mutants file and the paths #650 plans to read, for a name.

### D6. How "no logic in Swift" is measured

- A lexical census on the Linux box: a closed register giving every `.swift` under `ios/` one role;
  doors that confine the engine, the file system, the Keychain and the info dictionary each to one
  role; names no file may hold (other networking, other storage); and per-file decision counts,
  recorded exactly and bounded by a ceiling per role — chosen, because it runs without a compiler,
  every new branch is a visible line in the register's diff, and the ceiling bounds what review can
  approve.
- Review alone — rejected, because a rule nobody measures is a rule each reviewer must remember.
- SwiftLint or a SwiftSyntax tool in the macOS job — rejected, because it is a third-party tool on
  the runner, a macOS-only verdict for a rule the box can check, and its complexity rule bounds
  functions, not what a file is allowed to be.
- A cyclomatic counter from the package index — rejected, because it is a new dependency for a
  count the census reads itself.
- Refusing every Swift file that is not a view — rejected, because the engine call, the Keychain and
  the info dictionary are reachable only from Swift; the doors confine them instead.
- A ceiling without exact counts — rejected, because logic would then grow silently under the
  ceiling.

### D7. How the app is shown archivable

- An unsigned Release archive for a generic iOS device in the `harness` job, with the archived executable's required-reason imports compared with the app's privacy manifest — chosen, because
  it links the device slice and the app's privacy declarations before the lane exists, with no
  signing material anywhere.
- Leaving the first device link to the lane (#634) — rejected, because a slice, link or manifest
  failure would then first appear behind signing and an upload.
- A signed archive — rejected, because it needs the team and a certificate in this job, which are
  the lane's (SPEC-334 R20).
- A simulator build alone — rejected, because it never links the device slice.

### D8. Which collection the app opens

- One collection at a fixed path under Application Support, created by the engine when absent — chosen, because
  the deck list then reads the learner's own collection from the first launch, and the engine
  decides its contents (a fresh one holds its default deck).
- The harness's synthetic collection, bundled — rejected, because a real app would show made-up
  decks.
- Nothing until the first sync — rejected, because a first sync on an empty collection is the
  one-way download (1,6), refused until #631 and #633.
- A collection per sync user — rejected, because the app holds one collection; what a changed user
  means for it is a sync question (#633).

### D9. Where the census admits the network names

SPEC-347 R11 refused `import Network` and `NWConnection` in every Swift file. The card probe's
listeners (`ios/CardProbeTests/Listeners.swift`, SPEC-349) hold both, and the census starts from
that tree, so the rule as written refused it on its first reading.

- `import Network`, `NWConnection` and `NWConnectionGroup` admitted only in `isolation` files and in `test` files under `ios/CardIsolation/Tests/` and `ios/CardProbeTests/`, each admission printed by its file and its name — chosen, because
  the card view's proof listens for what a planted card would send, and the census still refuses
  the names in the app's, the codec's, the harness's and every other test file, and refuses every
  other forbidden name everywhere.
- R11 as written, the names refused in every file — rejected, because it refuses the tree it starts
  from: the listeners are how SPEC-349's planted suite shows that no card reaches the network.
- The names admitted in every test file — rejected, because no other test target holds a listener,
  and an admission wider than its holders would let a networking test grow unseen.
- The listeners removed or rewritten without the framework — rejected, because the planted suite's
  proof is the listening, and it is SPEC-349's, not this delivery's.
- A role of its own for the listener files — rejected, because they are test files of a test target,
  and a role per file grows the register's vocabulary for one admission.

### D10. Where the app's project is generated

SPEC-347 R13 put a second `xcodegen generate --spec ios/app.yml` line in the `harness` job's step
"the project, generated". Both TestFlight lanes copy the harness's steps from the framework's
download through that step, and SPEC-352 A20 holds each copy equal to the harness's.

- The generate line as the first line of the new step "the app's tests, Debug, on the iPhone and then the iPad", after the harness's steps — chosen, because
  the step "the project, generated" keeps its one line, so the lanes' copy stays equal with no
  lane file changed, and the app's project is generated in the step that first reads it.
- A second line in the step "the project, generated" — rejected, because the lanes copy that step,
  so it either reddens SPEC-352 A20 or changes both lane files, which are #634's.
- A step of its own that generates the app's project before the harness's tests — rejected, because
  it puts an app step among the harness's, which breaks A14's order (every app step after the
  harness's) and widens the lanes' copied range.
- The app's project generated in a job of its own — rejected, because R13 adds no job, and the
  app's tests need the framework this run's `harness` job places.

## Decision Outcome

Proposed options: D1 its own generator spec, `ios/app.yml`; D2 one configuration-keyed Keychain
item for the host key, after first unlock, this device only, never the password; D3 the server and
the user as build settings with reserved placeholders; D4 the endpoint guard in the core, parsed by
the engine's own URL parser, refusing in the engine's error shape; D5 the hand codec as the whole
client's; D6 a lexical census of roles, doors, names and exact decision counts; D7 an unsigned
device archive in the `harness` job; D8 one engine-created collection. Together they keep every
decision in Rust, every secret off the tree and on the device it was typed on, and hand the lane an
app it can sign and upload from its own environment.

Part 2 adds D9, the network names admitted in the card view's isolation and probe files alone, and
D10, the app's project generated by the first line of its own test step.

### Consequences

- Good, because a login can no longer reach a server the build did not name.
- Good, because the password outlives its call by nothing, and the host key never leaves the device.
- Good, because which data a build reaches is fixed when it is built (REL-01).
- Good, because "no logic in Swift" is a count a Linux test reads, with a register a reviewer reads.
- Good, because the lane's first device build finds a tree that already archives.
- Bad, because the core gains a direct edge on `url`, already linked through the engine.
- Bad, because the census reads Swift lexically: a construct it misreads shows as a wrong count or a
  refused name, which costs a register edit.
- Bad, because the macOS job builds one more project and one more archive per change.
- Bad, because a host key stays valid on the server until the sync password changes.
- Bad, because the codec stays hand-written: a field typed wrong is caught only by its literal bytes,
  its mutants and the simulator flow.
- Good, because the lanes' copy of the harness's steps is untouched by the app's project (D10).
- Bad, because the network names are admitted by path (D9): a listener outside the two admitted
  directories needs an amendment before the census accepts it.

### Confirmation

SPEC-347 A1 and A2 round-trip the login through the adapter against the engine's own server;
A3 and A4 hold the guard's rules and its place before the engine; A6 and A7 pin the codec's new
bytes; A8 to A11 open, list, refuse a login, store and delete the host key, and follow the size
class on both simulators; A12 holds D6; A13 holds D2's attributes, D3's placeholders and the
extended local-package rule; A14 holds D7's steps; A15 is the archive itself. The band's rows on
the pair, the guard's arms and each census rule prove the criteria observe them.
A12 also prints D9's admissions by file and name and refuses a network name anywhere else; A14
holds D10, the step "the project, generated" with its one generate line and the app's step opening
with its own.

## What would make this wrong

- The simulator refuses an unsigned app the Keychain; the app's test step then signs ad hoc for the
  simulator on its command line, and D2 is unchanged.
- The engine's bundled roots do not reach the staging server from a device; the TLS question then
  belongs to the engine's build features, and D4's guard is unchanged.
- The review screen needs a recursive message (the deck tree with its counts, #632) that a hand
  decoder cannot hold with literal bytes; generated types with a vendored runtime then win D5, and
  #632 decides it.
- The census's counts churn on every Swift change without catching logic a reviewer would refuse;
  D6's ceilings, not its exactness, are then the rule to keep.
- A dev build must reach a second server (a media host, say); D3's two settings then become a set,
  and the guard's rule applies to each.

## More Information

- SPEC-347; SPEC-334 (row 1.5; R4, R17, R19, R20); SPEC-336 (R1); SPEC-339.
- ADR-335 (the client and the Keychain), ADR-342 (one universal app), ADR-344 (internal builds),
  ADR-350 (the harness: D1, D2, D5, D9).
- #623 (the engine core), #624 (the umbrella and one library), #627 (sign-in), #628 (the sync
  server's deploy and cutover), #631 and #633 (sync), #632 (the review screen), #634 (the TestFlight
  pipeline), #650 (Swift mutation rows).
