# SPEC-392: the iPhone and iPad card view strips every link element, and a followed link opens no connection

- **Issues:** #664 and #677, both closed by this delivery. **Context(s):** `ios-harness` (`ios/`).
- **Decided by:** ADR-406 (this SPEC's own: L14, the link strip, in the factory's build; no new
  layer for a followed link; one delivery for both issues; no formal model or proof; two pushes).
  It builds on ADR-360 (L1 to L7), ADR-366 (the switch, its read-back, L8) and ADR-372 (L10 to
  L13), and amends none of them in place.
- **Schematic:** `docs/schematics/card-frame-channels.md`, amended INSERT-ONLY by an appended
  section 9 that places L14 in the card's path from the note to the view on each client, records
  #677's closure, and states the readings this SPEC predicts.
- **Status:** proposed.

## 1. The problem, measured

Every fact below was read at commit `164ac206` with `git show 164ac206:<path>` on the path it
cites.

- **The native path hands a card to the view as it came.** `makeCardWebView(html:)` compiles the
  rule list and builds (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:25-28`).
  `build` (`:40-49`) loads the card at `:47`, `load(html, into: view)`, and `load` (`:55-65`)
  prefixes L12's policy (`:59`) and hands the string to the view with no base URL (`:64`). Nothing
  on the native path removes an element.
- **Every `link` element in the card view is the card's own.** The engine's document wrapper
  writes no `<link`: `crates/ffi/tests/face.rs:136-153` counts it 0 for its fixture.
- **L3 already holds the hint.** The schematic records `preconnect` and `shadow-link` at one
  connection from the reference view and none from the card view, opened by removing L3 alone
  (`docs/schematics/card-frame-channels.md:241-242`). A strip is depth beside L3, not the only
  hold.
- **The planted cards that carry a static opener.** `ios/CardProbeTests/Planted.swift` holds a
  static `<link` at `:166`, `:169` (two), `:171`, `:173`, `:174` and `:177`, inside the cards
  `PLANTED` builds (`:133`, `:136-232`), and at `:506`, in `LOOKUP` (`:504`), which `PLANTED` does
  not hold. `RENDER` (`:258`) and `PERMITTED` (`:581`) hold none, and no scripted card's source
  holds the five characters (`createElement('link')` at `:349-351` and `:508`). `UNOBSERVABLE`
  (`:250-255`) names `dns-prefetch`, `prefetch` and `nav-data`.
- **#677's channel is closed in the tree, and the issue is open.** The suite asserts zero
  connections for every planted card from the card view
  (`ios/CardProbeTests/PlantedCardTests.swift:582-583`), with the control that `nav-self` and
  `nav-blank` each open one from `shippedWithout(.L10)` (`:584-594`), recorded green
  (`docs/red-first/SPEC-361.md:22-23`) and described in the schematic (`:461-465`). The
  schematic's section 6 still carries the residual (`:232-234`), as does ADR-360 D8.
- **The web card frame already strips.** W3 removes `link`, `meta`, `base` and `template` before
  the frame (`web/app/src/lib/card/frame-document.ts:17`, `:38-51`), proved by
  `frame-document.test.ts:49` and, for a card whose markup re-parses differently, by its refusal
  (`:75-81`).
- **The web suite cannot see a followed link's bare connection.** It counts requests by path and
  points `nav-self` at its HTTP listener (`web/app/tests-card/listeners.ts:45-55`,
  `web/app/tests-card/planted.ts:160`).

## 2. Requirements

- **R1.** The factory's `build` calls `load(LinkStrip.stripped(html), into: view)` at
  `CardWebViewFactory.swift:47`, and no other card load exists in the factory.
- **R2.** `LinkStrip.stripped(_:)`, in `ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift`,
  renames every `<link` (its four letters in any ASCII case), wherever it stands, to `<wbr`, keeps
  every other byte, and returns a card that holds none byte for byte. Its doc comment names what
  it does NOT stop: a link a script adds (L3 holds its hint); a link in a document a frame loads
  (L5); any other element's load (L3, L12); and it says that the five characters are renamed in
  text too, in a text area, a title, raw text, a comment or an attribute value.
- **R3.** `CardLayer` names L14 in place: `:9` becomes
  `case L1, L2, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13, L14`. Its doc comment (`:4-7`) is
  amended in place, four lines still, to add "then L14, the link strip (SPEC-392, section 9)", and
  `build`'s doc comment (`:38-39`) is amended in place to name L14. L14 is NOT in `required`,
  `CONTROLS`, the read-back (`present`, `:129-175`) or `LAYERS`. In `CardScripts.swift:10` and
  `CardScriptsTests.swift:17`, the words "every layer but L2, which is the verdict itself." are
  replaced in place so that they name L14 too, on the same line.
- **R4.** The change is line-neutral where other documents cite a line:
  `CardWebViewFactory.swift:200`, `:204` and `:210`, `CardScripts.swift:7`, and
  `scripts/tests/test_card_web_view_layers.py:178` and `:504` keep their content at their numbers.
- **R5.** The probe composes the strip by name. `Variant` gains `case referenceWith(Set<CardLayer>)`,
  whose layers are the set and which the factory does not build. `show` composes
  `let handed = layers.contains(.L14) ? LinkStrip.stripped(html) : html` once, above `:461`, and
  hands `handed` to the three loads the factory does not build (`:464`, `:468`, `:470`). `show`
  gains `countLinks: Bool = false`; when it is true, after `reading.loaded` (`:474`), it counts the
  `link` elements in the document, in every `template`'s content and in every open shadow root
  into `reading.links: Int?`, nil when the answer is not a number. `Planted.swift` appends
  `let LINKED: Set<String> = ["stylesheet", "preload", "prefetch", "preconnect", "dns-prefetch", "shadow-link"]`
  after its last line.
- **R6.** No new layer holds #677. L10 and L11 hold a followed link as SPEC-361 built them, A7's
  reading on both simulators closes the issue, and the schematic's section 9 records the closure.
- **R7.** The web card frame is unchanged: W3 already removes every `link`.
- **R8.** The rows of section 8: S39200 and S39201 in `scripts/mutation-rows.d/S39200-S39299.json`,
  and SW39200 to SW39204 appended to `ios/CardIsolation/swift-mutants.json`.
- **R9.** `docs/red-first/SPEC-392.md` holds, per criterion, one `red at <sha>: <failure>` line and
  one `green at <sha>` line, or one `not red: <why>` line. A4's red and green are measured where
  the census runs. The Swift reds of A1, A2, A5 and A6 are read in CI by job name on the first
  push, which carries the tests and the identity stub of `LinkStrip.stripped(_:)`, and each is
  written with the run, job and step it was read in. `changelog.d/card-view-network-residuals-392.md`
  holds an `### Added` list in public text.

## 3. Acceptance criteria of SPEC-392

`$IPHONE_SIM`, `$IPAD_SIM` and `$SIM_OS` are the `harness` job's own. A simulator criterion is
green only when its test passed on both destinations in one CI run.

| id | criterion | decided by | red first by |
|---|---|---|---|
| A1 | every `<link` opener is renamed to a `<wbr` opener, in any ASCII case and wherever it stands, and every other byte is kept, over A1's goldens below | `swift test` `LinkStripTests/test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case` | the identity stub, read red in CI's `card-isolation` job on the first push |
| A2 | an opener at either end of the card is renamed, over A2's goldens below | `swift test` `LinkStripTests/test_an_opener_at_either_end_of_the_card_is_renamed` | the identity stub, read red in CI's `card-isolation` job on the first push |
| A3 | markup that holds no `<link` opener is returned byte for byte, over A3's goldens below | `swift test` `LinkStripTests/test_markup_holding_no_link_opener_is_returned_byte_for_byte` | not red: the identity stub passes it; it pins the strip and kills SW39204 (mutation coverage) |
| A4 | the factory hands every card through the link strip: the census reads no problem in the tree, and refuses by name each planted tree that breaks R1, R2's file and function, or R3's L14 | `scripts/tests/test_card_web_view_layers.py` `TheFactoryStripsEveryCard/test_the_factory_hands_every_card_through_the_link_strip` | measured red where it runs, at the census's own commit: the factory hands a card past the strip, the strip's file and `stripped(_:)` are missing, and `CardLayer` names no L14 |
| A5 | no linked card reaches the card view with a `link` element: every planted card shown from the reference view and the card view with `countLinks: true` reads 0 from the card view, and the cards whose reference count is above 0 are exactly `LINKED` | `xcodebuild test` `PlantedCardTests/test_a_linked_card_reaches_the_card_view_with_no_link_element` | the identity stub, wired at `:47`, read red in CI's `harness` job on each destination on the first push |
| A6 | the link strip alone holds every observable linked card: with only L14 on, each of `LINKED.subtracting(UNOBSERVABLE.keys)` reaches nothing and opens no connection, while the reference reaches | `xcodebuild test` `PlantedCardTests/test_the_link_strip_alone_holds_every_observable_linked_card` | the identity stub, read red in CI's `harness` job on each destination on the first push |
| A7 | every planted card reaches from the reference view and nothing from the card view, zero connections included, with the `nav-self` and `nav-blank` control from `shippedWithout(.L10)` | `xcodebuild test` `PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view` | not red: SPEC-361's criterion, kept; its zero-connection reading on both simulators is #677's closing reading |
| A8 | removing one layer opens exactly its own channels | `xcodebuild test` `PlantedCardTests/test_removing_one_layer_opens_exactly_its_own_channels` | not red: SPEC-349's criterion, kept; L14 is in no base variant |

**A1's goldens** (input, then the expected output where the rename shows):
`<link rel="stylesheet" href="x">`; `<LINK REL=preconnect HREF=x>`; `<LiNk\trel=preload>`;
`<link\nrel=prefetch/>`; `<link>`; a `template shadowrootmode="open"` holding one;
`<svg><link href="x"/></svg>`; `<linkx>` to `<wbrx>`; two in one card; `<link<link>` to
`<wbr<wbr>`; one inside an `iframe srcdoc` value; one inside a `textarea`; `é<link>ü` to
`é<wbr>ü`. Each expected output is the input with every opener's five characters replaced by
`<wbr` and nothing else changed, written out literally in the test.

**A2's goldens:** `<link` to `<wbr`; `<p>a</p><link` to `<p>a</p><wbr`; `<LINK` to `<wbr`.

**A3's goldens**, each returned unchanged: the empty string; `<p>a card</p>`;
`<svg><use xlink:href="#a"/></svg>`; `<p>unlink and blink</p>`; `&lt;link rel=preconnect&gt;`;
`</link>`; `< link>`; `<l ink>`; `<lin\u{212A}>`; `<l\u{0130}nk>`; `<!-- a comment -->`.

**A4's census** is the class `TheFactoryStripsEveryCard`, inserted after
`test_card_web_view_layers.py:531` and before `if __name__`. Its `problems(root)` returns the
problems by name, and its behaviour assertion, `problems(REPO) == []`, comes first. Then each
planted tree is refused by its name: a factory that loads `load(html, into: view)` ("the factory
hands a card past the link strip"); the strip call only in a comment, read through the module's
`code_of` (the same name); no `LinkStrip.swift` ("the link strip's file is missing"); no
`public static func stripped(_ html: String) -> String` ("the link strip defines no
stripped(_:)"); and a `CardLayer` without L14 ("CardLayer does not name L14"). The good planted
tree reads `[]`.

**A5's reading** shows every card of `PLANTED` from `.reference` and `.shipped`, with
`countLinks: true` and `window: nil`. Its behaviour assertion comes first: no card view count
differs from 0, and a nil counts as differing. Its control: the cards whose reference count is
above 0 equal `LINKED`. Then `examined("planted cards", PLANTED)`.

**A6's reading** takes each card of `LINKED.subtracting(UNOBSERVABLE.keys).sorted()`
(`preconnect`, `preload`, `shadow-link` and `stylesheet`), reads it through the existing
`reference(_:_:)` helper, then from `.referenceWith([.L14])` with `window(took)`. Its behaviour
assertion comes first: no card reached or opened a connection with only L14 on. Its control:
every reference reached.

Every test that enumerates prints `examined N` and fails when N is 0, after its behaviour
assertion.

```acceptance
A1: swift test --package-path ios/CardIsolation --filter CardIsolationTests.LinkStripTests/test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case
A2: swift test --package-path ios/CardIsolation --filter CardIsolationTests.LinkStripTests/test_an_opener_at_either_end_of_the_card_is_renamed
A3: swift test --package-path ios/CardIsolation --filter CardIsolationTests.LinkStripTests/test_markup_holding_no_link_opener_is_returned_byte_for_byte
A4: python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_the_factory_hands_every_card_through_the_link_strip
A5: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_linked_card_reaches_the_card_view_with_no_link_element
A6: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_the_link_strip_alone_holds_every_observable_linked_card
A7: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view
A8: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_removing_one_layer_opens_exactly_its_own_channels
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift` | `ios-harness` | added: L14, `LinkStrip.stripped(_:)` (R2), with section 8's anchored lines |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | `ios-harness` | changed, line-neutral: `:4-7` and `:38-39` doc comments, `:9` L14, `:47` the strip call (R1, R3, R4) |
| `ios/CardIsolation/Sources/CardIsolation/CardScripts.swift` | `ios-harness` | changed in place: `:10`'s words name L14 (R3) |
| `ios/CardIsolation/Tests/CardIsolationTests/LinkStripTests.swift` | `ios-harness` | added: A1 to A3 |
| `ios/CardIsolation/Tests/CardIsolationTests/CardScriptsTests.swift` | `ios-harness` | changed in place: `:17`'s words name L14 (R3) |
| `ios/CardIsolation/swift-mutants.json` | mutation | changed: SW39200 to SW39204 appended (R8) |
| `ios/CardProbeTests/PlantedCardTests.swift` | `ios-harness` | changed: `referenceWith`, `handed`, `countLinks` and `reading.links` (R5); A5 and A6 added |
| `ios/CardProbeTests/Planted.swift` | `ios-harness` | changed: `LINKED` appended after the last line (R5) |
| `ios/swift-roles.json` | `ios-harness` | changed: `LinkStrip.swift` registered as `isolation` and `LinkStripTests.swift` as `test`, each in sorted position (the closed register every tracked Swift file under `ios/` owes) |
| `scripts/tests/test_card_web_view_layers.py` | CI | changed: `TheFactoryStripsEveryCard` (A4) inserted after `:531` |
| `scripts/mutation-rows.d/S39200-S39299.json` | mutation | added: S39200 and S39201 (R8) |
| `docs/specs/SPEC-392-the-iphone-and-ipad-card-view-strips-every-link-element-and-a-followed-link-opens-no-connection.md` | docs | added: this SPEC |
| `docs/decisions/ADR-406-a-link-strip-renames-every-link-start-tag-before-the-card-view-and-the-link-activation-refusal-holds-a-followed-link.md` | docs | added |
| `docs/schematics/card-frame-channels.md` | docs | changed: section 9 appended, insert-only |
| `docs/red-first/SPEC-392.md` | docs | added (R9) |
| `changelog.d/card-view-network-residuals-392.md` | docs | added (R9) |

## 5. What this does NOT cover

- No `meta`, `base` or `template` strip on iPhone and iPad: L3, L5 and L12 hold what each could
  open, and #664 names `link` alone (#653).
- A link a script adds, and running card scripts: the strip renames text before the load, and L3
  holds the hint a script-added link would make (#651).
- The web card frame, which W3 already strips before the frame (#651).
- The web suite's blindness to a followed link's bare connection: it counts requests by path and
  points `nav-self` at its HTTP listener (`listeners.ts:45-55`, `planted.ts:160`) (#761).
- Proof on a device: every reading here is a simulator's (#616).
- The card view's place and its iPad layout (#632).

## 6. Risks

- **A renamed link ends the card's head.** What follows it is placed in the body, where `style`,
  `meta`, `base` and `title` are still processed; L12's policy stays first, because `load`
  prefixes it ahead of the stripped card. A card's own policy `meta` after a renamed link stops
  applying; it is no layer of the card view. A5, A7 and A8 read every planted card through it.
- **Text is renamed too.** A card that shows the five characters in a text area, a title, raw
  text, a comment or an attribute value shows `<wbr`; A1's goldens pin it, and `&lt;link`, which a
  card writes to show the text, is left (A3).
- **A first-push red for another reason.** A Swift red read in CI whose failing line is not its
  criterion's failure is not a red-first record. It is quoted by name, and the second push waits
  until the red is explained; a push beyond the two is decided before it is made.
- **A line shift breaks the stride citations.** R4 keeps every cited line in place;
  `test_threat_model.py` judges them in CI's `hygiene` job.
- **A sibling lands first.** #752 (SPEC-361's file and a workflow) or #748 (the stride schematic)
  may land before this delivery; each shared path is re-measured at the cut.

## 7. Formal

FORMAL: not applicable - the strip is a pure transform run synchronously in the factory's build, adding no actor, shared state, timer or write path, and no formal cover can name a Swift item; of 219 cover lines under formal/ at 164ac206, none names a path in section 4.

ADR-406 D4 records the alternatives this was chosen against.

## 8. Mutation rows

The script rows, in `scripts/mutation-rows.d/S39200-S39299.json`, each killed by A4's census:

- **S39200-THE-FACTORY-STRIPS-EVERY-CARD.** Path
  `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift`; find
  `load(LinkStrip.stripped(html), into: view)`; replace `load(html, into: view)`; killer
  `test_card_web_view_layers.TheFactoryStripsEveryCard.test_the_factory_hands_every_card_through_the_link_strip`.
- **S39201-THE-CARD-LAYERS-NAME-THE-LINK-STRIP.** The same path; find
  `case L1, L2, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13, L14`; replace
  `case L1, L2, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13`; the same killer.

The Swift mutants, appended to `ios/CardIsolation/swift-mutants.json` with the house keys `id`,
`file`, `find`, `replace`, `killer` and `why`, each with `file`
`Sources/CardIsolation/LinkStrip.swift`, proved by CI's `card-isolation` job:

- **SW39200.** Find `(0x41...0x5A).contains(byte) ? byte | 0x20 : byte`; replace `byte`; killer
  `CardIsolationTests.LinkStripTests/test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case`;
  why: the strip folds ASCII capitals.
- **SW39201.** Find `out.append(contentsOf: renamed)`; replace
  `out.append(contentsOf: bytes[index..<(index + opener.count)])`; killer
  `CardIsolationTests.LinkStripTests/test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case`;
  why: the strip writes the renamed opener, not the one it found.
- **SW39202.** Find `guard index + opener.count <= bytes.count else {`; replace
  `guard index + opener.count < bytes.count else {`; killer
  `CardIsolationTests.LinkStripTests/test_an_opener_at_either_end_of_the_card_is_renamed`; why: an
  opener that ends the card is renamed.
- **SW39203.** Find `static let renamed = Array("<wbr".utf8)`; replace
  `static let renamed = Array("<meta".utf8)`; killer
  `CardIsolationTests.LinkStripTests/test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case`;
  why: the target is `wbr`, the void element with no attribute of its own.
- **SW39204.** Find `for offset in 0..<opener.count where`; replace
  `for offset in 1..<opener.count where`; killer
  `CardIsolationTests.LinkStripTests/test_markup_holding_no_link_opener_is_returned_byte_for_byte`;
  why: the opener's `<` is matched, so text without one is left.

The anchored lines, written verbatim in `LinkStrip.swift`, each occurring once in the file:
`static let opener = Array("<link".utf8)`; `static let renamed = Array("<wbr".utf8)`; `stripped`
walking `bytes` with `var index = 0` and the one line
`if opens(bytes, at: index) { out.append(contentsOf: renamed); index += opener.count } else { out.append(bytes[index]); index += 1 }`,
then `return String(decoding: out, as: UTF8.self)`; `opens` with the guard above and the loop
`for offset in 0..<opener.count where lowered(bytes[index + offset]) != opener[offset]`; and
`lowered` returning `(0x41...0x5A).contains(byte) ? byte | 0x20 : byte`.

No row's killer is a planted simulator test: the native mutation sweep runs the package's host
tests only. The identity stub of `LinkStrip.stripped(_:)` is the "no strip" mutant, and the
red-first record carries A5's and A6's red against it, read in CI's `harness` job on the first
push.
