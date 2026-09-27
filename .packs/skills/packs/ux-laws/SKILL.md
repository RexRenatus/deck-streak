---
requires_phxd_schema: phxd.pack.run.v1
tip_floor: e996e5b8
---

# packs/ux-laws

Systems-architect / compose **review rubric** and **white-box UX probes**
(SPEC-V2-1196 R33; SPEC-V2-1204; SPEC-V2-2191). Two stages:

- `rubric`: every law of UX that lawsofux.com lists, plus the founding
  complement *minimize target distance*. Each row is a review assert a
  reviewer applies by hand.
- `white-box`: probes that `pack run` executes against the reviewed tree,
  offline. They cover the laws' testable heuristics, WCAG 2.2 where the laws
  touch accessibility, deceptive patterns, gamification ethics and
  notification consent.

Repo `skills/` is authoritative, and the closed forbid set is
`skills/deny/DENY.md`.

Which seats consume this pack is its catalog row's `consumes`, the one
record of that edge (ADR-V2-1990), so this body names none. The pack
reports; it does not admit or bypass the ready-set.

## Probe (headless `phxd`)

`--format json` is required.

```
phxd pack run --pack ux-laws --project ID --format json
phxd pack run --pack ux-laws --project ID --root PATH --format json
```

Schema `phxd.pack.run.v1`. Every row is a READ of a tree the owner already
controls: no probe opens a socket, fetches a page or renders one.

What a walk answers:

- **The white-box rows are executed and counted in `examined`.**
  - A blocking row that finds something reads `red`: the card's `verdict` is
    `red`, `refuse_reason` is `pack_red`, and the exit is 4.
  - An advisory row that finds something reads `advisory`. It reports and
    never refuses (SPEC-V2-2194 R3).
  - A clean row reads `ok`. Its detail names what it read: a probe whose
    trigger is absent (no streak, no chest, no reminder) says so, with the
    file counts.
- **The rubric rows are reported with their declared severity, `advisory`.**
  They are never executed, because each law is the reviewer's to judge.
- **`--stage grey-box` and `--stage black-box` select no white-box row.**
  They walk only the rubric: `verdict: void`, `examined: 0`,
  `refuse_reason: declarative_walk`, exit 3.
- **A tree with no markup is refused by `controls.ui-surface`.** A UX walk
  over nothing a person sees has judged nothing, so the zero count is a red,
  never a pass.

## Applying it to another repository

The walk is project-bound. Register the repository once, then walk its root:

```
phxd project register --slug deckstreak --name DeckStreak --kind product --repo PATH
phxd pack run --pack ux-laws --project ID --root PATH --format json
```

What the probes read, under `--root`:

- **Markup.** `.html`, `.htm`, `.jsx`, `.tsx`, `.vue`, `.svelte` and `.astro`
  files, and `.rs` files holding a `view!` or `html!` template (Leptos, Yew).
  A Dioxus `rsx!` block is not tag syntax and is not read as markup.
- **Styles.** `.css`, `.scss`, `.sass` and `.less` files, `<style>` blocks,
  inline `style` (CSS text or a JSX object), and Tailwind size classes
  (`w-4`, `h-5`, `size-6`, `min-h-11`, `p-2`, `[20px]`).
- **Copy.**
  - Text between tags, and the `aria-label`, `title`, `placeholder`, `alt`,
    `value` and `label` attributes.
  - String literals in scripts and in `.rs` files. That is where a bot's
    reminder text lives.
  - `.json`, `.yaml`, `.yml`, `.toml`, `.ftl` and `.po` files under a
    `locales`, `locale`, `i18n`, `l10n`, `lang`, `translations`, `messages`,
    `copy` or `strings` directory.
- **Code and configuration.** Every file above, plus data files anywhere.
  These carry identifiers such as `quiet_hours`, `max_reminders_per_day`,
  `streak_freeze` and `leaderboard_visible`.
- **Skipped.** Dot-directories, `target`, `node_modules`, `dist`, `build`,
  `out`, `coverage`, `vendor`, `pkg` and `__pycache__`, lock files,
  `*.min.*`, and any file over 1 MiB.
- **Skipped as not the product.** Test and fixture directories (`tests`,
  `test`, `__tests__`, `__mocks__`, `fixtures`, `e2e`, `cypress`,
  `playwright`), and test files (`*.test.*`, `*.spec.*`, `*_test.rs`,
  `*_tests.rs`). A test that plants a dark pattern to prove a guard is not
  one.

Point `--root` at the product. Walked over a repository that also holds this
pack's own source, the probes read the pack's word lists as copy and report
advisories: phoenix-v2's root does, while its mini app, `ops/telegram/miniapp`,
reads `ok` on every white-box row.

Comments are blanked before anything is read, so a comment that quotes a
dark pattern is not one.

**What refuses.** A blocking row refuses the WALK (exit 4). Whether a red
walk refuses LAND is not this file's to decide. The pack's catalog `gate` is
`advisory`, and the opt-in elevation below decides it.

## The opt-in elevation, and where it lives

A red `ux-laws` row is advisory and does **not** refuse land. The
project elevates it by carrying the `pack-opt-in:ux-laws` artifact
locator, after which the same red row refuses with
`pack-hook:ux-laws:red`.

That elevation is `PackId::refuse_on_red` in
`crates/phx-coordination/src/completion_graph.rs` (SPEC-V2-1207 R9), and
this file holds **no second copy** of the rule. A rubric that also decided
land would be the second control plane 1196 R20 refuses. A row's severity
grades the walk and is never the land rule.

Two acts reach it (SPEC-V2-2203, i1812). The land hook reads only a
delivery's artifacts, so both end on the delivery.

- **The owner opts the project in, once.** It writes `pack.opted_in` about
  the project, and a second call answers `already`:

  ```
  phxd project opt-in --id ID --pack ux-laws --format json
  ```

- **The walk is recorded on the delivery.** Add `--delivery DELIVERY` to the
  walk above.
  - Each white-box row it judged is written as
    `pack-check:ux-laws/<id>/<severity>/<green|red>/white`, and so is
    `pack-opt-in:ux-laws` when the project has opted in.
  - An advisory finding and a rubric row are not written: neither can gate.
  - A grey-box or black-box walk, an unknown delivery and another project's
    delivery are refused before the walk, exit 4, and nothing is written.
  - The card's `recorded.land` is what land will read over the delivery's
    whole record: `allow`, or `pack-hook:ux-laws:red`.

The record only grows. A later walk adds its rows, and a red row of any walk
still refuses, so a fixed tree lands through its next round's delivery.

Each `checks.json` row is closed: `id`, `stage`, `severity`, `reason`,
`probe`, `scope`. The review assert a reviewer applies is the `review assert`
column below, because prose belongs in this file. `reason` is the ONE
spelling for why a red row refuses (SPEC-V2-1618 §1).

## Catalog: every checks.json row

`checks.json` is exhaustive: **66** rows.

- rubric **31**, every one advisory;
- white-box **35**: **7** blocking and **28** advisory. By prefix:
  `controls` 4, `laws` 10, `motion` 2, `deceptive` 8, `gamification` 7,
  `notifications` 4.

Blocking is kept for a requirement that a standard or an official document
states. Advisory marks a heuristic or a best-practice judgement
(SPEC-V2-2191 R3).

### rubric (31): review asserts

The founding nineteen come first, in their closed order (SPEC-V2-1204). The
other twelve laws lawsofux.com lists follow, in the site's order. The last
column names the white-box rows that test the law's heuristic, where one
exists.

| id | law | review assert | refuse | tested by |
|---|---|---|---|---|
| `hick` | Hick's Law | Decision time grows with the number and complexity of choices; the visible choice set is bounded and progressive, not an unbounded dump. | `choice-set-unbounded` | `laws.hick-choices` |
| `fitts` | Fitts's Law | Time to acquire a target is a function of its distance and size; primary controls are large enough and close enough to hit. | `control-too-small-or-far` | `controls.target-minimum`, `laws.fitts-comfortable-target`, `controls.drag-alternative` |
| `jakob` | Jakob's Law | Users spend most of their time on other sites and prefer this one to work the same way; nav, back, search, and cart keep convention. | `convention-broken` | `laws.jakob-native-controls` |
| `proximity` | Law of Proximity | Objects near each other are perceived as related; related items share a cluster and unrelated items are separated. | `related-items-scattered` | review only: needs a render |
| `miller` | Miller's Law | Working memory holds about seven (plus or minus two) chunks; lists, nav groups, and form steps stay at or below seven. | `chunk-count-exceeds-seven` | `laws.miller-group-size` |
| `doherty` | Doherty Threshold | Productivity soars when interaction stays under 400 ms; feedback is at or under 400 ms, or a progress cue is shown. | `feedback-slower-than-400ms` | `laws.doherty-feedback` |
| `von-restorff` | Von Restorff Effect | The item that differs from its peers is remembered; one primary action or plan is visually isolated. | `no-distinct-target` | review only: web-launch's `single-cta` counts CTAs |
| `serial-position` | Serial Position Effect | People remember the first and last items best; the edges of lists and flows carry the important items. | `edges-not-emphasized` | review only |
| `peak-end` | Peak-End Rule | Experiences are judged by the peak moment and the end; both the peak and the exit are designed, not leftover. | `peak-or-end-undesigned` | `laws.peak-end-session-end` |
| `zeigarnik` | Zeigarnik Effect | Unfinished tasks stay in memory better than finished ones; open work is marked with progress and a resume path. | `open-task-unmarked` | `laws.zeigarnik-progress` |
| `pragnanz` | Law of Prägnanz | People interpret the simplest possible form; figures, icons, and layouts reduce to a simple reading. | `figure-not-simple` | review only: needs a render |
| `similarity` | Law of Similarity | Like items are grouped by shared visual treatment; same-kind items share style, unlike items do not. | `like-items-unstyled` | review only: needs a render |
| `uniform-connectedness` | Law of Uniform Connectedness | Visually connected items are read as a group; a group is one region by border, fill, or connecting line. | `group-unlinkable` | review only: needs a render |
| `tesler` | Tesler's Law | Complexity is conserved and can only be moved; irreducible complexity is on the system, not dumped on the user. | `complexity-dumped-on-user` | review only |
| `postel` | Postel's Law | Be liberal in what you accept and conservative in what you send; input is tolerant, output is strict and consistent. | `input-intolerant` | `laws.postel-pattern-hint`, `laws.postel-paste-allowed`, `controls.input-purpose` |
| `parkinson` | Parkinson's Law | Work expands to fill the time available; steps and slots are time-boxed so work cannot fill an open slot. | `work-fills-the-slot` | review only |
| `occam` | Occam's Razor | The simplest working solution is preferred; no extra mechanism exists that a simpler one would replace. | `extra-mechanism` | review only |
| `pareto` | Pareto Principle | Roughly eighty percent of effects come from twenty percent of causes; the common path is shorter and clearer than rare paths. | `rare-path-equals-common` | review only |
| `minimize-target-distance` | Minimize target distance | Frequent next targets stay near the pointer; the next likely control is not across the canvas. | `target-far-from-cursor` | review only: needs a layout |
| `aesthetic-usability` | Aesthetic-Usability Effect | People judge attractive designs easier to use; polish never hides a defect a usability test would find. | `aesthetics-mask-usability` | review only |
| `choice-overload` | Choice Overload | Too many options overwhelm a decision; related options compare side by side, one is recommended, and long lists can be searched or filtered. | `options-overwhelm` | `laws.hick-choices` |
| `chunking` | Chunking | Information is split into meaningful groups; long forms, codes and lists are grouped, never dumped whole. | `content-unchunked` | `laws.chunking-form-groups`, `laws.miller-group-size` |
| `cognitive-bias` | Cognitive Bias | Systematic errors of thinking shape every decision; the design never turns one against the user. | `bias-exploited` | the `deceptive.*` and `gamification.*` rows |
| `cognitive-load` | Cognitive Load | A task costs mental effort; every cost the task itself does not need is removed. | `load-exceeds-task` | `laws.chunking-form-groups` |
| `flow` | Flow | Full, energized focus in an activity; challenge matches skill, feedback is immediate, and nothing breaks the focus without cause. | `flow-interrupted` | review only |
| `goal-gradient` | Goal-Gradient Effect | Effort rises as the goal nears; progress toward the goal is visible, and any head start shown is honest. | `progress-invisible` | `laws.zeigarnik-progress` |
| `common-region` | Law of Common Region | Elements inside a clearly bounded area read as a group; a group shares one bounded region. | `region-unbounded` | review only: needs a render |
| `mental-model` | Mental Model | Users carry a compressed model of how a system works; the design meets the model users bring, not the implementation's. | `model-mismatch` | review only |
| `active-user-paradox` | Paradox of the Active User | Users skip manuals and start at once; help is contextual and sits where the task is. | `help-out-of-context` | review only |
| `selective-attention` | Selective Attention | People attend to what serves their goal; nothing important looks like an ad, and nothing moves to take attention away. | `attention-hijacked` | `motion.autoplay-controls`, `motion.infinite-animation` |
| `working-memory` | Working Memory | A small store holds what a task needs; the interface remembers for the user and never asks again for what it already knows. | `memory-overloaded` | review only: WCAG 3.3.7 needs a session |

Order is closed (1196 R33, then lawsofux.com). A later law is an ADR
amendment, not a review invention.

### white-box (35): probes

The `reads` column says what makes the row red. Each row's detail names the
file, the line and what it measured.

#### controls (4)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `controls.ui-surface` | blocking | `ui-surface-none` | the root holds no markup, so no probe here could judge anything | the zero-count rule: a guard that examined nothing refuses |
| `controls.target-minimum` | blocking | `target-below-24px` | a pointer target whose declared width AND height are both under 24 CSS px, on the most generous reading (min-size plus padding, over inline style, Tailwind classes and every matching rule) | WCAG 2.2 SC 2.5.8 (AA) |
| `controls.drag-alternative` | advisory | `drag-without-alternative` | a front-end file that handles a drag or a swipe and holds no button for the same action | WCAG 2.2 SC 2.5.7 (AA), 2.5.1 (A) |
| `controls.input-purpose` | advisory | `autocomplete-missing` | an input for the user's own email, phone, name, password or address with no `autocomplete` token, or `off` | WCAG 2.2 SC 1.3.5 (AA) |

#### laws (10)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `laws.hick-choices` | advisory | `choice-set-over-limit` | a radio group of more than 7 options, or a `<select>` of more than 20 options with no `<optgroup>` | Hick's Law; Choice Overload |
| `laws.fitts-comfortable-target` | advisory | `target-below-44px` | a sized target with a side under 44 px that is not already under 24x24 | Fitts's Law; WCAG 2.5.5 (AAA); NN/g 1 cm; Apple 44 pt |
| `laws.jakob-native-controls` | advisory | `click-on-non-control` | a `div`, `span`, `li`, `p`, `img`, `td`, `tr`, `section`, `article`, `i` or `svg` with a click handler and no `role` | Jakob's Law; Telegram Mini App design guidelines |
| `laws.miller-group-size` | advisory | `nav-group-over-seven` | a `<nav>`, or a `navigation`, `tablist`, `menu` or `menubar` role, holding more than 7 items | Miller's Law, a signal and never a limit (lawsofux.com) |
| `laws.doherty-feedback` | advisory | `feedback-over-400ms` | a transition or animation on an interactive selector (`:hover`, `:active`, `:focus`, `button`) longer than 400 ms | Doherty Threshold; NN/g response-time limits |
| `laws.peak-end-session-end` | advisory | `session-without-end` | a front end with a study session (lesson, quiz, flashcard, deck) and no designed end (session complete, done for today, summary) | Peak-End Rule |
| `laws.zeigarnik-progress` | advisory | `steps-without-progress` | a file with two or more step markers (`data-step`, `aria-current="step"`, class `step`) that shows no progress (`<progress>`, `progressbar`, N of M) | Zeigarnik Effect; Goal-Gradient Effect |
| `laws.postel-pattern-hint` | advisory | `pattern-without-hint` | an input restricted by `pattern` with no `title`, `placeholder` or `aria-describedby` saying what it accepts | Postel's Law; WCAG 3.3.2 (A) |
| `laws.postel-paste-allowed` | advisory | `paste-blocked` | a field or a listener that cancels paste | Postel's Law; WCAG 3.3.8 (AA) |
| `laws.chunking-form-groups` | advisory | `form-fields-unchunked` | a `<form>` asking more than 7 fields with no `<fieldset>` or steps | Chunking; Cognitive Load |

#### motion (2)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `motion.autoplay-controls` | blocking | `autoplay-without-controls` | a `<video>` or `<audio>` that autoplays in a loop with no `controls` | WCAG 2.2 SC 2.2.2 (A), 1.4.2 (A) |
| `motion.infinite-animation` | advisory | `animation-cannot-pause` | an `infinite` CSS animation outside a reduced-motion block with no `animation-play-state` control anywhere, or a `<marquee>` | WCAG 2.2 SC 2.2.2 (A); Selective Attention |

#### deceptive (8)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `deceptive.preselected-consent` | blocking | `consent-preselected` | a checkbox, radio, switch or toggle checked by a literal default whose name or label asks consent: marketing, newsletter, notifications, reminders, tracking, sharing, terms | GDPR Recital 32, Art. 4(11) and 7; CJEU C-673/17 Planet49; EDPB 03/2022; deceptive.design Preselection |
| `deceptive.paid-extra-preselected` | blocking | `paid-extra-preselected` | the same, labelled with a price or a paid extra: a star or currency sign, Stars, premium, insurance, add-on | Consumer Rights Directive 2011/83/EU Art. 22; deceptive.design Sneaking |
| `deceptive.confirmshaming` | advisory | `confirmshaming-copy` | a decline ("No thanks", "No, I", "I'd rather") that shames: lose, fail, forget, give up, full price, "to learn" | deceptive.design Confirmshaming; NN/g; EDPB emotional steering |
| `deceptive.fake-urgency` | advisory | `countdown-pressure` | deadline copy (hurry, last chance, offer ends, ends in, only N hours left, countdown) or a countdown element | deceptive.design Fake urgency; FTC 2022; UCPD Annex I point 7; WCAG 2.2.1 (A) |
| `deceptive.fake-scarcity` | advisory | `scarcity-claim` | stock or demand copy: only N left, almost gone, selling fast, high demand | deceptive.design Fake scarcity; NN/g scarcity |
| `deceptive.fake-social-proof` | advisory | `social-proof-randomized` | activity copy ("N learners studying now") in a file that draws a random number | deceptive.design Fake social proof |
| `deceptive.trick-wording` | advisory | `double-negative` | a double negative around a choice: uncheck with not, want to not | deceptive.design Trick wording |
| `deceptive.cancel-path` | advisory | `cancel-path-missing` | a recurring charge (a price per month, or a Telegram `subscription_period`) and no way to cancel it | DSA Art. 25(3)(c); ROSCA 15 U.S.C. 8403; FTC 2022; deceptive.design Hard to cancel |

#### gamification (7)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `gamification.paid-random-odds` | blocking | `odds-undisclosed` | a randomized reward (chest, loot, gacha, mystery box) that is sold, and no copy states its odds | Apple App Store Review Guideline 3.1.1; Google Play Payments policy |
| `gamification.reward-odds` | advisory | `variable-reward-opaque` | a free randomized reward, and no copy states its odds | Octalysis drive 7; deceptive.design Addictive design |
| `gamification.streak-forgiveness` | advisory | `streak-without-freeze` | a streak with no freeze, repair, shield, grace or rest day | loss aversion (Kahneman and Tversky 1979; NN/g); Duolingo streak freeze |
| `gamification.streak-repair-sold` | advisory | `loss-aversion-monetized` | a way to keep a streak that is sold | Octalysis drive 8; deceptive.design Addictive design |
| `gamification.streak-loss-copy` | advisory | `streak-loss-threat` | copy that threatens the user's streak: you will lose it, don't break it | loss aversion; EDPB emotional steering |
| `gamification.infinite-feed` | advisory | `infinite-scroll` | a front end that loads without end: an IntersectionObserver with load-more, an infinite-scroll component | deceptive.design Addictive design; the Commission's DSA findings on TikTok and Meta (2026) |
| `gamification.leaderboard-opt-out` | advisory | `leaderboard-without-opt-out` | a leaderboard with no way to hide from it | Octalysis drive 5; EDPB 03/2022 |

#### notifications (4)

| id | severity | red reason | reads | source |
|---|---|---|---|---|
| `notifications.opt-out` | blocking | `unsubscribe-hidden` | a notification opt-in (`requestWriteAccess`, `Notification.requestPermission`, a push subscription, "turn on reminders") with no way to turn it off (turn off, mute, unsubscribe, `/stop`) | GDPR Art. 7(3); DSA Art. 25(3)(c); Apple App Store Review Guideline 4.5.4 |
| `notifications.permission-in-context` | advisory | `permission-on-load` | a permission request placed in a load or mount hook rather than a user gesture | MDN Notifications API; NN/g permission requests |
| `notifications.frequency-cap` | advisory | `no-frequency-cap` | a tree that sends reminders and declares no cap (max per day, cooldown) | NN/g push notifications and notification fatigue |
| `notifications.quiet-hours` | advisory | `no-quiet-hours` | a tree that sends reminders with no quiet hours or user-chosen time | NN/g push notifications; Duolingo reminder time |

## Octalysis: the reference model for gamification

Yu-kai Chou's eight core drives are the vocabulary a reviewer uses to say
WHY a mechanic motivates.

- **Drives 1 to 3 are white hat.** Epic meaning, accomplishment and creative
  feedback leave the learner in control.
- **Drives 6 to 8 are black hat.** Scarcity, unpredictability and loss
  motivate through pressure.

A black-hat mechanic is not banned. It is disclosed and bounded, and the rows
above are what hold it to that.

| drive | hat | what this pack holds it to |
|---|---|---|
| 1 Epic meaning and calling | white | review only |
| 2 Development and accomplishment | white | `laws.zeigarnik-progress`, `laws.peak-end-session-end` |
| 3 Empowerment of creativity and feedback | white | `laws.doherty-feedback` |
| 4 Ownership and possession | neutral | `deceptive.paid-extra-preselected` |
| 5 Social influence and relatedness | neutral | `gamification.leaderboard-opt-out`, `deceptive.fake-social-proof` |
| 6 Scarcity and impatience | black | `deceptive.fake-scarcity`, `deceptive.fake-urgency` |
| 7 Unpredictability and curiosity | black | `gamification.paid-random-odds`, `gamification.reward-odds`, `gamification.infinite-feed` |
| 8 Loss and avoidance | black | `gamification.streak-forgiveness`, `gamification.streak-repair-sold`, `gamification.streak-loss-copy` |

A streak should reward coming back, never punish a missed day beyond
repair. A chest may surprise, but its odds are shown before it is bought.
A reminder is asked for at a moment that explains it, can be turned off
where it was turned on, and is capped.

## Owned by another pack

These practices are checked where they belong, and cited here rather than
checked twice:

- **web-launch (d2189):** html `lang`, image alt text, form input labels,
  skip link, landmarks and heading structure.
- **vibecode-polish (d2190):**
  - focus-visible (WCAG 2.4.7 and 2.4.11) and `prefers-reduced-motion`;
  - the loading, empty, error and offline states;
  - dark-mode support;
  - the Telegram-native behaviour: theme changes, BackButton and MainButton,
    haptics, safe areas.
- **ui-styles (d2192):** contrast (1.4.3 and 1.4.11), design tokens, the type
  and spacing scales, dark-mode token pairs, and component states.

## Must not

- Fetch, render or run the reviewed site: every probe reads files.
- Treat a green walk as a UX review: the rubric is still the reviewer's.
- Decide land: the opt-in elevation lives in `completion_graph` alone.
- Example a forbidden token. See `skills/deny/DENY.md`.
