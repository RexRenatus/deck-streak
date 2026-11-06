By signing the commit that adds this file, the owner admits two things: a card's math may be typeset by MathJax outside the card frame, in an isolated typesetter on each platform, through the amendments of ADR-352 (its cost line, `docs/decisions/ADR-352-a-card-face-runs-no-script-and-each-platform-closes-every-other-channel-with-a-named-layer-a-planted-card-proves.md:56`, and two new decisions, D9 and D10) and of ADR-360 D2 (one more web-view constructor, `docs/decisions/ADR-360-card-view-on-iphone-and-ipad.md:46-50`) that this file states, while no card script runs; and DeckStreak's note authoring suite, on the web and on iPhone and iPad, whose every write is one gesture of the owner's own in the study client, under `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`.

# OWNER RULING 2026-10-27: math typeset outside the card frame, and the note authoring suite as the owner's own gestures

The owner chose this on 2026-10-27, answering how DeckStreak's notes are authored (a DeckStreak note editor, import only, or frozen notes) with: "option 1, with a full notes editing suite, contain mathjax, css, lets brainstorm other features as well, that would improve user experience and card quality". This file is that answer, for the owner to sign. It edits no other file: each delivery that builds on it makes the amendment or the dated note named below, naming this ruling.

## What was held

At `dev`, at the parent of the commit that adds this file:

| place | what it holds |
|---|---|
| `docs/decisions/ADR-352-a-card-face-runs-no-script-and-each-platform-closes-every-other-channel-with-a-named-layer-a-planted-card-proves.md:32` | D1, "No card script runs, on either platform" |
| the same file, line 56 | D1's cost line: "Cost: a card that needs script (a hint toggle, MathJax, a typed-answer helper) renders without it." |
| the same file, lines 140-147 | D8, the last of its eight decisions; "Decision Outcome" follows at line 149 |
| `docs/decisions/ADR-360-card-view-on-iphone-and-ipad.md:46-50` | D2: `CardWebViewFactory.makeCardWebView(html:)` in `ios/CardIsolation` is the only public way to build a card view, and "A tree guard refuses any other construction of a `WKWebView` or a `WKWebViewConfiguration` under `ios/` outside test targets, any script message handler and any `loadFileURL`." |
| `docs/specs/SPEC-341-a-card-face-renders-on-the-web-in-a-sandboxed-frame-that-reaches-neither-the-app-nor-the-network.md:180-182` | "It does not run card scripts. A scripted card (a hint toggle, MathJax, a typed-answer helper) renders without its script until an owner decision on scripts, with the peer-connection control that would need, is recorded (#651)." |
| `docs/specs/SPEC-350-the-web-study-screens-review-a-card-from-the-engine-in-the-browser-in-the-sandboxed-frame-by-touch-keys-or-a-remote.md:285-286` | "It runs no card script: MathJax, hint toggles and other scripted faces render without their script (#651)." |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md:334-335` | "Bad: a card that needs its script (MathJax, a hint toggle) or a typed answer renders without it (#651, #611)." |
| `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md:24-28` | charter 4 as amended: "The study client writes the owner's own answers, and the owner's own gestures under `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`. Every other write goes through a declared write class (ADR-301)." |
| `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md:26-29` and `:33-34` | a gesture's three conditions, and the containment its build must prove |
| `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md:112` and `:123-124` | the never-list, and its entries 7 (change a reviewed note's type) and 8 (delete a reviewed card or note) |

## What was measured

- Four places at `dev` hold that a math card renders without its script: ADR-352 line 56, SPEC-341 lines 180-182, SPEC-350 lines 285-286 and ADR-361 lines 334-335. The study screens show such a card as markup (ADR-352 line 57).
- ADR-352 holds eight decisions, D1 to D8 (lines 32 to 147). The typesetter and its allowlist are its D9 and D10.
- ADR-360 D2's guard admits one public constructor of a web view under `ios/`, outside test targets.
- None of the 22 signed rulings under `docs/rulings` names MathJax, a note editor or note authoring.
- #651 asks for the owner's decision on card scripts in the study frame. This file is not that decision: it turns no card script on.

## What it closes

- A card's math shows typeset, not as markup, on the web and on iPhone and iPad, while the card frame and the card view run no card script.
- The owner writes and edits notes, note types, card templates and card CSS in DeckStreak itself, which the answer chose over import only and frozen notes.

## What replaces it

**Math, typeset outside the card frame.**

- D1 stands as written: no card script runs, on either platform, and the card frame and the card view are built as they are. A hint toggle and a typed-answer helper still render without their script.
- MathJax runs in an isolated typesetter outside the card frame. It receives TeX strings only, never card markup, and returns SVG. The SVG passes DeckStreak's allowlist before it enters the face, so a face holds no script and loads nothing.
- It fails closed: an error, a timeout or a refused SVG renders that formula as its TeX source, escaped as text, never as raw markup.
- **The ADR-352 amendment**, in `docs/decisions/ADR-352-a-card-face-runs-no-script-and-each-platform-closes-every-other-channel-with-a-named-layer-a-planted-card-proves.md`:
  - the cost line at line 56 gains: a card's math is typeset outside the frame into allowlisted SVG, so a math card shows its math with no script in the frame;
  - a new D9, after D8 (lines 140-147): the typesetter and its isolation on each platform, with what each was chosen against. On the web it reaches neither the app's origin, storage and channels nor the network; no script of the app's own origin typesets math, as a fallback or otherwise; and where the typesetter cannot start, each formula renders as its escaped source. On iPhone and iPad it is the math view below;
  - a new D10: the SVG allowlist, with what it was chosen against. It is a closed list of elements and attributes, each value matched to a closed grammar; no link, script, style element, foreign object, image, `use` element or event attribute survives it, and each formula is labelled with its TeX source;
  - a dated note naming this ruling at SPEC-341 lines 180-182, SPEC-350 lines 285-286 and ADR-361 lines 334-335.
- **The ADR-360 D2 amendment**, at `docs/decisions/ADR-360-card-view-on-iphone-and-ipad.md:46-50`: the guard admits exactly one more constructor, `MathViewFactory.makeMathView()` in `ios/CardIsolation`. The math view carries D1's L1 (a non-persistent store), L2 (page JavaScript off), L3 (the compiled rule list that blocks every load) and L4 (no script message handler) (lines 31-33), and L8, the peer-connection removal (`docs/decisions/ADR-366-card-scripts-on-iphone-and-ipad.md:48`), and its page holds no card markup; TeX reaches MathJax as a call argument, never as script text. The guard still refuses any other construction of a `WKWebView` or a `WKWebViewConfiguration` under `ios/` outside test targets, any script message handler and any `loadFileURL`.
- **The proof before it ships.** Each platform's typesetter is proved as ADR-352 D7 proves the card frame: planted formulas and a planted typesetter build against a listener the test owns, with a reference control beside every absence, in Chromium, WebKit and Firefox on the web and on one iPhone and one iPad simulator.
- **What this file does not admit.** The typesetter's packages, their licences and their size are admitted by the ADR that adds them.

**The note authoring suite, as the owner's own gestures.**

- DeckStreak's study client, on the web and on iPhone and iPad, gains a note authoring suite: adding a note; a note's fields, tags and deck; cloze tools; math in a field, typeset as above; pictures, sound and fonts added as media files; note types, with their fields, card templates, card CSS and field settings; changing a note's type; deleting a note; and a preview of each card through the same renderer, in the card frame.
- **Every write the suite makes is one owner gesture**, under the owner-taps ruling's three conditions (lines 26-29): it is made from a UI gesture the owner performs in the study client; it is one gesture's own write, on the note or note type the gesture names; and it is shown before it is made, with what it changes, with a confirmation where Anki asks for one. These writes are among "the owner's own gestures" charter 4 admits (the app-surfaces ruling, lines 25-26).
- Never-list entries 7 and 8 (ADR-301 lines 123-124) are reached from the suite only as such gestures, as the owner-taps ruling admits them. Every other path stays bound by them.
- **Containment the build must prove**, as in the owner-taps ruling (lines 33-34): the suite's write functions are reachable only from the study client's UI layer, and a test proves that no write class, agent route, server job or sync repair reaches them.
- **One gesture, never a batch.** An edit over many notes at once, such as a find and replace or a tag or deck change across a selection, is not admitted by this file; it needs a decision of its own.
- **Agents stay outside it.** An agent's change to a note type stays the live-edit class's (`docs/rulings/OWNER-RULING-2026-10-07-live-edit-mode.md:18-31`), never the suite's.
- **The one-way sync stays the owner's own tap.** A gesture whose change makes the next sync a one-way sync says so before it is made, and that sync keeps the owner-taps ruling's guards (lines 35-36).
- **Card templates and CSS** the owner writes render as every card's do: in the card frame, under D1, with no card script.
- The answer's invitation to brainstorm further features admits none by itself; each is decided on its own.

**Order.** ADR-352, ADR-360, SPEC-341, SPEC-350 and ADR-361 keep their text until the delivery that builds against them makes its amendment or adds its dated note, naming this ruling. Each delivery that builds on this file lands on `dev` only after the signed commit that adds it is there.

## Why it is admitted

The math half narrows nothing the card frame holds: D1 still turns every card script off, and the typesetter takes TeX, never a card's markup, and hands back only what the allowlist passes. It is a weakening of ADR-360 D2 by its letter, because the guard admits a second web-view constructor. Its reach is one view with the card view's store, script, load and peer-connection layers and no handler, and a planted suite proves it before it ships. It was chosen against:

- **MathJax inside the card frame.** It needs card script on, which D1 refuses for the peer-connection reason (ADR-352 lines 34-54).
- **A script engine inside the app's own process on iPhone and iPad.** It needs no second constructor, but it runs in the app's process, and public API cannot stop it on a timeout. It stays the runner-up a measured latency may still choose, and then the second constructor is not built.
- **Typesetting on the server.** The device must typeset with no network.
- **Math left as markup**, D1's cost as it stands. The owner's answer asks for MathJax in the suite.

The authoring half reaches only the owner's own hand: each write is one gesture the owner makes, sees and confirms in their own client, as the owner-taps ruling already admits for the never-list's taps. It adds no write class, no batch, no agent path and no job, and every other path stays bound by ADR-301 as it is. It was chosen against:

- **Import only, and frozen notes.** The owner chose a DeckStreak note editor over both.
- **The suite as a declared write class.** A write class runs batches under ADR-301's preview, budget and ladder; a note the owner types is one gesture, which the owner-taps ruling's conditions already bound.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
