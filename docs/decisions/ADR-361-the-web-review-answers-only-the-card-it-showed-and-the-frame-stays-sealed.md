---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# The web review answers only the card it showed, through the engine's own table, and the card frame stays sealed while media, sound and speech reach the learner

## Context and Problem Statement

SPEC-334 row 1.4 asks for the web client at Phase 1 parity: a deck list, review with the card in
a sandboxed frame, undo, the card's templates, media, Web Speech, Anki's desktop keys and the
Gamepad API, and the Home Screen. Five records already decide most of it. ADR-336 puts the engine
in a Worker over OPFS inside an 8000000-byte budget. ADR-338 shows the next interval on the answer
buttons. ADR-342's web clause maps the remote's actions from the Gamepad API and keyboard events and
holds a screen lock while a gamepad is connected during a review. ADR-352 and SPEC-341 seal the
card frame: no sandbox token, its own policy, no channel to the page or the network. SPEC-345 puts
every web engine call behind the core's dispatcher and its web column.

They leave open how the screens use the engine (which calls, and which card a rating reaches), how
the frame is sized and styled, how keys survive a tap on the card, how media, sound and speech reach
a frame that runs no script and fetches only `data:`, where a voice choice and a mapping are kept,
and how the app becomes installable without loosening the page policy. The measurements are
SPEC-350 section 1. This ADR decides those questions.

## Decision Drivers

- The card frame does not widen: no token, no policy source, no listener (ADR-352, SPEC-341 A7).
- Logic stays in the engine; the page renders and forwards gestures.
- Every engine call is a pair in the study rule's table and the core's web column (SPEC-345 A5).
- A rating must reach the card the learner saw, whatever arrives twice or out of turn.
- Anki's keys and the remote work in both of its modes, and a person can turn the single-key
  shortcuts off (WCAG 2.1.4).
- The size budget holds (465418 bytes of headroom at SPEC-338 M3).

## Decisions, and the alternatives each was chosen against

### D1. The review's calls are ordinary pairs added to both tables

The review needs eight pairs the web column lacks (SPEC-350 M10). Each is added to
`STUDY_CALLS` and to the core's `ORDINARY` with web on, and the (27,6) row the core already holds
turns web on. Each is reached by a named Worker operation over a named engine export, never by
a route that takes a pair from the page. None joins `EXEMPT`: answering, undo, burying one card
and the red flag are the study client's ordinary actions, and the owner-taps ruling's never-list
does not name them.

Chosen against:

- A Worker route to `run_method` that the page fills with a pair: rejected because SPEC-345's
  section 5 adds no such route, and the page would then choose engine calls.
- The owner-gesture token for undo, bury and flag: rejected because the token guards the writes in
  `EXEMPT`, and these are not among them; a token on every grade would make the review a sequence of
  owner taps.
- Building on WEB-1's study rule alone, before #662: rejected because the web column and the study
  calls would diverge until SPEC-345 A5 met them, and #662 rewrites the `call()` every new export
  uses.

### D2. The engine side keeps the shown card, and a rating reaches only that card

When the Worker shows a card it keeps the card's id, the scheduling states it read and the card's
flag. `rate`, `bury` and `flag` carry the card's id and are refused as `not-shown` unless it is the
kept card. `rate` answers with the kept states and the next state the rating picks. Showing replaces
the kept card, and rating, burying and undo clear it. The check is a target-independent function in
`study.rs`, tested natively.

Chosen against:

- DEV's `answer`, which reads the queue's head again when it answers: rejected because a double
  press, or an undo between show and rate, answers a card the learner never saw.
- The page holding the states and sending them back: rejected because scheduling state would live
  in the page, and the engine's own check (`card was modified`) compares states, not which card was
  on screen.
- A rating with no card id: rejected because a stale gesture could not be told from a fresh one.

### D3. One card view carries both sides, the engine's interval labels and its undo label, in the app's language

One operation returns the head card's question and answer, rendered fully through the note type's
templates with sound and speech tags stripped by the engine, its CSS, ordinal and flag, the four
labels `DescribeNextStates` gives for the kept states, the queue's counts and the undo label. The
answer side is held by the page until show answer. `open` passes the app's locale, mapped to the
engine's language codes, to the engine's `init`, so the labels are the engine's own in the
learner's language. The intervals always show, as SPEC-334 R18 and ADR-338 decide.

Chosen against:

- Formatting intervals in the page from the states: rejected because it re-implements Anki's
  rounding and its translations in a second place that can drift.
- A separate reveal operation: rejected because it adds a round trip at the moment the learner
  waits, and the answer is no secret from the page, which already holds the card.
- Reading Anki's preference that hides the intervals: rejected because R18 shows them; a switch
  that hides them is a later parity setting (#611).

### D4. The frame fills the card area and scrolls itself, and its body carries the card's classes

The card frame takes the space between the counts and the answer controls, and a long card scrolls
inside it. `frameDocument` gains one argument, the body's classes, accepted only as
`card card<n>` with the night-mode classes, and its re-parse check confirms the body carries
exactly those. A refused card shows a translated message in the page, and its answer controls stay
usable.

Chosen against:

- Sizing the frame to its content: rejected because only the frame knows its height, and telling
  the page needs a script and a message listener, which SPEC-341's census refuses.
- A wrapper element built by the page around the card: rejected because the note type's CSS styles
  `.card` as the body, so a wrapper leaves the frame's background and height unstyled.
- Any body attribute the caller names: rejected because it opens an input to the frame document
  that the census does not hold.

### D5. The review consumes #663's input modules; one handler serves every source; focus comes back to the page

The default map lives where SPEC-343 put it, `src/lib/remote/mapping.ts`, and the review binds
`readKey`, the `GamepadReader`, `resolve` and `sideAfter` unchanged. A key, a gamepad button, the
stick and a click call one handler, so each source reaches the same action. A switch, stored per
device, turns the single-character keys off. After every action, and when a pointer moves focus
into the card frame, focus returns to the review region, because a key pressed while the frame
holds focus is dispatched in the frame's scriptless document (UI Events 3.5.5). A Tab into the
frame is left there, so a keyboard user can scroll a long card. The screen lock follows ADR-342's
condition through #663's `WakeLockHolder`.

Chosen against:

- A second map in the study code: rejected because SPEC-343 gives #630 the review's use of these
  modules, and two maps drift.
- Keys bound only while a review region holds focus: rejected because the remote's keyboard mode
  and a tap on the card leave focus elsewhere, and the keys would stop without a sign.
- Leaving focus where a tap put it: rejected because the keys then reach the frame, where nothing
  listens.
- `pointer-events: none` on the frame: rejected because a long card could no longer be scrolled by
  touch.

### D6. Media reach the frame as `data:` URLs the Worker reads from OPFS (part 2)

The engine names each side's media files (41,8). The Worker reads each named file from the OPFS
media directory that the web sync screens fill (#631), and returns a `data:` URL for a type on an
image and audio allow-list, within a per-file and a per-card bound. `frameDocument` replaces a
`src` that names a returned file in its inert parse, before its re-parse check. The frame policy
already admits `data:` images, media and fonts, so nothing widens.

Chosen against:

- `blob:` URLs: rejected because the frame policy would gain `blob:`, a source no test holds today.
- A route that serves media, through a service worker or the server: rejected because it gives the
  frame a network path, and the browser's service worker is web push's (#640).
- `allow-same-origin`, so the frame reads the page's storage: rejected by ADR-352.

### D7. Sound plays in one page-owned audio element (part 2)

The engine extracts each side's sound tags (27,3). The page plays them, in order, in one audio
element it owns: on show and on reveal once the page has user activation, and at any time from the
replay control or the remote's replay. A blocked play leaves the replay control. A sound tag naming
a video plays its audio track.

Chosen against:

- Audio elements inside the card frame: rejected because the sandbox's automatic-features flag
  stops them playing themselves and no script can start them, so the frame's own media controls
  would be the only way, card by card.
- Web Audio: rejected because it needs the same activation and adds decoding code for no gain.

### D8. A voice choice is stored on the device, per language (part 2)

A speech tag's language is converted to BCP 47. The voice is the device's stored choice for that
language, else a voice the tag names that the device has, else the language's default (a null
voice). The voice picker lists the device's voices for the card's languages, and stores the choice
in the browser's local storage, keyed by language.

Chosen against:

- The collection's configuration: rejected because a voice is a fact about one device, the config
  syncs it to devices that lack the voice, and the write would need a configuration pair in the web
  column.
- The engine's voice list (27,1): rejected because the engine lists only one desktop platform's
  voices and refuses on every other target, the browser's included
  (`rslib/src/card_rendering/tts/other.rs:8-10` at the pinned revision).
- The language's default voice alone: rejected because a learner with several voices for one
  language could not choose, as the desktop and AnkiMobile let them.

### D9. Installable by a manifest, with no service worker and no policy change (part 2)

A manifest (`name`, `short_name`, 192 and 512 pixel icons, `start_url` at the deck list,
`display` `standalone`) and a touch icon are linked from `app.html`. The page policy holds no
`default-src`, so neither the manifest nor its icons need a directive, and `csp.test.ts` stays as it
is. The release stages the module beside the app at `/engine/`, held to the size gate.

Chosen against:

- A service worker that caches the shell and the module: rejected because the browser's service
  worker arrives with web push (#640), and a cache that serves an old module beside a new page is a
  failure this delivery would own.
- A `manifest-src` directive: rejected for this delivery because the page policy's exact map is the
  security tests' fixture, and no source needs it.
- Serving the module from another origin: rejected because the page would admit a script source
  and a connection it does not need.

### D10. The mapping is stored per device and per mode (part 2)

SPEC-343 gives #630 the mapping screen and the persisted mapping. The mapping is stored in the
browser's local storage, with a default for the gamepad mode and one for the keyboard mode, and the
key switch moves onto the same screen.

Chosen against:

- The collection's configuration: rejected because each device pairs its own remote.
- The server: rejected because it needs sign-in outside Telegram (#627) and a network path for a
  local preference.

### D11. Two pull requests

Part 1 builds the engine surface, the Worker operations, the deck list, the review with its answer
buttons, undo, bury, flag, keys, gamepad and screen lock, and the study suite. Part 2 builds media,
sound, speech, installability and the mapping screen.

Chosen against:

- One pull request: rejected because the delivery touches two Rust crates, the Worker, two routes,
  seven message files, a CI job and the release, and each of part 2's surfaces brings its own device
  reading.
- A pull request per surface: rejected because each later surface needs the review loop part 1
  builds, and a split smaller than two leaves a review that cannot be tested end to end.

### D12. Media through the core's face, asked twice, from one flat directory (part 2; amends D6)

D6's `data:` URL built by the Worker and its `src` replacement in `frameDocument` are superseded.
The core holds one cap pair and one closed type table that both clients read, and builds each
face's `data:` URLs itself (ADR-359 D1). One new export answers both faces of the card on screen
only, through the check `rate` makes. It reads media through a reader over the files it is given,
which also records each name the core asks for that the files lack, with the limit the core asked
for, and it answers both faces and that list. The core reads synchronously and the browser's file
reads do not, so the Worker asks twice: once with no files, then with each named file's first
`limit` bytes; the second answer is the reply. The media directory is `deck-streak-media` at the
origin's root, flat, each file under the name the engine stores, read through the Worker's
injected storage; an absent file or directory is absent. `open()` keeps its empty media folder, so
the engine never reads the directory itself. No pair joins a table, and the frame, its policy and
the census are unchanged, since the policy already admits `data:` images and media.

Chosen against:

- Two copies of the media rules, one in the Worker: rejected because two clients would then
  decide a cap or a type in two places, the shape ADR-359 D1 rejected as a cap pair and a type
  table in each client.
- A reader over the storage pool, so the engine reads the directory itself: rejected because the
  pool's export reads a whole file into the module's memory before any cap applies, and a module's
  memory never shrinks.
- `blob:` URLs in the frame: rejected for D6's reason, since the frame policy would gain a source no
  test holds.
- The core's media and sound pairs, (41,8) and (27,3), called from the page: rejected because
  ADR-359 D2 rejected (27,3) as an adapter pair, and the face call already answers both.

### D13. The voice is the device's choice for the language, and the rate is the core's (part 2; amends D8)

A speech clip carries its text, its language and the native platform's rate, the engine's speed
times its default of 0.5 (SPEC-348 P3); it carries no voice the tag names. The page speaks it in
that language with the device's stored voice for the language, else the language's default voice,
at the clip's rate divided by 0.5, one named constant. The choice is stored per language under one
local-storage key, and a storage the browser refuses keeps the choice for the page. A tag's named
voice is #666's.

Chosen against:

- The voice the tag names, ahead of the default: rejected because the core's clip does not carry
  it, and adding it is the parity work #666 holds.
- The clip's rate passed to the browser unchanged: rejected because the browser's normal rate is 1
  and the native platform's is 0.5, so every card would speak at half speed.

### D14. The release's module staging is a follow-up (part 2; amends D9)

D9's release step, which stages the module at `/engine/` and holds it to the size gate, and its
criterion A29 move to #685. Part 2 edits no workflow file.

Chosen against:

- The release step in this pull request: rejected because a workflow change costs every open pull
  request a merge round, and the release workflow carries rows of its own that the step would move.

### D15. The mapping is a parameter of #663's readers, stored per device (part 2; amends D10)

#663's key reader and gamepad reader each gain one trailing parameter that defaults to today's
map, so #663's callers and tests are unchanged. The study input reads the stored mapping for each
mode and passes it in. The mapping screen is the route `/study/mapping`, and it also binds the key
switch, which the review keeps.

Chosen against:

- A second copy of the readers in the study screens: rejected because the remote's rules would
  then live in two places.
- Moving the key switch off the review: rejected because a learner who turned the keys off needs
  the switch where the keys act.

### D16. A plain generated icon (part 2)

The 192 and 512 pixel icons and the 180 pixel touch icon are plain squares, made once by a short
standard-library command quoted in the red-first record; no generator ships.

Chosen against:

- A generator in the build: rejected because it adds a step and a dependency for three files that
  do not change.
- Waiting for drawn art: rejected because none has been supplied, and the install criteria need an
  icon now.
- Committed PNG files: rejected because the public scrub refuses every binary file in the tree and
  in its history.

Amended before the pull request was opened: the icons are built from text at build time, so no
icon is made once or committed, and the tree and its history hold no binary file.

- Chosen: an endpoint at each icon's path that the build prerenders into the file the site serves
  there, encoding the plain square with the platform's own deflate (`web/app/src/lib/icon.ts`).
  `vite build` already runs SvelteKit's prerender pass, so the endpoints add neither a build step
  nor a dependency, the two reasons a generator in the build lost above.

The endpoint's answer names no content type: the build writes it to the file at the icon's path
whatever its type, and the host types the file by its extension.

- Naming the PNG media type in the answer: rejected because a media type named in the app is a copy
  of the core's one closed type table (D12), which SPEC-350 A24's census refuses.

## Decision Outcome

The web review crosses the core's dispatcher with ordinary pairs named in both tables, answers only
the card it showed with the states it read, and shows the engine's own interval and undo labels in
the learner's language. The card frame keeps its sandbox, its policy and the census; it gains only
the card's body classes. Input comes from #663's modules through one handler, with a key switch
and focus kept in the page. Media, sound and speech arrive in part 2 through `data:` URLs and
page-owned players, and the app becomes installable by a manifest alone.

### Consequences

- Good: no engine call, no frame channel and no page policy source is added that a test does not
  name.
- Good: a double press, an undo between show and rate, or a stale remote event cannot rate a card
  the learner did not see.
- Bad: the installed app needs the network to start until the browser's service worker exists
  (#640).
- Bad: a card that needs its script (MathJax, a hint toggle) or a typed answer renders without it
  (#651, #611).
- Bad: media inlined as `data:` URLs cost about a third more memory than their bytes, which the
  per-card bound limits.

## What would make this wrong

- A device reading shows that gamepad input stops while the card frame holds focus even with the
  focus return (#629): then the review needs another way to keep focus in the page.
- The size gate fails with the new exports: then the exports are wrong-shaped, since the backend
  methods they reach are already linked.
- The owner rules the review's grades are owner gestures: then D1's ordinary rows move to `EXEMPT`
  with the token.
- A learner's media regularly exceed the per-card bound: then the bound, or `data:` inlining, is
  revisited, with the frame policy unchanged.

## More Information

- SPEC-350 (the measurements, requirements and acceptance criteria), and
  `docs/schematics/web-study-screens.md`.
- ADR-336, ADR-338, ADR-342, ADR-352; SPEC-338, SPEC-341, SPEC-343, SPEC-345.
- UI Events 3.5.5 (key event target); HTML's activation-triggering input events and the sandboxed
  automatic-features flag; the Gamepad, Screen Wake Lock and Web Speech API specifications; the
  install criteria and autoplay policies cited in SPEC-350 section 1.2.

## Amendment: an answer is held by a token, and a rating is one of two (SPEC-365)

ADR-376 amends D1. D1 rejected "a token on every grade", and its "What would make this wrong" foresaw
grades moving into `EXEMPT` with the owner-gesture token. Neither is what was built: AnswerCard
leaves the ordinary pairs for the `ANSWERED` set, which `rate` reaches only through
`Dispatcher::run_answer` with an `OwnerAnswer` minted from the kept card and the pressed grade. The
wire's ratings are 1 for Again and 3 for Good; 2 and 4 are refused by name. D2 stands: a rating still
reaches only the card the review showed.

- A token of its own for answering, outside `EXEMPT`: chosen because answering is not a never-list write.
- The owner-gesture token for every grade, with the rows in `EXEMPT`: rejected because it would put every grade under the owner-taps ruling's conditions.
- Keeping the grade pairs ordinary: rejected because three doors would still record a grade with no press behind it.

## Amendment: a token for undo, for the review's own last answer only (SPEC-371)

ADR-382 amends D1, for undo only. The rest of D1 stands.

- **D1 (`:37-55`).** D1 rejected a token for undo (`:50-51`). For the undo of the review's own
  last answer that rejection gives way: Undo is an exempt write behind the owner's gesture, checked
  at the write against a record the Worker kept when the answer was made (ADR-382 D1 to D4). Bury
  and flag stay ordinary calls, as D1 decided, and Undo no longer reverts them.
- **D2 (`:57-72`)** is unchanged in effect: a confirmed undo still clears the kept card.

## Amendment: the release builds, gates and stages the web engine's module (#685), and the alternatives each decision was chosen against

D9 decided that the release stages the module beside the app at `/engine/`, held to the size gate,
and D14 moved that step and its criterion, A29, to #685. These decisions are that step. SPEC-350
section 12 states it and section 14 holds A29's tests. D9 and D14 stand as written.

### D17. The release builds the module with CI's own steps, copied byte for byte

After its Node setup, the release job runs the five steps CI's `web-engine` job runs to make and
measure the module: the pinned toolchain and its wasm32 target, the bindings generator and the
optimiser downloaded at their pinned releases with each digest checked, the C compiler and archiver
SQLite's source needs, `scripts/web-engine-build.sh` with CI's compiler, archiver and output
directory (`target/web-engine`), and the size gate (D19). Each is CI's step byte for byte, and a
test holds each pair equal, so the release ships a module built the way the module CI's browser
tests ran over was built. The release job already sets `CARGO_INCREMENTAL` to `0` as that job does
and sets no `RUSTFLAGS`, so the build's only wasm32 settings stay on the build script's command line
(ADR-348). The release restores no cache, and builds the module from the tag's tree alone.

Chosen against:

- A separate job that builds the module and hands it to the release job as an artifact: rejected because it adds an upload and a download to the path that publishes, the release workflow's two jobs are pinned by its tests, and one tag's build would be split across two runners.
- The module CI's `web-engine` job built for the tag's commit, fetched from that run: rejected because no CI run is guaranteed at a tag's commit, and the release builds once, from the tag (ADR-062).
- A shared install script that both workflows call: rejected because it edits `ci.yml`, whose web-engine steps CI's own checks pin by their text, and every open pull request touching `ci.yml` would take a merge round.
- Other build flags or another compiler in the release: rejected because the release would ship a module built otherwise than the one CI's browser tests ran over.
- Restoring CI's Rust cache before the build: rejected because a cache another run saved could feed the release's build, and the release workflow promises that no step uses one.

### D18. The stage runs after the app's build, so the tarball carries the module at `web/engine/`

After the app's build, the release runs CI's stage step, `bash scripts/web-engine-stage.sh`, which
copies the module and its bindings from `target/web-engine` into `web/app/build/engine/` and
refuses, naming the file, when either is missing (SPEC-350 A22). The tarball step is unchanged: its
copy of `web/app/build` into `web/` carries both files to `web/engine/`, and the manifest, the
digests and the attestation cover them. The app's Worker loads its bindings and module from
`/engine/` at the origin's root (`worker.ts` `ENGINE_BASE`), so an origin that serves the release's
`web/` at its root answers that URL, as the study suite's server answers it over the staged build.
The stage follows the app's build and precedes the tarball, the order CI's `web-engine` job runs
them in.

Chosen against:

- Copying the two files into the tarball's `web/engine/` inside the tarball step: rejected because it repeats the stage's checks in a second place and edits the tarball step, whose text the second release path's tests and rows hold.
- Staging at another path, or under a hashed name: rejected because the Worker's URL is fixed at `/engine/` and pinned by `worker.test.ts`, so the published Worker could not load a module anywhere else.
- Copying the module into `web/app/static/engine/` before the app's build: rejected because the build's inputs would hold generated files in a tracked source directory, which a local build would leave behind untracked.

### D19. The size gate stops the release before the draft

The release runs CI's size gate, `python3 scripts/web-engine-size.py`, over the module and the
bindings it built: after the Node setup, because the gate measures brotli with Node and reads VOID
without it, and before the app's dependencies, the stage, the tarball and the draft. The bound is
ADR-336's 8000000 bytes `gzip -9` for module plus bindings, the script's `BUDGET` (SPEC-350 M12).
The gate exits 1 over the bound and 2 when it cannot measure. Either exit fails the step; the step
carries no `continue-on-error` and its command no `||`, and no later step carries an `if`, so a
refused module stops the job and no draft release exists.

Chosen against:

- A warning in the job's summary, with the release published anyway: rejected because #685 asks the release to hold the module to the bound, and a warning publishes the module it warns about.
- Gating the staged copy under `web/app/build/engine/` after the stage: rejected because the gate would wait on the app's install and build for the same bytes, and a refusal would come later than it needs to.
- A second budget for the release alone: rejected because the bound has one home, ADR-336 and the script's `BUDGET`, and two bounds would drift apart.

### D20. Three tests decide A29, and their red is read in CI

A29 is decided by three tests in `scripts/tests/test_release_workflow.py`, in the class
`TheReleaseCarriesTheWebEngine`. The first runs the release's own steps from the app's build to the
draft under bash in a planted tree, and reads the tarball they write: the module and the bindings
sit at `web/engine/`, the manifest holds each one's digest, and the same steps without the stage
pack the app alone. The second holds each of the release's six web engine steps equal to the CI
step that runs the same command, in CI's order. The third holds the gate between the module's
build and the stage, as one command, with no `continue-on-error` on it and no `if` after it. The
tests plant fixed bytes, so each digest is a known input's; they add one subprocess site, listed in
`test_ci_workflows.py`'s census, and import nothing new but `hashlib`. The module runs in CI only,
so each red is read by name in CI's hygiene job.

Chosen against:

- Building the real module inside the test: rejected because the wasm32 release build is the costliest step of CI's web-engine job, the hygiene job runs every test module, and that build is already proved in the web-engine job.
- Reading a published release's tarball: rejected because the test would need the network and a release that already exists, so it could not be red before the change.
- A text-only test of the workflow's lines: rejected because A29's claim is about the tarball the release's own steps assemble, and text alone would pass a stage that writes somewhere else.

### D21. An insert-only amendment of SPEC-350 and this record

#685 is SPEC-350's own follow-up: R17's last sentence, A29, D9 and D14 already name it. So the
release's change is decided by new last sections of SPEC-350 (12 to 14) and by D17 to D22 here,
and no earlier line of either changes. The release's schematic gains a section that draws the
module from its build to `/engine/`.

Chosen against:

- A new SPEC and ADR for the release step: rejected because A29 and R17 already state the criterion and name its owner, and a second record would split one criterion across two SPECs.

### D22. The release job's bound rises to 150 minutes

The release job already builds the daemon and the sync server from cold; the module adds a cold
wasm32 release build of the engine, for which CI's `web-engine` job is given 20 to 60 minutes with
its browser tests (`test_ci_workflows.py` `WEB_ENGINE_TIMEOUT_MINUTES`). The job's
`timeout-minutes` rises from 90 to 150, that band's top above the old bound. Rows S19011 and S19016
anchor on the bound's line and move with it, each keeping its mutant and its killer.

Chosen against:

- Keeping 90 minutes: rejected because no measured release run holds the module's build within it, and a run cut at its bound leaves a tag with no release that only a new patch tag can recover, since every run of a tag reads the tag's own workflow.
- A bound derived at the cut from measured release runs: rejected because no release has yet run with the module, and the bound must be in the tag's tree before its first run.

What would make these wrong: a tag whose module differs from CI's in behaviour, though both were
built by the same steps at one commit, would call for the browser tests in the release (SPEC-350
section 13); and a release run that ends near its new bound would call for D22's bound to be
measured again.

## Amendment: the token for undo also reaches the review's last bury or flag (SPEC-383)

ADR-397 amends D1, for undo only, after SPEC-371's amendment. The rest of D1 stands.

- **D1 (`:37-55`).** SPEC-371's amendment let the undo of the review's own last answer through the
  owner's gesture, and said that Undo no longer reverts a bury or a flag. ADR-397 widens that reach
  to the review's last bury or flag, through the same gesture, checked at the write by the record's
  kind (ADR-397 D3). Bury and flag themselves stay ordinary calls, as D1 decided.
- **D2 (`:57-72`)** is unchanged in effect: a confirmed undo still clears the kept card.
