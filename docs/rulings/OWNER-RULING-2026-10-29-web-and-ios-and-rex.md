The owner amends CHARTER's mission and items 2, 14 and 19, and adds item 22, in the owner's own words (owner, 2026-10-29): "ensure deckstreak documents, and charter represent new reality of webapp and ios and the deprecation of miniapp, also our use of rex"; on the bot, "deprecated bot, since everything is being folded into the webapp and ios app"; on Rex in public text, "Name Rex and its method"; on who DeckStreak serves, "Owner-only at launch (Recommended)"; on the surfaces of the design round, "ios and web app, mini app is deprecated"; on where the v9 cutover runs, "Move to the web app"; on the charter's form, "Add notes, as always (Recommended)"; and "deckstreak is the product not rex". The owner's answers on Rex, each verbatim: "Rex live at launch (Recommended)", "Scheduling-ready and live (Recommended)", "our goal is to beat both but not a cutover requirement, we will continue to refine our model, research already proved that lower logloss does not equal better schedule", "Rex sets same-day dues", "option 2, our research proofs proved that 2 grades are better", "Let both in", "Off until earned (Recommended)", "Fail closed (Recommended)", "Stop after switch (Recommended)", "Amend the charter (Recommended)", "option 1, lets also ensure we are incorporating the kar3l dataset for content aware fitting and training", "Keep with a note (Recommended)" and "Core public, rest private (Recommended)": DeckStreak's surfaces become the web app and the universal iPhone and iPad app; the Telegram Mini App and the Telegram bot are deprecated, and the v9 cutover runs on the web app; DeckStreak answers one owner, through the apps' own sign-in; Rex is named as DeckStreak's scheduler, with its method, its on-device inference core public in this repository and its training, fitting and research private; the FSRS-7 exception in the 2026-10-04 replacement of item 4 ends, as the 2026-10-18 ruling removed FSRS from live scheduling; and the charter keeps every earlier text, with one dated note that quotes each replacement (`CHARTER.md:5-11`, `:14`, `:28-29`, `:72-76`, `:106-107`, `:135-136`, `:147-148`).

# OWNER RULING 2026-10-29: the web app, the iPhone and iPad app, and Rex

## What was held

At dev `4e600dbb`. Every CHARTER text below stays as it was, and the new amendment note quotes what replaces it, as
every earlier amendment did ("Add notes, as always (Recommended)"); the last row is the gap that item 22 fills:

| where | the text held | the owner's answer it meets |
|---|---|---|
| `CHARTER.md:5-11`, the mission | "... in a Telegram Mini App and its bot ... read in the Mini App and archived to the owner's second-brain vault." | "new reality of webapp and ios and the deprecation of miniapp": the two apps are the surfaces; the readings are read in them |
| `CHARTER.md:14`, the mission | "... a public, AGPL-licensed Rust workspace with a SvelteKit front end." | the same: a SvelteKit web app and a SwiftUI iPhone and iPad app |
| `CHARTER.md:28-29`, item 2 | "Two surfaces, one policy. The Mini App and the bot are two views of one service." | "deprecated bot, since everything is being folded into the webapp and ios app": the two apps are the surfaces, and the Mini App and the bot are deprecated |
| `CHARTER.md:135-136`, item 2 as amended 2026-10-04 | "The Mini App and the bot remain until the new surfaces carry the router, and the v9 cutover runs on today's surfaces." | "Move to the web app": the replacement of item 2 keeps "The Mini App and the bot remain until the new surfaces carry the router" as signed on 2026-10-04, and the v9 cutover runs on the web app; their code then leaves the tree in its own removal delivery |
| `CHARTER.md:147-148`, item 4 as amended 2026-10-04 | "An experimental model only reorders cards the stock scheduler already made due (OQ4), except FSRS-7 on the one preset the owner switches (OQ1)." | the signed 2026-10-18 ruling, item 2: "FSRS is removed from live scheduling"; the FSRS-7 exception is superseded, and the rest of this replacement and item 4 itself are kept |
| `CHARTER.md:72-76`, item 14 | "The bot gates every command on the owner's identity, and the Mini App validates Telegram `initData` ..." (amended 2026-10-04: sign-in on the web and on the iPhone and iPad) | "Owner-only at launch (Recommended)": "The service answers one owner" stays; its mechanism is the apps' own owner sign-in |
| `CHARTER.md:106-107`, item 19 | "the Mini App is the primary surface and the vault keeps an archive copy" | "ios and web app, mini app is deprecated" |
| `CHARTER.md`, no item | Rex is named in three files only: `docs/rulings/OWNER-RULING-2026-10-18-rex-enters-deckstreak.md`, the amendment of ADR-339 and its changelog fragment `changelog.d/docs-ruling-rex-enters-988.md` | "Name Rex and its method", the answers on Rex above, and "Core public, rest private (Recommended)": the on-device inference core is public in this repository, and training, fitting and research stay private |

## What replaces it

This ruling words each replacement; the charter's new amendment note quotes the first four word for word, below the
earlier texts it keeps.

- **The mission.** DeckStreak turns the reviews its owner does into XP, levels, streaks, quests and badges, in a web
  app and a universal iPhone and iPad app. Both apps study the owner's own Anki collection over the same engine,
  against the owner's own Anki sync server; DeckStreak scores every study day and speaks through one policy that
  decides when to celebrate, when to nudge and when to stay quiet. Its scheduler is **Rex**, DeckStreak's own memory
  model (constraint 22). Its flagship is the **daily pre-study reading**: each morning, one short text per topic that
  primes the day's new cards, written by a named mentor for that subject, read in the apps and archived to the owner's
  second-brain vault. The Telegram Mini App and the Telegram bot DeckStreak began as are deprecated.

  DeckStreak is a full-parity successor to its predecessor, a private single-user service ("v9"), rebuilt as a public,
  AGPL-licensed Rust workspace with a SvelteKit web app and a SwiftUI iPhone and iPad app. The predecessor is the
  behavioural oracle: its numbers are proved, never re-derived.
- **Charter 2: two apps, one policy.** DeckStreak's surfaces are the web app and the universal iPhone and iPad app,
  with native push (APNs and web push) as the router's way to reach the owner outside them. Every celebration and nudge
  passes through ONE router, so an event can never celebrate twice. Native nudges are opt-in, at most one a day. The
  Telegram Mini App and the Telegram bot are deprecated, and neither gains a feature. The Mini App and the bot remain
  until the new surfaces carry the router, as signed on 2026-10-04, and the v9 cutover runs on the web app; their code
  then leaves the tree in its own removal delivery.
- **Charter 14: owner-only is a safety property.** The service answers one owner. Sign-in on the web app and on the
  iPhone and iPad app pins to the one owner account, which is configuration held in a secret, by the methods ADR-131
  and ADR-132 specify, and no surface opens a session before its gate is built. Commands with stakes or destructive
  effects (panic, pardon, contracts, the agent's on-demand run) carry the same gate on every surface. While the
  deprecated Telegram surfaces remain in the tree, the bot gates every command on the owner's identity and the Mini App
  validates Telegram `initData` on the server and pins it to the owner's user id; both gates leave with their code.
- **Charter 19, one clause.** "the Mini App is the primary surface" reads "the web app and the iPhone and iPad app are
  the primary surfaces"; every other reading rule is unchanged.
- **Charter 22 (new): Rex schedules, on the device, and fails closed.** Rex is DeckStreak's own memory model and its
  only scheduler, with FSRS as its yardstick. The item states its method: a recurrent model with a memory per card and
  a memory per study day, a curve head over whole days aimed at the home preset's desired retention, same-day learning
  and relearning steps, two grades (Again and Good), live inputs that include reading and exposure events and content
  embeddings; a frozen version in a Rust core on the device, the server only relaying, and that on-device inference
  core public, entering this repository as the 2026-10-18 ruling signed; an on-device adapter off until it earns a
  held-out gain; central training on research data only, with KAR3L's content terms at zero weight until they earn a
  gain; fitting against FSRS-6 and FSRS-7 on development learners and a sealed set, judged by a scheduling simulator,
  with beating both a reported goal and not a gate; fail closed; no 1.0 release without Rex scheduling every review;
  the stock Anki apps stop studying a collection Rex schedules; and what stays private: Rex's training, fitting and
  research, which stay in Rex's private repositories with the licensed review data and the raw research.
- **Item 4's 2026-10-04 replacement, in part.** The new amendment note says: "The 10-04 replacement of constraint 4 is
  superseded where it allows FSRS-7 on the preset the owner switches: the signed 10-18 ruling removed FSRS from live
  scheduling, and rule 22 keeps FSRS only as the yardstick." Item 4's own text, its 2026-10-01 note and the rest of the
  2026-10-04 replacement are kept.
- **The 2026-10-18 ruling, with a note.** `docs/rulings/OWNER-RULING-2026-10-18-rex-enters-deckstreak.md` stays as it
  was signed. This ruling notes, and does not edit it: its dated release and go/no-go are superseded by "Rex live at
  launch" (Rex schedules every review in the first 1.0 release, whenever its gates are met); its gate 3(a), a log-loss
  and calibration win over FSRS-7, is superseded as a release gate by "Scheduling-ready and live", the win staying a
  reported goal; its item 1's "compiled for ... the server" reads, by "Amend the charter", that the server only relays
  and the x86_64 build serves tools and parity. Its item 1 otherwise stands as signed, by "Core public, rest private
  (Recommended)": Rex's on-device inference core enters this public repository, and Rex's training, fitting and
  research stay in Rex's private repositories, as its item 4 keeps the research cycle private. Its gate 3(b), "(a)
  holds, and Rex also wins the offline simulator", is superseded the same way: the scheduling simulator still judges
  every version, and a win over FSRS-7 there is a reported goal, not a release gate. Its item 6's "Central training
  uses public data only" is read as constraint 22 words it, "research data only": the licensed research collections
  its gate 3(c) names and the public KAR3L study log, never the owner's or any learner's own reviews. Its gate 3(c),
  its item 4 and its item 6, so read, stand unchanged.
- **Order.** CHARTER changes as `docs/rulings/` records here: its mission and items 2, 14 and 19 keep their text, item
  22 is added, and one dated amendment note quotes each replacement above; constraint 4 and every earlier amendment
  note are kept byte for byte. README, ARCHITECTURE, AGENTS, the PRD, the docs index, the guides and the two campaign
  SPECs change in the same commit. Landed SPECs, ADRs, rulings and changelog fragments are records: ADR-NNN
  supersedes what they say of the Mini App and the bot, and their text is kept.

## Why it is admitted

Each change is the owner's own answer (2026-10-29). The charter keeps every earlier text: charter 10, charter 11,
charter 12, charter 13 and charter 15 are untouched, and so is charter 4, which a test pins byte for byte. Owner-only
stays a safety property at launch. This ruling amends DeckStreak's charter alone, under its own signature.

The rejected alternatives:
- **Keep the bot as a channel beside the apps.** The owner chose to deprecate it: "everything is being folded into the
  webapp and ios app".
- **Keep Rex unnamed in public text, as "the scheduling companion".** The owner chose "Name Rex and its method"; only
  the licensed data and the raw research stay private.
- **Serve other learners at launch.** The owner chose "Owner-only at launch (Recommended)".
- **Make a log-loss win over FSRS-6 and FSRS-7 the cut-over bar.** The owner: "research already proved that lower
  logloss does not equal better schedule".
- **Run the v9 cutover on the Telegram surfaces until it completes, as signed on 2026-10-04.** The owner chose "Move
  to the web app".
- **Restate the charter in place, with each earlier text quoted only in this ruling.** The owner chose "Add notes, as
  always (Recommended)": the charter keeps every earlier text, and its new note quotes each replacement, as every
  earlier amendment did.
