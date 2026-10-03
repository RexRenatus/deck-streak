# SPEC-083: the skip day is recorded by DeckStreak, bridges the game, and writes its reschedule back to Anki with an exact undo

- **Wave:** W3. **Issue:** #108 (epic #4). **Context(s):** `deck-streak-ingest` (the skip record, its
  once-per-study-day key, the card snapshot, the write and its undo on a working copy, and the
  summary); `deck-streak-economy` (the tariff's price, read from `economy.json`);
  `deck-streak-coordination` (the preview, the take and the undo, the tariff's purchase and refund,
  and the skip set every recompute step reads); `deck-streak-api`, `deck-streak-bot` and the Mini
  App (`web/app`).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-037 (one daily sync plus the
  owner's triggers; its no-upload condition is superseded for this path only by ADR-089), ADR-071
  (the recompute settles each study day once, in order), ADR-083 (the owner's option (a), made on a
  working copy with incremental syncs only), and ADR-089 (the owner's decision at #266: the skip day
  writes its reschedule back to Anki under the owner's guardrails and an exact undo, and every other
  path never uploads).
- **Prerequisites:** SPEC-022 (the syncer, the collection lock and the recording layer), SPEC-023
  (the study-event rule), SPEC-071 (the rollup whose `due_today` the preview shows, and the
  recompute's step order), SPEC-072 (the consistency run and Ascendant's arming, which read the skip
  set), SPEC-076 (both streaks and the governor, which read it), SPEC-082 (the wallet's purchase and
  credit ports). SPEC-080 and SPEC-081 read the skip set when they land. **Mutation band:**
  `S08300-S08399`.
- **Status:** in delivery: moved to `docs/specs/` with its tests and `docs/red-first/SPEC-083.md`
  (ADR-016).

## 1. The problem, measured

- **What exists at `dev` 53184dd.** `crates/ingest/src/` syncs the private copy and reads it, and
  holds no skip; no table records a declared day off. The engine port (`crates/ingest/src/engine.rs`)
  offers a normal sync and a full download, and no card write. SPEC-049's lapse episode takes its
  skip days from its caller, which passes an empty set "until the skip day exists (#108)" (SPEC-049
  R15), and SPEC-072's consistency run and SPEC-076's streaks take theirs the same way.
- **The recording layer, measured at the same commit.** SPEC-022's recording layer
  (`crates/ingest/tests/support/recording.rs`) keeps every request the syncer sends, its body
  decoded, and relays it to the engine's own sync server. The census's classifier (`local_change`,
  private to `crates/ingest/tests/sync.rs`) marks an `upload`, and any change, grave or chunk that
  carries a card, note or review-log row. No test drives a write through the layer: the one planted
  upload (`support::upload_from_another_client`) posts to the server's own endpoint, beside the
  layer. Nothing yet proves that the layer would see an upload, so this SPEC proves it first (A33).
- **What the predecessor does** (`27ee2bc`). `pipeline_layers/skip.py:SkipDaysLayer.skip_preview`
  shows today's due review count (the day's rollup `due_today`), whether a skip is already active,
  the day spec and this month's tariff. `SkipDaysLayer.take_skip_day` records a row and, through
  `sync.py:AnkiSyncer.skip_day`, converges the collection with the sync server, snapshots every
  affected card, reschedules today's due review cards with `set_due_date` and uploads, all or
  nothing. `SkipDaysLayer.undo_skip_day` restores the snapshot, uploads again and refunds the tariff.
  `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` prices a skip by the month's prior
  skips, clipped to the wallet and outside the daily loss cap, and a skip it cannot fund still
  applies. `skip.py:summarize_skips` shows counts only. Every other context reads the set of
  applied, not-undone skip days (`database.py:GamifyStore.skip_days_set`).
- **The owner's decision** (#266, ADR-089). The owner chose option (a): "Upload the skip day". It
  "writes the reschedule back to Anki, as the old app did". ADR-037's no-upload condition is
  superseded for this path only, and CHARTER constraint 4 stands as written. Each of the owner's
  guardrails, and each of the undo's rules, is a criterion below whose test is planned red first:
  - (i) the only writes ever made are the skip day's reschedule of that day's due review cards, and
    its exact inverse; every other path records zero uploads against the recording fake sync server
    (A5, A6, A24, A38, A40, A44);
  - (ii) incremental sync only: a full or one-way sync demand aborts, writes nothing and tells the
    owner (A25, A26);
  - (iii) the write runs only on the owner's explicit skip declaration, inside ADR-037's
    owner-trigger rule, with no new scheduled sync (A27, A6);
  - (iv) a preview of the cards to be rescheduled is shown before the write, and their prior due
    dates are recorded so the skip can be undone (A28, A29, A35, A39);
  - (v) the recording-server proof covers both the skip day's exact changes and the zero-upload
    paths (A5, A6, A30, A38, A40, A44), and the recorder is proved to see a planted upload (A33);
  - the undo restores exactly the recorded prior due dates of exactly those cards (A30, A41); it is
    owner-triggered, incremental only, and aborts on any full-sync demand (A27, A32); it refuses
    while the study day's take is `pending`, and its preview and confirm name the study day it
    undoes (A43); it carries the same recording-server proof (A6, A30, A31); and it writes only
    cards whose current state still equals what the skip wrote, listing every card reviewed or
    changed before its converge and every card reviewed during its own window (A31, A34, A42); a
    change other than a review made during the take's or the undo's window is not seen (§6) when
    the write is newer than it, and one newer than the write wins the merge.
- **Where DeckStreak's write differs from the predecessor's**, by those guardrails:
  - the predecessor's wrap keeps only new and learning cards out of a configured search
    (`sync.py:AnkiSyncer._skip_day_blocking`); here a configured search that does not parse as one
    expression is refused, any other is also held to the study day's due review cards, and no card
    in a filtered deck is moved (R3, i);
  - the predecessor's converge resolves a full-sync demand by a full download
    (`sync.py:AnkiSyncer._converge`); here any full or one-way demand aborts the skip (ii);
  - the predecessor writes its own copy and reverts that copy in place when the upload fails
    (`AnkiSyncer._restore_cards`), which leaves the reverted cards as local changes; here the write
    runs on a working copy beside the private copy, discarded when the run ends, so no later sync
    has a local change to send (i);
  - the predecessor's preview shows a due count (`SkipDaysLayer.skip_preview`); here it also lists
    the cards and binds the take to them (iv);
  - the predecessor's undo restores every snapshotted card (`AnkiSyncer.undo_skip`); here it writes
    only cards still as the skip left them (the undo's rules).
- **Corrections to the issue.** #108's first criterion (the wrapped search, the reschedule under the
  5,000-card guard) holds as R3 and R21. Its second (the card snapshot, once per study day,
  serialised with undo, bounded by the timeout with background reconciliation) holds as R2, R22 and
  R26, with the undo's compare (R31) where the predecessor restored every card. The preview's due
  count is absent when the study day has no rollup yet, where the predecessor showed 0 (R7). Its
  third criterion's golden of `EconomyLayer._charge_skip_tariff` is kept (R8), and its fourth holds
  as R11 and R18.
- **An inert switch and cap.** The predecessor reads a skip-day switch it never checks
  (`config.py:Settings.skip_enabled`), and defines a monthly cap of skip-day bridges
  (`constants.SKIP_BRIDGE_MONTHLY_CAP`) it never enforces. Both stay unenforced here: the owner
  decided at #269 to enforce both, and #280 (W5) does. `economy.json` keeps the cap declared
  (`streak.skip_bridge_monthly_cap`), unenforced, as the game-economy pack's reference requires.
- **What the parity oracle proves.** The day spec (`skip.py:skip_spec`); the search as the write path
  wraps it (`sync.py:AnkiSyncer._skip_day_blocking`, driven through a stand-in collection that
  records the search and holds no card, so nothing is written); the preview
  (`SkipDaysLayer.skip_preview`); the charge and the refund (`EconomyLayer._charge_skip_tariff`,
  `EconomyLayer._refund_skip_tariff`); the summary (`skip.py:summarize_skips`); and the constants.
  The reschedule itself is the engine's own Set Due Date, proved against the recording layer (A5),
  not against a golden. R3's holds on the wrapped search are DeckStreak's own, and A38 proves them
  against the recording layer.
- **The process's zone, measured** in chrono 0.4.45, the version `Cargo.lock` pins, which the
  pinned engine reads the zone through (`Local`). Each thread keeps its own cache of the zone, and a
  new thread loads it afresh (`src/offset/local/unix.rs:25-35`, `89-99`). Within a thread, with
  `TZ` set, chrono keys the zone by a hash of the `TZ` string, checks that key at most once a
  second, and loads the zone again only when the string changes (`unix.rs:43-50`, `110-116`,
  `133-144`); with `TZ` unset it reads `/etc/localtime`, and loads it again when that file's
  modification time changes (`unix.rs:51-62`, `127-131`). An empty `TZ` reads UTC; `localtime`, a
  `:` prefix and an absolute path read the named file; a relative name is opened under the zone
  directories; and only a string that opens no file there is parsed as a POSIX rule
  (`src/offset/local/tz_info/timezone.rs:29-75`, `609-634`). A zone that cannot be loaded falls
  back to the zone the host's `/etc/localtime` links to (`unix.rs:80-87`, `102-104`;
  iana-time-zone 0.1.65, `src/tz_linux.rs:3-7` and `35`). Measured on a throwaway crate with `TZ`
  naming a zone file by path, which opens it as a zone name opens its file under the zone
  directories: the first thread and a new thread each read 19800 (UTC+05:30); with the file
  replaced by a UTC+09:00 zone's, the first thread still read 19800 more than a second later, its
  `TZ` string unchanged, and a new thread read 32400; with the file removed, the first thread
  still read 19800 and a new thread read 0, the measuring host's zone, through the fallback. A POSIX
  rule that names no zone file opens no file, so every thread reads the rule and nothing else (R3's
  pin).
- **Prerequisites.** SPEC-022, SPEC-023, SPEC-071, SPEC-072, SPEC-076 and SPEC-082, as the header
  lists. SPEC-080 and SPEC-081 read this SPEC's skip set and prove their own reactions to it.

## 2. Requirements

The record (#108)

R1. `deck-streak-ingest` owns `skip_days` (`migrations/008301_ingest_skip_days.sql`, `STRICT`,
    `created_at`): one row per skip taken, with its study day (an epoch day), the due review count
    the preview showed (absent when the day had no rollup), the write's state (`pending`, `applied`
    or `failed`, with one bounded reason when failed), the number of cards the write moved, whether
    its tariff went unfunded, and whether and when it was undone. A partial unique index on the
    study day over the rows `pending` or `applied` and not undone holds the once-per-study-day rule
    in the migration (the predecessor's guard, `database.py:GamifyStore.get_active_skip_day`), so a
    `failed` take leaves the day free.
R2. Taking a skip on a study day that already holds one `pending` or `applied` and not undone is
    refused with `already_skipped`: it writes no row and sends no request. After an undo the day
    can be skipped again. The record's writes run inside the kernel's `BEGIN IMMEDIATE` write
    (SPEC-020 R16), and the collection work of a take or an undo holds the exclusive collection lock
    (SPEC-022 R7), so two concurrent takes write one row, and a take never interleaves with an undo
    or a sync.
R3. The search and the day spec: the configured search (`DECKSTREAK_SKIP_SEARCH`, defaulting to the
    predecessor's `constants.SKIP_DEFAULT_SEARCH`) wrapped exactly as the golden of
    `sync.py:AnkiSyncer._skip_day_blocking` wraps it (`-is:new -is:learn`), and held to the study
    day's due review cards by the default search's own terms
    (`prop:due=0 -is:suspended -is:buried`), so whatever `DECKSTREAK_SKIP_SEARCH` says, no card is
    selected that is new, learning, relearning, suspended, buried or due on another day. The
    configured search is parsed on its own first, and one that does not parse as one expression
    (for example `deck:X) or (deck:X`, which closes the wrap's group) is refused at start with a
    bounded reason, before any preview, request or write. Wrapped as text, such a search would leave
    the group and escape every hold: the pinned engine's parser keeps a flat list of terms and its
    SQL writer emits their `or` and `and` in order, which SQLite binds `and` first
    (`rslib/src/search/parser.rs:165-230`, `rslib/src/search/sqlwriter.rs:98-116`). The search is
    also held out of filtered decks (`-deck:filtered`), a deviation from the golden: the engine's
    Set Due Date moves a card in a filtered deck back to its home deck (the pinned engine's
    `rslib/src/scheduler/filtered/card.rs:52-58`), so its exact inverse would put the card back into
    a filtered deck that may have been emptied, rebuilt or deleted since, and the engine's card
    update writes a card's deck without checking that the deck exists
    (`rslib/src/card/mod.rs:285-339`); the predecessor moved such cards and recorded their original
    deck (`odid`) but not their deck (`did`) (`sync.py:AnkiSyncer._snapshot`). A review card due
    today in a filtered deck is left where it is (A5, A38). The search the preview shows and the
    write runs is therefore the golden's wrap of the configured search followed by
    `prop:due=0 -is:suspended -is:buried -deck:filtered`, the default search included; with the
    default search it selects what the golden's search selects, less the cards in a filtered deck.
    The day spec is the golden of `skip.py:skip_spec`, which carries no `!`, so with FSRS off the
    engine's Set Due Date keeps each review card's interval; with FSRS on it sets the interval by
    its own rule, and the snapshot records the interval either way (R22). The preview shows both,
    and the write runs them. The default search's `prop:due=0` and the day spec count from the
    engine's own day, which is the collection's rollover hour in the process's own zone (the pinned
    engine's `rslib/src/scheduler/mod.rs:90-108`, `rslib/src/scheduler/timing.rs:27-48`), not the
    study day (SPEC-020 R1, SPEC-023 R7). The preview, the take and the undo run only when the
    engine's day is the study day and the collection's configured UTC offset equals the process's
    zone; otherwise each refuses with a bounded reason before any request or write. The take and
    the undo check both conditions again on the converged working copy before any card changes,
    reading the configured UTC offset without a day computation, which would rewrite it first: a
    converge that brings another client's rollover hour or configured UTC offset that fails
    either ends the take as a failure before the push's first request (R25) and the undo with
    nothing written and its skip `applied`, each with its own bounded reason, and nothing is
    pushed. At both checks a collection with no configured UTC offset counts as one whose offset
    differs: the engine counts a missing offset as UTC and writes the key at its first day
    computation whenever the process's zone's offset is not zero (the pinned engine's
    `rslib/src/scheduler/mod.rs:91-104`). The checks cannot hold a rollover in the moment between a
    check and the engine's next day computation (§6); a change of the process's zone offset in that
    moment is prevented, by the daylight-saving refusal and by the zone's pin, both below. The
    service's process runs in a time zone that observes no daylight saving: a zone whose UTC offset,
    read as the engine reads the process's zone (the `TZ` variable, a zone name or a POSIX rule,
    else `/etc/localtime`, through chrono's `Local`: the pinned engine's
    `rslib/src/timestamp.rs:39-44` and `64-66`), is not the same at every instant of the current and
    the next calendar year refuses the preview, the take and the undo with a bounded reason before
    any request or write, because an offset change between a check and the engine's next day
    computation makes the engine rewrite the configured UTC offset
    (`rslib/src/scheduler/mod.rs:101-104`), a setting guardrail (i) forbids, and no check can sit
    between that computation and the push that carries it
    (`rslib/src/sync/collection/normal.rs:80-90`). The service's zone is pinned as a fixed rule, a
    POSIX rule that names no zone file. The deployment's environment file sets `TZ` to a POSIX rule
    that names no zone file, a documented key in `deploy/deck-streak.env.example`, and each unit
    whose process runs the preview, the take and the undo (`deploy/systemd/deck-streak-api.service`,
    `deploy/systemd/deck-streak-bot.service`) reads that file through its `EnvironmentFile=`. The
    preview, the take and the undo each refuse, with a bounded reason before any request or write,
    when the process's `TZ` is unset or empty, is `localtime`, begins with `:` or `/`, names a file
    under the zone directories (which covers a `..` path, every zone name, and a rule-shaped name
    that is also a zone file), or does not parse as a POSIX rule; the daylight-saving refusal above
    then refuses a rule with a daylight period, so the three run only under a POSIX rule that names
    no zone file and has no daylight period. The pin refuses only these three, never the service's
    start. The day, offset and daylight-saving checks read the process's zone only through chrono's
    `Local`, as the engine does, and only the pin reads `TZ` itself. Under a POSIX rule that names
    no zone file chrono opens no file: it opens a zone name's file again on every new thread and
    falls back to the host's zone when it cannot, while it parses a rule from the string alone (§1,
    the process's zone), so under the pin a change of the host's zone cannot reach a running process
    (A44).
R4. The skip set is exactly the study days that hold an `applied` skip not undone
    (`database.py:GamifyStore.skip_days_set`); a `pending` or `failed` skip is not in it. It is read
    through one port in coordination that every consumer calls; no other module queries
    `skip_days`.
R5. Undo targets the most recent `applied` skip not undone
    (`database.py:GamifyStore.latest_undoable_skip`); with none it answers `nothing_to_undo`. While
    the study day's take is `pending`, the undo refuses before any request or write, with a bounded
    reason that the take's outcome is not known yet, because any `applied` skip it could target is
    then an older day's. Every undo's preview and confirm name the study day it undoes (A43). The
    skip is marked undone, at the undo's instant, only when the undo's write was accepted or had no
    card to write (R29 to R32).
R6. The summary shows counts only, equal to the golden of `skip.py:summarize_skips`: the skips in the
    current study day's calendar month, all time, the last skip's study day, and the cards the
    writes moved, all time.

The preview, the tariff and undo (#108)

R7. The preview, a coordination use case, shows the current study day's due review count
    (SPEC-071's rollup `due_today`, or absent, never 0, when the day has no rollup yet), whether a
    skip is active today, the search and the day spec (R3), the tariff, and whether the balance
    covers it; these equal the golden of `SkipDaysLayer.skip_preview` for every case, the cases with
    no rollup (class `no-rollup`) compared as absent. It also lists the cards the write would move
    (R20), which the golden does not hold.
R8. The tariff is `economy.json`'s `streak.skip_tariff_coins`, which equals the golden constant
    `constants.SKIP_TARIFF_LADDER` (0, 50 and 100 coins), indexed by the skips already applied and not
    undone in the study day's calendar month, the last price repeating. The charge is a purchase
    through economy's port, made only when the take's write is `applied` (R24, R26), clipped to the
    wallet and outside the daily loss cap; a skip whose tariff the wallet cannot cover still applies
    and its row records the shortfall. The price and the amount paid equal the golden of
    `EconomyLayer._charge_skip_tariff`.
R9. Undo refunds exactly the coins the skip paid, as a credit on the undo's study day, only when the
    undo is accepted (R32); a free skip refunds nothing (the golden of
    `EconomyLayer._refund_skip_tariff`).
R10. The tariff's price is economy's function, reading the ladder from `economy.json` once at start;
    no tariff number is typed in code.

What a skip changes, at the next recompute (#108)

R11. From the recompute that follows an applied take, each context that reads the skip set applies
    its own rule to the day: the language streak bridges it and consumes no freeze
    (`skip.py:real_misses`, SPEC-076); the law streak bridges it (`analytics.py:bridged_streak`,
    SPEC-076); the consistency run leaves it unchanged (`gamification/governor.py:tier_down_run`,
    SPEC-072); Ascendant never arms on it
    (`pipeline_layers/governor.py:GovernorLayer._maybe_grant_ascendant`, SPEC-072); and the
    governor's silent run neither counts it nor ends at it (SPEC-049, SPEC-076).
R12. The day's quests are voided, never failed (`pipeline_layers/loot.py:LootLayer._update_quests`
    returns before minting or evaluating them), no session chest is rolled on it
    (`LootLayer._grant_session_chests`), and a race week with two or more skip days is a rest week
    that is neither raced nor settled
    (`pipeline_layers/ghost_race.py:GhostRaceLayer._update_ghost_race` and
    `GhostRaceLayer._settle_due_races`). SPEC-080 and SPEC-081 read this SPEC's port and prove
    each when they land.
R13. In the game, an undo changes only what the next recompute reads: from then on the day is a
    missed day wherever a rule counts one. Transitions already settled stay as they were, as the
    predecessor's
    persisted transitions do (`pipeline.py:GamifyPipeline._advance_streak`), and no settled XP of a
    closed day is lowered (ADR-071).

The surfaces (#108)

R14. The bot keeps `/skip`, with `/cheat` dispatched as its alias and not listed in the menu, and
    `/skipundo` and `/skipstats`, each answering the owner only (SPEC-026). `/skip` answers the
    preview: the due count, the cards the write would move counted per top-level deck, the tariff,
    and Confirm and Cancel buttons (callback data `sk:go:<digest>` and `sk:no`, within the Bot
    API's 64 bytes). After a confirm it answers the take's outcome: the cards moved, `pending`, or
    why nothing was written, with every card left alone listed. `/skipundo` asks before undoing
    (`sk:undo`), naming the study day it undoes (R5), and answers with the cards restored and every
    card left alone. `/skipstats` shows the summary.
R15. The API serves `GET /api/skip/preview`, `POST /api/skip` (carrying the preview's digest),
    `POST /api/skip/undo` and `GET /api/skip/stats` to the owner's session only (SPEC-024).
R16. The Mini App's `/skip` route shows the preview (the due count, the cards the write would move,
    the tariff and whether the balance covers it) with one confirm, then the outcome: the cards
    moved, or why nothing was written, with every card left alone listed. The undo's confirmation
    names the study day it undoes (R5) and says that it restores only cards it finds unchanged
    since the skip. The route joins `ROUTES`. Its copy
    names the skip as a day off the tariff prices, and threatens no loss.

Privacy and the collection (#108)

R17. `skip_days` and `skip_card_snapshot` are declared once in ingest's data-rights port as exported
    and erased, in `privacy.json` as the category `skip-days`, and with one line in `PRIVACY.md`;
    the register of DeckStreak's own tables gains both rows.
R18. The engine's Set Due Date writes one review-log row of type 4 with ease 0 for each card it
    moves, and the read never counts one as a study event (SPEC-023 R2): a skip day stays a day with
    no study. An undo removes no review-log row, because an incremental sync carries none away, so
    those rows stay, still never study events.

The write to the collection (#266, ADR-089)

R19. Owner triggers only (guardrail iii). A take's write runs only when the owner confirms a skip:
    the bot's `sk:go`, or the Mini App's confirm through `POST /api/skip`, each owner-only, as an
    owner trigger under ADR-037. No scheduled job, recompute step, restart catch-up, startup path,
    agent tool, retry or spawned task calls the take or the undo or sends their push again, so each
    runs once per confirm. Neither adds a scheduled sync: the `sync` job keeps its one daily slot
    (SPEC-027). Their syncs are recorded on the skip's own row, never in
    `sync_runs` (SPEC-022 R16), and ADR-037's five-minute reuse of a recent sync does not apply to
    them: a write always follows its own converge.
R20. The preview binds the take (guardrail iv). The preview lists the cards the wrapped search (R3)
    selects in the private copy, read under the shared collection lock: each card's id, its
    top-level deck and its current due, with a digest of the listed ids. The confirm carries the
    digest. The take first lists the cards again from the private copy, and when the confirm
    carries no digest, or one that differs from the list it reads, it writes nothing, sends no
    request, and answers `preview_changed` with the new preview.
R21. The working copy and the converge (guardrails i and ii). Under the exclusive collection lock,
    the take copies the private copy to a working copy beside it and converges the working copy with
    one normal (incremental) sync. The cards it moves are the previewed cards that the wrapped
    search (R3) still selects in the converged working copy. A previewed card no longer due, and a
    card due that the preview did not list, are left alone and listed in the answer. More cards than
    `SKIP_MAX_CARDS` (5,000, the golden constant) are refused with `too_many_cards` before any
    change; with no card to move, the skip is recorded `applied` with none moved, and no card is
    written and no second sync runs (the predecessor's `noop`).
R22. The prior state first (guardrail iv). Before any card changes, the take commits one
    `skip_card_snapshot` row per card to be moved (`migrations/008302_ingest_skip_card_snapshot.sql`,
    `STRICT`, `created_at`, keyed to its skip): the card's id, its prior due date, and the rest of
    its prior scheduling state (at least the predecessor's snapshot, `sync.py:AnkiSyncer._snapshot`:
    queue, type, interval, ease factor, original deck and original due). After the reschedule and
    before the push, it commits the state the reschedule left each card in, with the card's
    modification time.
R23. The reschedule (guardrail i). The engine's own Set Due Date moves exactly those cards in the
    working copy, with the day spec (R3). It changes no other card, and no note, deck, notetype or
    tag. It changes no setting either. In a client-mode collection whose configured UTC offset
    differs from the process's zone, the engine rewrites that offset at its first day computation
    (`rslib/src/scheduler/mod.rs:102-104`; every normal sync that exchanges changes makes one,
    `rslib/src/sync/collection/normal.rs:87`), and every push after a reschedule carries the whole
    config table and the creation stamp (`rslib/src/sync/collection/changes.rs:131-134`,
    `meta.rs:81`), which the server stores in place of its own (`changes.rs:237-240`).
R24. The push (guardrails i, ii and v). A second normal sync pushes the working copy's change:
    exactly the moved cards and the review-log rows the engine wrote for them, beside the settings
    and the creation stamp that the sync carries whole (R23), each as the server held it at the
    converge, the engine's own last-unburied day aside. When the server accepts it, the skip is
    recorded `applied` with the cards moved, and then the tariff is charged (R8). The working copy
    is discarded when the take ends, applied or not, so the private copy is written by SPEC-022's
    syncer alone and never holds a local change.
R25. Incremental only (guardrail ii). When the server demands a full or one-way sync at the converge
    or at the push, the take aborts: it resolves the demand neither by a download nor by an upload,
    the working copy is discarded, the skip is recorded `failed` with `full_sync_required`, no tariff
    is charged, and the answer tells the owner that a full sync is needed and that nothing was
    written. Any other failure before the push's first request ends the same way, with its own
    bounded reason. A failure after it is not known to precede the server's acceptance: the engine's
    sync server commits a push only at `finish` (`rslib/src/sync/collection/start.rs:98`,
    `rslib/src/sync/collection/finish.rs:31-37`), and a lost answer to `finish` reaches the take as
    an error. Such a take is never recorded `failed`: its answer says that the outcome is not known
    yet, never that nothing was written, and its row stays `pending` until R26's compare settles it.
    A take retries no failed step: the owner takes the skip again.
R26. An outcome the answer cannot wait for (#108's reconciliation). The take's answer waits for the
    write within the syncer's bounded timeout (SPEC-022 R8). A write still running then is never
    cancelled mid-sync: the answer says `pending`, and the write's own outcome settles the row, as
    the predecessor's reconciliation does (`SkipDaysLayer._reconcile_later`). A row still `pending`
    when the service starts, or left `pending` by R25, is settled after the private copy's next
    successful sync: `applied` when any of its recorded cards carries the scheduling state and
    modification time the take recorded after its reschedule (R22), which only the take's push can
    have put on the server, and `failed` otherwise, a row with no such record included. A due date
    alone never settles a row `applied`: a card reviewed on another client can land on the date the
    skip wrote, and a tariff charged for a push that never landed is worse than a tariff missed. The
    tariff is charged only when a row settles `applied`.
R27. The read-back (the undo's rules). After the push, the take reads each moved card back from the
    synced working copy. A card whose state differs from what the skip wrote, or whose review log
    holds a study event after the take's converge (a review on another client that the sync's merge
    kept or overwrote), is listed to the owner in the answer, and the undo leaves it alone (R31).
R28. Every other path uploads nothing (guardrail i). Only the take's and the undo's write
    (`crates/ingest/src/skip_write.rs`) reaches the engine's card writes (Set Due Date and the card
    update) or pushes a local change. Every other path, from a scheduled or owner sync to a preview,
    a refused take and an aborted or failed one, and a refused undo, one with no card to write and
    an aborted or failed one, records zero uploads and zero local changes through the recording
    layer, and leaves the private copy's bytes as its own sync left them.

The undo's write (#266, ADR-089)

R29. Owner-triggered (the undo's rules). The undo runs only on the owner's explicit undo: the bot's
    `sk:undo`, or `POST /api/skip/undo`, each owner-only, as an owner trigger under ADR-037 (R19).
R30. Incremental only (the undo's rules). The undo works on its own working copy under the exclusive
    collection lock, and converges it with one normal sync. A full or one-way sync demand at its
    converge or at its push, in either direction, aborts it: it resolves the demand neither by a
    download nor by an upload, nothing is pushed, the private copy's bytes are unchanged, the
    working copy is discarded, the skip stays `applied` with its record and its tariff unchanged,
    nothing is refunded, and the answer tells the owner that a full sync is needed and that nothing
    was written.
R31. Only cards still as the skip left them (the undo's rules). The undo writes a card only when its
    scheduling state and modification time in the converged working copy equal what the skip wrote
    (R22), and its review log there holds no study event after the take's converge. A card its
    converge shows reviewed or changed since the skip is left as it is and listed to the owner,
    never written; a card deleted since is listed as gone. A card changed on another client after
    the undo's converge is not seen by this compare: R32's read-back lists it when it holds a
    review or no longer equals what the undo wrote, and §6 names the rest.
R32. The exact inverse (guardrail i and the undo's rules). For each card it writes, the undo restores
    exactly its recorded prior due date and the rest of its recorded prior state, through the
    engine's card update, and nothing else: no other card, no note, deck, notetype, tag or setting,
    and no review-log row. The push carries exactly those cards, beside the settings and the
    creation stamp that the sync carries whole (R23), each as the server held it at the undo's
    converge, the engine's own last-unburied day aside. When the server accepts it, or when no card
    needed writing, the skip is marked undone (R5) and its tariff refunded (R9); the working copy is
    discarded either way. An undo whose push fails after its first request answers that its outcome
    is not known yet, never that nothing was written, and the skip stays `applied`; a later undo
    counts a card that already equals its recorded prior state as restored, never as changed since
    the skip. After the push, the undo reads each restored card back from the synced working copy, as
    the take does (R27): a card whose state differs from what the undo wrote, or whose review log
    holds a study event after the undo's converge (a review on another client between the undo's
    converge and its push, which the sync's merge kept or overwrote), is listed to the owner in the
    undo's answer, with its review-log row kept.

The recorder's own proof (#266, ADR-089)

R33. The recording layer is proved to see a write before any proof rests on it. Another synthetic
    client's full upload, driven through the layer, is recorded as an `upload`, and its normal sync
    of one review is recorded as a chunk carrying that card and its review-log row; the census's
    classifier marks both. The classifier moves from `crates/ingest/tests/sync.rs` into
    `crates/ingest/tests/support/recording.rs`, beside a reader of the cards, the review-log rows and
    the settings each request carries, so SPEC-022's census and this SPEC's proofs share one
    classifier.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a skip is recorded once per study day, a second take is refused with `already_skipped` writing nothing and sending no request, and after an undo the day can be skipped again | `a_skip_is_recorded_once_per_study_day_and_again_after_an_undo` |
| A2 | a raw insert of a second skip `pending` or `applied` and not undone for one study day is refused by the migration's key, and a `failed` row leaves the day free | `the_migration_refuses_a_second_skip_not_undone_for_one_study_day` |
| A3 | undo targets the most recent applied skip not undone, and with none answers `nothing_to_undo` | `undo_reverses_the_most_recent_skip_not_undone` |
| A4 | the skip set holds exactly the study days with an applied skip not undone, and no `pending` or `failed` one | `the_skip_set_holds_exactly_the_days_with_an_applied_skip_not_undone` |
| A7 | the search the preview shows and the write runs is the golden's wrap of the configured search followed by R3's holds (`prop:due=0 -is:suspended -is:buried -deck:filtered`), for the default search and for each synthetic custom search the golden records, and the day spec equals the golden of `skip.py:skip_spec` | `the_search_and_day_spec_equal_the_parity_goldens` |
| A8 | the summary shows counts only, with the cards the writes moved, and equals the golden of `skip.py:summarize_skips` | `the_summary_shows_counts_equal_to_the_parity_golden` |
| A10 | the preview equals the golden of `SkipDaysLayer.skip_preview`, and its due count is absent when the study day has no rollup | `the_preview_matches_the_parity_golden_and_is_absent_without_a_rollup` |
| A11 | the tariff charged equals the golden of `EconomyLayer._charge_skip_tariff` for every count of prior skips in the month, and never counts against the daily loss cap | `the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap` |
| A12 | a skip the wallet cannot fund still applies, the wallet stops at zero, and the row records the shortfall | `an_unfunded_skip_still_applies_and_records_the_shortfall` |
| A13 | undo refunds what the skip paid as a credit on the undo's study day, and a free skip refunds nothing | `undo_refunds_what_the_skip_paid_on_the_undo_day` |
| A14 | the tariff ladder is read from `economy.json` and equals the golden constant | `the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden` |
| A15 | at the next recompute a recorded skip day bridges the language streak without consuming a freeze, and bridges the law streak | `a_skip_day_bridges_both_streaks_without_a_freeze` |
| A16 | a skip day leaves the consistency run unchanged and arms no Ascendant | `a_skip_day_leaves_the_consistency_run_unchanged_and_arms_no_ascendant` |
| A17 | a skip day neither counts toward nor ends the governor's silent run | `a_skip_day_neither_counts_nor_ends_the_governors_silent_run` |
| A18 | after an undo, the next recompute treats the day as a missed day, and transitions already settled stay as they were | `after_an_undo_the_day_is_a_missed_day_at_the_next_recompute` |
| A23 | ingest's data-rights port lists `skip_days` and `skip_card_snapshot` as exported and erased, and an erase empties both | `the_skip_tables_are_exported_and_erased` |
| A33 | (v) the recording layer records a planted upload and a planted local change: another client's full upload through it is recorded as `upload`, and that client's normal sync of one review as a chunk carrying the card and its review-log row, and the classifier marks both; and that client's normal sync after it changes one setting is recorded as a change carrying the setting's new value, and the classifier marks it | `the_recorder_sees_a_planted_upload_and_a_planted_local_change` |

```acceptance
A1: cargo test -p deck-streak-ingest --test skip_record -- --exact a_skip_is_recorded_once_per_study_day_and_again_after_an_undo
A2: cargo test -p deck-streak-ingest --test skip_record -- --exact the_migration_refuses_a_second_skip_not_undone_for_one_study_day
A3: cargo test -p deck-streak-ingest --test skip_record -- --exact undo_reverses_the_most_recent_skip_not_undone
A4: cargo test -p deck-streak-ingest --test skip_record -- --exact the_skip_set_holds_exactly_the_days_with_an_applied_skip_not_undone
A7: cargo test -p deck-streak-ingest --test skip_record -- --exact the_search_and_day_spec_equal_the_parity_goldens
A8: cargo test -p deck-streak-ingest --test skip_record -- --exact the_summary_shows_counts_equal_to_the_parity_golden
A10: cargo test -p deck-streak-coordination --test skip_flow -- --exact the_preview_matches_the_parity_golden_and_is_absent_without_a_rollup
A11: cargo test -p deck-streak-coordination --test skip_flow -- --exact the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap
A12: cargo test -p deck-streak-coordination --test skip_flow -- --exact an_unfunded_skip_still_applies_and_records_the_shortfall
A13: cargo test -p deck-streak-coordination --test skip_flow -- --exact undo_refunds_what_the_skip_paid_on_the_undo_day
A14: cargo test -p deck-streak-economy --test skip_tariff -- --exact the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden
A15: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_bridges_both_streaks_without_a_freeze
A16: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_leaves_the_consistency_run_unchanged_and_arms_no_ascendant
A17: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_neither_counts_nor_ends_the_governors_silent_run
A18: cargo test -p deck-streak-coordination --test skip_effects -- --exact after_an_undo_the_day_is_a_missed_day_at_the_next_recompute
A23: cargo test -p deck-streak-ingest --test skip_rights -- --exact the_skip_tables_are_exported_and_erased
A33: cargo test -p deck-streak-ingest --test recorder_control -- --exact the_recorder_sees_a_planted_upload_and_a_planted_local_change
```

The write's tests run the engine's own sync server in a child process with SPEC-022's recording
layer in front of it (SPEC-022 §3 and §7), on a synthetic collection built by
`crates/ingest/tests/support/synthetic.rs` that holds review cards due today; new and learning
cards the wrap must exclude; relearning, suspended and buried cards due today; a review card due
today in a filtered deck, which R3 leaves where it is; a card the skip does not move that is already
due within the day spec's range; and cards due on other days. Every card above sits in one deck,
the filtered-deck card by its home deck, which `deck:` also matches (the pinned engine's
`rslib/src/search/sqlwriter.rs:531-532`), and A38's group-closing search names that deck. A38's
buried card is buried after the burying client and the private copy have each synced once on the
engine's current day, since a normal sync first unburies every buried card, whatever day it was
buried, whenever the collection's last-unburied day is before the engine's day, and does so
without marking the card modified (`rslib/src/sync/collection/normal.rs:88`,
`rslib/src/scheduler/bury_and_suspend.rs:32-50`). The write's tests run with FSRS off and on, and
nothing reaches the owner's server. A40 plants each condition alone on a synthetic collection, the others held equal, under a test process zone, a POSIX rule that names no zone file, that observes no daylight saving and whose offset is not zero, so that R3's daylight-saving refusal cannot answer for another check: a rollover hour that makes the engine's day differ from the study day at the test's instant; a configured UTC offset that differs from the test process's zone by less than an hour (for example UTC+05:00 against a zone at UTC+05:30); and no configured UTC offset. Its daylight-saving plants run the test process under POSIX rules that name no zone file, with a daylight period, never the owner's zone, one in effect at the test's instant and one not yet begun, each on a collection whose configured UTC offset equals the zone's offset at the test's instant and whose rollover hour makes the engine's day the study day. A6 and A30 run their offset clauses on the offset plant. A5 runs in a test process whose zone, a POSIX rule that names no zone file, such as `IST-5:30`, observes no daylight saving and whose offset is not a whole number of hours, equal to the collection's configured UTC offset. For its undo, A40 takes a
skip first, and then another client's change of the rollover hour or the configured UTC offset,
or its removal of the offset, reaches the private copy by the copy's next sync; in its converge
cases the same change reaches the server after the private copy's last sync. A second synthetic
client plays the owner's other device. A26, A29, A31, A32, A34 and A42 act between the steps of a
take or an undo through a seam the skip write offers its tests: a hook called after the converge,
after the take's snapshot commit, and before the push, which does nothing in production. For A31
and A34 the other client's review
lands at the hook after the converge and at the hook before the push, the second at least one whole
second after the reschedule's modification time, because the sync's merge keeps the card with the
newer modification time, in whole seconds, a tie keeping the working copy's card, and adds every
review-log row (the pinned engine's `rslib/src/sync/collection/chunks.rs:168-193`). For A42 the other
client's review lands at the hook after the undo's converge, before its restore, and at the hook
before the undo's push, the second at least one whole second after the restore's modification
time. A39's other client changes its cards after the preview and before
the take's converge. A35 leaves a row `pending` from a take whose push never landed, and the private
copy's next sync brings one of its recorded cards, reviewed on another client with a 1-day interval,
onto the date the skip wrote, with that review's state and modification time; that card's prior
interval is 1 day and FSRS is off, so only its modification time separates it from the state the take
recorded. A35 and A41 have the recording layer drop the answer to a push's `finish` after the server
commits the push. A43 leaves the study day's take `pending` the same way, over an older `applied`
skip, and then asks for an undo. A27 plants a failure at the converge and one at the push, for a
take and for an undo. A30 compares every field of each moved card's row before the take and after
the undo. A33 is the control every other proof rests on, and it fails when the layer records
nothing. Every test that
enumerates reports its examined count and refuses zero. The red-first record gives each of A5, A6,
A24 to A35, and A38 to A44 a red commit whose failure is the criterion's own reason.

The skip's tests pin the test process's zone as the service pins its own, or R3's pin would refuse
them. `.cargo/config.toml` sets `TZ` to the POSIX rule `UTC0`, forced, in its `[env]` table, for every process Cargo runs: a rule that names no zone file, at the zero offset GitHub's hosted runners
already read. Measured on a throwaway crate at Rust 1.97.0, an edition-2024 test target reads that
value under `cargo test` and under `cargo nextest run` 0.9.146 with no code of its own, and `force`
overrides a `TZ` the shell sets. A5, A40 and A44 set their own zone, and A40 and A44 change it after
the process has started, which needs a route in this workspace: it is edition 2024 and forbids
unsafe code (`Cargo.toml`: `edition = "2024"`, `unsafe_code = "forbid"`), and edition 2024 makes
`std::env::set_var` and `std::env::remove_var` unsafe. `crates/ingest/Cargo.toml` compiles the
`skip_write` test target, which holds all three, at edition 2021 through Cargo's per-target
`edition` field, where both functions are safe, so no `unsafe` is written. Measured on the same
throwaway crate: in an edition-2024 target `set_var` fails to compile (E0133); in the edition-2021
target it builds, runs and passes `cargo clippy --all-targets -- -D warnings`; and Cargo warns that
a target's `edition` field is deprecated. Every test in that target holds one lock for its whole
run and sets the zone it runs in, because `cargo test` runs a target's tests on threads of one
process, and the zone is the process's. Each plant that sets or removes `TZ` waits more than one
second before the next preview, take or undo, because chrono checks `TZ` at most once a second on
each thread (chrono 0.4.45, `src/offset/local/unix.rs:110-116`): measured, a `TZ` changed within
that second still read the old zone, and 1.1 seconds later read the new one.

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. Every pack named here is enforced already, and this
delivery changes no pack's state.

| id | criterion | decided by |
|---|---|---|
| B1 | the privacy inventory declares the new category, over `privacy.json`, `PRIVACY.md` and `crates/ingest/src/data_rights.rs`, examining the `skip-days` category and the `skip_days` and `skip_card_snapshot` tables | the privacy-gdpr pack |
| B2 | the economy stays equal to its reference, over `economy.json`, examining the skip tariff as a rising list of coin prices and the unenforced bridge cap as declared | the game-economy pack |
| B3 | the skip sheet's copy shames no choice and threatens no streak loss, over `web/app/src/routes/skip/` and `web/app/src/lib/skip/` | the ux-laws pack |
| B4 | the `/skip` route passes the audit in both Telegram colour schemes, over `web/app/src/routes/skip/+page.svelte` | the accessibility pack |
| B5 | the skip commands keep their callback data, the preview's digest included, within the Bot API's limit and answer the owner only, over `crates/bot/src/skip_commands.rs` and `crates/bot/src/commands.rs` | the telegram-platform pack |

## 3c. Delivered by the next pull requests

This SPEC lands in three pull requests, in order (ADR-321). This one (#108, E4a) delivers the
record, the preview without its card list, the tariff and its refund, the skip set's port and its
seven readers, and what a skip changes in the game: the criteria of section 3 and of section 11.
E4b delivers the take's write with its backup, its restore drill, its counts and the class's stop;
E4c delivers the undo, the take and undo use cases, the bot, the API, the Mini App and the daemon's
wiring. The table below holds the criteria those two deliver, each row naming its part, and the
lines under it are their fence lines, each prefixed with that part. A6, A40 and A44 have undo arms:
E4b delivers each whole only when E4c has landed first, and the part that lands second moves the
row back. A part moves each of its criteria back verbatim: the row into section 3's table, without
the `delivered by` column, and the line into the acceptance fence, without the prefix. B3, B4 and
B5 (section 3a) are judged when E4c adds their files.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A5 | (i, v) a take's push, through the recording layer, carries exactly the previewed cards the wrapped search still selects at the converge; each pushed card equals the state the take recorded before the push (R22) and differs from its row at the converge only in its due date, which lies in the day spec's range from the study day, its modification time, its sync number, and what the engine's Set Due Date itself sets for that card (its interval when FSRS is on); one review-log row of type 4 with ease 0 for each; no other card, note, grave, deck, notetype or tag; no setting and no creation stamp whose value differs from the server's at the converge (the sync carries the whole config table once the working copy is newer), the engine's own last-unburied day aside and never the configured UTC offset; and no upload; a review card due today in a filtered deck is neither listed nor pushed, and its row is unchanged (R3); and the private copy's bytes are unchanged. It runs with FSRS off and with FSRS on, in a test process whose zone observes no daylight saving and whose offset is not a whole number of hours (for example UTC+05:30), equal to the collection's configured UTC offset | `a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows` | E4b |
| A6 | (i, iii, v) every other path records zero uploads and zero local changes through the recording layer: a scheduled sync, an owner sync, a preview, each refused take (`already_skipped`, `preview_changed`, `too_many_cards`), an aborted take and a failed one, a refused undo (`nothing_to_undo`), an undo that finds no card to write, an aborted undo and a failed one, and the sync that follows each; the private copy's bytes stay as its own sync left them; and each refused, aborted or failed take and undo sends no request after its answer, over a wait longer than the syncer's timeout (SPEC-022 R8); and with the collection's configured UTC offset different from the test process's zone, a preview, each refused take and each refused undo leave the private copy's bytes as its own sync left them | `every_path_but_the_take_and_the_undo_records_zero_uploads` | E4b (take arms), E4c (undo arms) |
| A9 | the engine's Set Due Date writes review-log rows the read does not count as study events | `a_reschedule_by_the_engine_is_not_a_study_event` | E4b |
| A19 | `/skip` and `/cheat` answer the preview, the cards to move counted per deck and the tariff, with Confirm and Cancel, and only for the owner | `skip_and_cheat_answer_the_preview_with_confirm_and_cancel` | E4c |
| A20 | `/skipundo` asks before undoing and lists every card left alone, and `/skipstats` shows counts only, both only for the owner | `skipundo_asks_before_undoing_and_skipstats_shows_counts_only` | E4c |
| A21 | the four skip routes answer the owner's session and refuse any other caller with no data | `the_skip_routes_answer_only_the_owner` | E4c |
| A22 | the Mini App's skip sheet shows the due count, the cards to move, the tariff and whether the balance covers it, then the outcome with every card left alone | `shows the due count, the cards to move and the tariff, then the outcome` | E4c |
| A24 | (i) only the skip's write module reaches the engine's card writes or pushes a local change, and a planted fixture that does elsewhere is refused (examined count reported, zero refused) | `only_the_skip_write_reaches_an_engine_write_or_a_push` | E4b |
| A25 | (ii) a full or one-way sync demand at a take's converge aborts it, in either direction, one a download would resolve (another client forced a one-way sync) and one only an upload would resolve (the server holds an empty collection): no request carries a card, the private copy's bytes are unchanged, the working copy is discarded, the row is `failed` with `full_sync_required`, and the answer tells the owner | `a_full_sync_demand_at_the_converge_aborts_the_take_writing_nothing` | E4b |
| A26 | (ii) a full or one-way sync demand at a take's push, in either direction, forced after the converge, aborts it the same way | `a_full_sync_demand_at_the_push_aborts_the_take_writing_nothing` | E4b |
| A27 | (iii, undo) only the owner's confirm reaches the take and the undo, once per confirm: the bot's skip callbacks and the API's two skip routes are their only callers, no scheduler job, recompute step, startup path, retry or spawned task calls them or sends their push again, and a planted caller is refused (examined count reported); and, decided dynamically, one confirm under a planted failure, at the converge or at the push, reaches the take once, and the recording layer records one converge and no second push for it; one undo confirm under the same failures reaches the undo once, with one converge and no second push | `only_the_owners_confirm_reaches_the_take_and_the_undo` | E4c |
| A28 | (iv) the preview lists the cards the write would move with their digest, and a take whose confirm carries no digest, or a digest that differs from the list the take reads again, writes nothing, sends no request and answers `preview_changed` with the new preview | `the_preview_lists_the_cards_and_a_changed_list_writes_nothing` | E4b |
| A29 | (iv) each moved card's prior due date and prior state are committed before any card changes: a take stopped between that commit and the reschedule leaves the snapshot rows and no request carrying a card | `the_prior_state_is_recorded_before_any_card_changes` | E4b |
| A30 | (undo, v) after a take and its undo each moved card's row equals its pre-skip row in every field but its modification time and sync number, with FSRS off and on, and the undo's push, through the recording layer, carries exactly those cards, no card the skip did not move, no note, grave, deck, notetype or tag, no setting and no creation stamp whose value differs from the server's at the undo's converge (the engine's own last-unburied day aside and never the configured UTC offset), no review-log row, and no upload | `an_undo_restores_exactly_the_prior_state_of_the_moved_cards` | E4c |
| A31 | (undo) the undo writes a card only when its scheduling state and modification time equal what the take recorded (R22) and its review log holds no study event after the take's converge: a card whose due date still equals what the skip wrote but that was suspended on another client, one only flagged there, and one reviewed on another client between the take's converge and its reschedule (the take's newer push overwrote the review, so its due date, scheduling state and modification time all equal what the skip recorded) are each left as they are and listed to the owner, and no request of the undo carries any of them | `an_undo_never_writes_a_card_changed_before_its_converge` | E4c |
| A32 | (ii, undo) a full or one-way sync demand at the undo's converge or its push, in either direction, aborts it: no request carries a card, the private copy's bytes are unchanged, the working copy is discarded, the skip stays `applied` with its record and tariff unchanged, nothing is refunded, and the answer tells the owner that a full sync is needed and that nothing was written | `a_full_sync_demand_aborts_the_undo_writing_nothing` | E4c |
| A34 | a card reviewed on another client between the take's converge and its push is listed to the owner in the take's answer, both when the review lands before the reschedule (the push overwrites it and only its review log shows it) and after it (the merge keeps the review), that later review landing at least one whole second after the reschedule's modification time, because a tie keeps the working copy's card (the pinned engine's `rslib/src/sync/collection/chunks.rs:184`) | `a_card_reviewed_during_the_take_is_listed_to_the_owner` | E4b |
| A35 | a take whose write outlives the answer's wait answers `pending`, and its write's own outcome settles the row; a row still `pending` at start is settled after the next successful sync by the scheduling state and modification time the take recorded after its reschedule (R26): a row whose push landed settles `applied` and is charged once, and a row whose push never landed settles `failed`, is charged no tariff and leaves its day out of the skip set, so the day is not bridged, even when one of its recorded cards, a card with a prior interval of 1 day reviewed on another client with a 1-day interval and its ease factor kept, lands on the date the skip wrote, so that, with FSRS off, only its modification time separates it from the state the take recorded after its reschedule; and a take whose push the server commits while the relay drops the answer to its `finish` answers that the outcome is not known yet, leaves its row `pending`, and after the private copy's next sync settles it `applied`, charged once and undoable | `a_write_that_outlives_the_answer_settles_its_own_row` | E4c |
| A36 | more cards than the golden 5,000 are refused with `too_many_cards` before any change or request, and a day with no card to move records the skip with no card written | `the_card_guard_refuses_a_large_set_and_an_empty_set_writes_nothing` | E4b |
| A37 | a take that is refused, aborts or fails charges no tariff and puts no day in the skip set, and an applied take charges once | `a_take_that_does_not_apply_charges_nothing` | E4c |
| A38 | (i) with a custom `DECKSTREAK_SKIP_SEARCH` that also matches a review card due on another day, a suspended and a buried review card due today, a relearning card due today, a review card due today in a filtered deck, and new and learning cards, the preview lists and the push carries only the review cards due that study day outside a filtered deck; and a configured search that closes the wrap's group (`deck:X) or (deck:X`, over a deck holding every card above) is refused before any preview, request or write | `a_custom_search_moves_only_the_study_days_due_review_cards` | E4b |
| A39 | (iv) a card the preview did not list, which another client made due before the converge, and a previewed card, which another client rescheduled before it, are each left alone and listed in the take's answer, and the push carries neither | `a_card_that_changed_between_the_preview_and_the_converge_is_left_alone` | E4b |
| A40 | (i, v) when the engine's own day differs from the study day (the synthetic collection's rollover hour differs from the study-day rule's), when the collection's configured UTC offset differs from the test process's zone, and when the collection holds no configured UTC offset and the test process's zone's offset is not zero, each planted alone, the other conditions held equal, under a test process zone, a POSIX rule that names no zone file, that observes no daylight saving, a preview and a take either list and push only that study day's due review cards, each moved to a day in the day spec's range from the study day, with no setting whose value differs from the server's at the converge, or refuse before any request or write, and an undo either restores its cards as A30 requires or refuses before any request or write; the private copy's bytes are unchanged either way; and when another client changes the server's configured UTC offset, and in a second case its rollover hour, after the private copy's last sync, so that the private copy passes both checks, a take and an undo push nothing, and no request carries a setting whose value differs from the server's at the converge; and the same holds in a third case, where that client's change leaves the server with no configured UTC offset; and in a test process whose zone observes daylight saving, on a collection that passes every other check at the test's instant (its configured UTC offset equal to that zone's offset then, and the engine's day the study day), a preview, a take and an undo each refuse before any request or write, and the private copy's bytes are unchanged, both with `TZ` holding a POSIX rule that names no zone file whose daylight period is in effect at the test's instant and with `TZ` holding one whose daylight period has not begun at the test's instant and begins within the next calendar year, and with `TZ` changed after the service has started from a rule that names no zone file and has no daylight period to a rule that names no zone file and has a daylight period, so that a check made only at the start would pass and each of the three refuses | `a_take_holds_to_the_study_day_and_writes_no_setting_when_the_engines_day_or_zone_differs` | E4b (take arms), E4c (undo arms) |
| A41 | (undo) an undo whose push the server commits while the relay drops the answer to its `finish` answers that its outcome is not known yet and never that nothing was written; the skip stays `applied`; a second undo marks it undone, refunds it once, and lists no restored card as changed since the skip | `an_undo_whose_finish_answer_is_lost_says_its_outcome_is_not_known` | E4c |
| A42 | (undo) a card reviewed on another client between the undo's converge and its push is listed to the owner in the undo's answer, with its review-log row kept, both when the review lands before the restore (the push overwrites it and only its review log shows it) and after it (the merge keeps the review), that later review landing at least one whole second after the restore's modification time, because a tie keeps the working copy's card (the pinned engine's `rslib/src/sync/collection/chunks.rs:184`) | `a_card_reviewed_during_the_undo_is_listed_to_the_owner` | E4c |
| A43 | (undo) while the study day's take is `pending`, an undo (the bot's `/skipundo` or `POST /api/skip/undo`) refuses before any request or write with a bounded reason that the take's outcome is not known yet; and every undo's preview and confirm name the study day it undoes | `the_undo_refuses_while_the_take_is_pending_and_names_the_study_day_it_undoes` | E4c |
| A44 | (i, v) the service's zone is pinned: on a synthetic collection that passes every other check (its configured UTC offset equal to the offset of the zone the test process then reads, and the engine's day the study day), with `TZ` unset in the test process, and in turn with `TZ` empty, `localtime`, `:/etc/localtime`, `/etc/localtime`, a relative name holding a `..` segment, `GMT0` (a zone file whose name also parses as a rule with no daylight period), `Etc/UTC` (a zone file whose name does not parse as a rule), and a value that names no zone file and does not parse as a POSIX rule, each planted alone, a preview, a take and an undo each refuse before any request or write with the bounded reason that the zone is not pinned, and the private copy's bytes are unchanged; with `TZ` holding the POSIX rules `UTC0` and `IST-5:30`, which name no zone file, none of the three refuses for the pin; and the checks read the process's zone only through chrono's `Local`, never from `TZ` or a zone file themselves: in `crates/ingest/src/skip_write.rs` only the pin reads `TZ` or opens a zone directory, and nothing names `/etc/localtime` | `the_skip_refuses_a_zone_the_service_does_not_pin` | E4b (take arms), E4c (undo arms) |
| A47 | the take's backup passes its restore drill before any card changes, and a backup whose drill fails ends the take with nothing written and nothing pushed | `the_takes_backup_passes_its_restore_check_before_any_card_changes` | E4b |
| A48 | the take's counts move only the review-log rows and the wrapped search's due count, by exactly the cards moved, and a planted extra change ends the take before its push and sets the class's stop | `the_takes_counts_move_only_the_review_log_rows_and_the_due_count` | E4b |
| A49 | while the class's stop is set the take refuses with writes_stopped before any request or write, and a stop set during a take ends it before its push | `the_take_refuses_while_the_classs_stop_is_set` | E4b |
| A50 | the undo's counts move only the wrapped search's due count, and a planted extra change ends it writing nothing | `the_undos_counts_move_only_the_due_count` | E4c |
| A51 | only the owner's command sets and clears the class's stop, each after a confirm | `only_the_owners_command_sets_and_clears_the_classs_stop` | E4c |

E4b: A5: cargo test -p deck-streak-ingest --test skip_write -- --exact a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows
E4b: A6: cargo test -p deck-streak-ingest --test skip_zero_upload -- --exact every_path_but_the_take_and_the_undo_records_zero_uploads
E4b: A9: cargo test -p deck-streak-ingest --test skip_write -- --exact a_reschedule_by_the_engine_is_not_a_study_event
E4c: A19: cargo test -p deck-streak-bot --test skip_commands -- --exact skip_and_cheat_answer_the_preview_with_confirm_and_cancel
E4c: A20: cargo test -p deck-streak-bot --test skip_commands -- --exact skipundo_asks_before_undoing_and_skipstats_shows_counts_only
E4c: A21: cargo test -p deck-streak-api --test skip_routes -- --exact the_skip_routes_answer_only_the_owner
E4c: A22: pnpm exec vitest run web/app/src/lib/skip/skip-sheet.test.ts -t "shows the due count, the cards to move and the tariff, then the outcome"
E4b: A24: cargo test -p deck-streak-ingest --test skip_census -- --exact only_the_skip_write_reaches_an_engine_write_or_a_push
E4b: A25: cargo test -p deck-streak-ingest --test skip_write -- --exact a_full_sync_demand_at_the_converge_aborts_the_take_writing_nothing
E4b: A26: cargo test -p deck-streak-ingest --test skip_write -- --exact a_full_sync_demand_at_the_push_aborts_the_take_writing_nothing
E4c: A27: cargo test -p deck-streak-coordination --test skip_callers -- --exact only_the_owners_confirm_reaches_the_take_and_the_undo
E4b: A28: cargo test -p deck-streak-ingest --test skip_write -- --exact the_preview_lists_the_cards_and_a_changed_list_writes_nothing
E4b: A29: cargo test -p deck-streak-ingest --test skip_write -- --exact the_prior_state_is_recorded_before_any_card_changes
E4c: A30: cargo test -p deck-streak-ingest --test skip_undo -- --exact an_undo_restores_exactly_the_prior_state_of_the_moved_cards
E4c: A31: cargo test -p deck-streak-ingest --test skip_undo -- --exact an_undo_never_writes_a_card_changed_before_its_converge
E4c: A32: cargo test -p deck-streak-ingest --test skip_undo -- --exact a_full_sync_demand_aborts_the_undo_writing_nothing
E4b: A34: cargo test -p deck-streak-ingest --test skip_write -- --exact a_card_reviewed_during_the_take_is_listed_to_the_owner
E4c: A35: cargo test -p deck-streak-coordination --test skip_flow -- --exact a_write_that_outlives_the_answer_settles_its_own_row
E4b: A36: cargo test -p deck-streak-ingest --test skip_write -- --exact the_card_guard_refuses_a_large_set_and_an_empty_set_writes_nothing
E4c: A37: cargo test -p deck-streak-coordination --test skip_flow -- --exact a_take_that_does_not_apply_charges_nothing
E4b: A38: cargo test -p deck-streak-ingest --test skip_write -- --exact a_custom_search_moves_only_the_study_days_due_review_cards
E4b: A39: cargo test -p deck-streak-ingest --test skip_write -- --exact a_card_that_changed_between_the_preview_and_the_converge_is_left_alone
E4b: A40: cargo test -p deck-streak-ingest --test skip_write -- --exact a_take_holds_to_the_study_day_and_writes_no_setting_when_the_engines_day_or_zone_differs
E4c: A41: cargo test -p deck-streak-ingest --test skip_undo -- --exact an_undo_whose_finish_answer_is_lost_says_its_outcome_is_not_known
E4c: A42: cargo test -p deck-streak-ingest --test skip_undo -- --exact a_card_reviewed_during_the_undo_is_listed_to_the_owner
E4c: A43: cargo test -p deck-streak-coordination --test skip_callers -- --exact the_undo_refuses_while_the_take_is_pending_and_names_the_study_day_it_undoes
E4b: A44: cargo test -p deck-streak-ingest --test skip_write -- --exact the_skip_refuses_a_zone_the_service_does_not_pin
E4b: A47: cargo test -p deck-streak-ingest --test skip_write -- --exact the_takes_backup_passes_its_restore_check_before_any_card_changes
E4b: A48: cargo test -p deck-streak-ingest --test skip_write -- --exact the_takes_counts_move_only_the_review_log_rows_and_the_due_count
E4b: A49: cargo test -p deck-streak-ingest --test skip_write -- --exact the_take_refuses_while_the_classs_stop_is_set
E4c: A50: cargo test -p deck-streak-ingest --test skip_undo -- --exact the_undos_counts_move_only_the_due_count
E4c: A51: cargo test -p deck-streak-bot --test skip_commands -- --exact only_the_owners_command_sets_and_clears_the_classs_stop

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/skip.rs` | `deck-streak-ingest` | added: the record, its once-per-study-day take, the snapshot's rows, the undo's compare, a pending row's compare by state and modification time (R26), the skip set, the summary, the search with its holds (R3) and the day spec |
| `crates/ingest/src/skip_write.rs` | `deck-streak-ingest` | added: the take's and the undo's write on a working copy: the preview's list and digest, the converge, the snapshot's commits, the reschedule or the restore, the push and the read-back, incremental syncs only, the refusal while the engine's day is not the study day, the configured UTC offset is missing or not the process's zone, the process's zone observes daylight saving, or its `TZ` is not a POSIX rule that names no zone file, each checked before any request, and the first two, the day and the offset, again on the converged working copy (R3, A40, A44), an outcome not known yet after a push's first request (R25, R32), and the test seam between steps |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | changed: the port gains the wrapped search's cards with their scheduling state, Set Due Date over a card list, and the card update that writes recorded fields back; `RslibEngine` implements them over the engine |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | changed: `DECKSTREAK_SKIP_SEARCH`, defaulting to the golden constant, refused at start when it does not parse as one expression, and the process's zone refused at start when it observes daylight saving (R3) |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | changed: `skip_days` and `skip_card_snapshot`, exported and erased |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the skip modules |
| `crates/ingest/Cargo.toml` | `deck-streak-ingest` | changed: `chrono`, the engine's own, for the checks' reading of the process's zone through its `Local` (R3); and the `skip_write` test target, compiled at edition 2021 so that its tests set `TZ` without `unsafe` (§3) |
| `crates/ingest/tests/skip_record.rs` | `deck-streak-ingest` | added: A1 to A4, A7, A8 |
| `crates/ingest/tests/skip_write.rs` | `deck-streak-ingest` | added: A5, A9, A25, A26, A28, A29, A34, A36, A38 to A40, A44, each test holding the target's one lock and setting the zone it runs in (§3) |
| `crates/ingest/tests/skip_undo.rs` | `deck-streak-ingest` | added: A30 to A32, A41, A42 |
| `crates/ingest/tests/skip_zero_upload.rs` | `deck-streak-ingest` | added: A6 |
| `crates/ingest/tests/skip_census.rs` | `deck-streak-ingest` | added: A24, with its planted fixture |
| `crates/ingest/tests/recorder_control.rs` | `deck-streak-ingest` | added: A33 |
| `crates/ingest/tests/skip_rights.rs` | `deck-streak-ingest` | added: A23 |
| `crates/ingest/tests/support/recording.rs` | `deck-streak-ingest` | changed: the census's classifier moves here, beside a reader of the cards, the review-log rows and the settings each request carries (R33), and the relay can drop the answer to a push's `finish` after the server commits the push (A35, A41) |
| `crates/ingest/tests/support/mod.rs` | `deck-streak-ingest` | changed: another client's normal sync, full upload and change of one setting through a given endpoint (R33, A33); another client's change of the configured UTC offset and of the rollover hour (A40), and one that leaves the server with no configured UTC offset (A40) |
| `crates/ingest/tests/support/synthetic.rs` | `deck-streak-ingest` | changed: review cards due today; new and learning cards; relearning, suspended and buried cards due today; a review card due today in a filtered deck; a card the skip does not move that is already due within the day spec's range; cards due on other days; one deck holding every card above, the filtered-deck card by its home deck (A38); a second client's review at the hook after the converge, and one at the hook before the push at least one whole second after the reschedule's modification time; for A42, a second client's review at the hook after the undo's converge, before its restore, and one at the hook before the undo's push at least one whole second after the restore's modification time; a collection for each of A40's plants, each alone: a rollover hour that makes the engine's day differ from the study day, a configured UTC offset less than an hour from the test process's zone, no configured UTC offset, and a configured UTC offset equal to a daylight-saving zone's offset at the test's instant (A40); one whose configured UTC offset is not a whole number of hours (A5); and FSRS off and on |
| `crates/ingest/tests/sync.rs` | `deck-streak-ingest` | changed: the census calls the shared classifier; SPEC-022's A15 is unchanged |
| `crates/economy/src/tariff.rs` | `deck-streak-economy` | added: the skip tariff's price, from `economy.json`'s ladder |
| `crates/economy/src/lib.rs` | `deck-streak-economy` | changed: the tariff module |
| `crates/economy/Cargo.toml` | `deck-streak-economy` | changed: `serde` and `serde_json` as dev-dependencies for the golden reader, when SPEC-082 has not added them (SPEC-029 R8) |
| `crates/economy/tests/skip_tariff.rs` | `deck-streak-economy` | added: A14 |
| `crates/coordination/src/skip/mod.rs` | `deck-streak-coordination` | added: the preview, the take and the undo, the undo's refusal while the study day's take is `pending` and the study day each undo names (R5), the tariff's purchase after an applied take and its refund after an accepted undo, and the settlement of a pending row |
| `crates/coordination/src/skip/days.rs` | `deck-streak-coordination` | added: the skip-set port every recompute step reads, in place of the empty set SPEC-071's recompute passes |
| `crates/coordination/src/recompute/` | `deck-streak-coordination` | changed: the steps that take skip days read them from `skip/days.rs` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the skip module |
| `crates/coordination/tests/skip_flow.rs` | `deck-streak-coordination` | added: A10 to A13, A35, A37 |
| `crates/coordination/tests/skip_effects.rs` | `deck-streak-coordination` | added: A15 to A18 |
| `crates/coordination/tests/skip_callers.rs` | `deck-streak-coordination` | added: A27, with its planted caller and its planted failures, and A43, with the study day's take left `pending` over an older `applied` skip |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded `skip_days` and `skip_card_snapshot` rows |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: ingest's port is registered already (SPEC-021); listed under SPEC-021's six-file rule |
| `crates/api/src/skip_routes.rs` | `deck-streak-api` | added: the four skip routes, the take carrying the preview's digest |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: mounts the skip routes |
| `crates/api/tests/skip_routes.rs` | `deck-streak-api` | added: A21 |
| `crates/bot/src/skip_commands.rs` | `deck-streak-bot` | added: the skip, cheat, skipundo and skipstats commands, and the `sk:` callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table, and the owner's menu gains the skip, skipundo and skipstats commands |
| `crates/bot/tests/skip_commands.rs` | `deck-streak-bot` | added: A19, A20 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the skip use cases joined to the record, the write, the rollup and the wallet, and the settlement of a pending row at start |
| `web/app/src/routes/skip/+page.svelte` | miniapp | added: the skip sheet's screen |
| `web/app/src/lib/skip/SkipSheet.svelte` | miniapp | added: the preview with the cards to move, the confirm, and the outcome with every card left alone |
| `web/app/src/lib/skip/api.ts` | miniapp | added: the skip routes' client |
| `web/app/src/lib/skip/skip-sheet.test.ts` | miniapp | added: A22 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the skip route joins `ROUTES` |
| `migrations/008301_ingest_skip_days.sql` | `deck-streak-ingest` | added: `skip_days`, `STRICT`, with the write's state and the partial unique index on the study day over rows `pending` or `applied` and not undone |
| `migrations/008302_ingest_skip_card_snapshot.sql` | `deck-streak-ingest` | added: `skip_card_snapshot`, `STRICT`, keyed to its skip, with each card's prior state and the state the reschedule left it in |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `Cargo.toml` | workspace | changed: `chrono` in `[workspace.dependencies]`, at the version `Cargo.lock` already pins for the engine |
| `.cargo/config.toml` | repo | added: `TZ` set to `UTC0`, forced, in `[env]`, so every process Cargo runs has a pinned zone (§3) |
| `.env.example` | repo | changed: `DECKSTREAK_SKIP_SEARCH`, empty, with the default named in its comment |
| `deploy/deck-streak.env.example` | deploy | changed: `TZ`, set to the POSIX rule `UTC0` as a neutral value, which pins the service's zone as a fixed rule (a POSIX rule that names no zone file); its comment says that the value is a POSIX rule that names no zone file and has no daylight period and that the skip's preview, take and undo refuse any other (R3) |
| `deploy/systemd/deck-streak-api.service`, `deploy/systemd/deck-streak-bot.service` | deploy | unchanged: each reads the environment file already (`EnvironmentFile=`), so the file's `TZ` reaches the process that runs the preview, the take and the undo; listed so the pin's route is named |
| `scripts/tests/test_deploy_templates.py` | repo | changed: `TZ` joins `RUST_LOG` as a key of the environment example that no role declares, because chrono reads it (R3) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `skip_days` and `skip_card_snapshot` |
| `docs/OWNER-SETUP.md` | docs | changed: the sync server's section says that the skip day's reschedule and its undo are DeckStreak's only writes to the owner's server (ADR-089), and that a preview, a take or an undo refuses unless the process's zone is the collection's configured UTC offset, the engine's day is the study day, and the service's zone observes no daylight saving and is pinned as a POSIX rule that names no zone file by `TZ` in its environment (R3) |
| `privacy.json` | repo | changed: the `skip-days` category, over both tables |
| `PRIVACY.md` | repo | changed: the `skip-days` line |
| `tools/parity-oracle/registry/spec_083.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/skip_spec.json` | repo | added: the golden of `skip.py:skip_spec` (function) |
| `tools/parity-oracle/goldens/skip_search.json` | repo | added: the golden of `sync.py:AnkiSyncer._skip_day_blocking` (adapter; a stand-in collection records the search and holds no card) |
| `tools/parity-oracle/goldens/skip_preview.json` | repo | added: the golden of `pipeline_layers/skip.py:SkipDaysLayer.skip_preview` (adapter; a stub store) |
| `tools/parity-oracle/goldens/skip_tariff.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` (adapter; a stub store records the charge) |
| `tools/parity-oracle/goldens/skip_tariff_refund.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer._refund_skip_tariff` (adapter; a stub store records the credit) |
| `tools/parity-oracle/goldens/skip_summary.json` | repo | added: the golden of `skip.py:summarize_skips` (adapter; days as epoch days) |
| `tools/parity-oracle/goldens/skip.constants.json` | repo | added: the skip constants (constants) |
| `scripts/mutation-rows.d/S08300-S08399.json` | repo | added: the hand-proved rows of §9 |
| `docs/specs/SPEC-083-the-skip-day-is-recorded-by-deckstreak-and-bridges-the-game-without-writing-to-anki.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-083-the-skip-days-write-to-the-collection-waits-for-the-owner.md` | docs | changed: accepted at this delivery, with the owner's decision at #266 (ADR-089) |
| `docs/red-first/SPEC-083.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It enforces neither the predecessor's unchecked skip switch nor its unenforced monthly cap of
  skip-day bridges: the owner decided at #269 to enforce both, and #280 (W5) does.
- It resolves no full-sync demand inside a take or an undo: either aborts, and the owner's `/sync`
  downloads as SPEC-022 R6 says (#266).
- It retries no failed write: the owner takes the skip again, or undoes it again (#266).
- It moves no review card in a filtered deck: such a card stays due there, because the exact
  inverse of Set Due Date's move back to the home deck cannot be kept once the filtered deck is
  emptied, rebuilt or deleted (R3, #266).
- It proves no quest void, chest pause or race-week exemption itself: the quests, the chests and the
  race read this SPEC's port and prove each when they land (#100, #102, #124).
- It pauses no committed-window verdict (#109), no contract breach (#113) and no fine (#110), and
  extends no wager (#112): the discipline wave reads the same port.
- It withholds no evening nudge on a skip day and adds no skip button to the evening stakes card
  (#117).
- It publishes no skip badge or public skip count (#156).
- It serves no agent tool that takes, undoes or lists skips (#157).
- It imports none of the predecessor's skip rows (#61).
- It amends no charter constraint: CHARTER constraint 4 stands as written, and ADR-089 records the
  owner's decision (#266).

## 6. Risks

- **A defect in the write reaches the owner's collection.** Prevented by the working copy, the
  incremental-only rule and the undo's compare (R21 to R32); detected by A5, A6, A24 to A34, and
  A38 to A44, each red first against the recording layer, whose own control runs first (A33).
- **A review on another client during a take loses its schedule to the reschedule**, because the
  newer change wins the sync's merge. Detected by the read-back (A34): the owner sees the card
  listed, and the undo leaves it alone (A31).
- **The undo's window.** A review made on another client between the undo's converge and its push
  can lose its schedule to the restore, which is newer; R32's read-back lists the card to the owner,
  who may reschedule it. Detected, not prevented: the sync keeps the newer card. A suspension, a
  flag, a deck move or a setting made on another client in that window is overwritten when the
  restore is newer, and is not detected: such a card equals what the undo wrote and holds no
  study event.
- **A review or another change on a device that syncs only after a push is lost, unlisted.** A review, a suspension, a flag or a deck move made on another device before a take's reschedule or an undo's restore, and synced only after that run's push, reaches the server older than the card the push left there, so the sync's merge keeps the pushed card and adds a review's review-log row (the pinned engine's `rslib/src/sync/collection/chunks.rs:182-190`). Not detected: that change reaches the server after the read-back, so where this SPEC says a card reviewed or changed on another client before or during a window, it means a review or a change that reached the server by then. It is the sync's last-writer-wins rule, the one that decides between two devices' changes to one card.
- **A change other than a review, made on another client during a take, is overwritten.** A
  suspension, a flag, a deck move or a setting made on another client between the take's converge
  and its push is overwritten when the reschedule is newer: the sync's merge keeps the card with the
  newer modification time (the pinned engine's `rslib/src/sync/collection/chunks.rs:182-190`), and
  the push carries the whole config table once the working copy is newer (R23). Not detected: the
  read-back reads only the moved cards, and such a card equals what the skip wrote and holds no
  study event. The window is the seconds between the converge and the push.
- **A skip leaves a filtered deck's review cards due.** Visible: the preview lists only the cards it
  moves, and a card in a filtered deck is never among them (R3, A5, A38); the owner can empty the
  filtered deck in Anki before taking the skip.
- **A pending row settles `failed` although its push landed**, when every card it moved was
  reviewed or changed on another client before the private copy's next sync. Accepted: no tariff is
  charged and the day is not bridged, because a tariff charged for a push that never landed is
  worse than one missed (R26, A35).
- **A push's answer is lost after the server committed it.** Visible: the take answers that its
  outcome is not known yet, and its row stays `pending` until the private copy's next sync settles
  it (R25, R26, A35); an undo answers the same and leaves its skip `applied`, and a later undo
  counts the cards already restored as restored (R32, A41).
- **The owner's server demands a full sync**, so a take aborts. Visible: the answer says why, and the
  owner's `/sync` resolves a download (SPEC-022 R6). A server that only a full upload could satisfy
  keeps the skip from writing, which is guardrail (ii) working (ADR-089).
- **The process's zone is not the collection's configured UTC offset, or the engine's day is not the
  study day** (for example in the hours between the collection's rollover and the study-day rule's,
  when the two differ), so a preview, a take or an undo refuses (R3); a collection with no
  configured UTC offset is refused the same way, and a converge that brings such a setting from
  another client ends a take before its push and an undo with nothing written. Visible: the
  answer gives the bounded reason, and the owner-setup guide says so (§4).
- **The service's environment does not pin its zone as a POSIX rule that names no zone file**, so
  every preview, take and undo refuses (R3, A44). Visible: the answer gives the bounded reason, and
  the owner-setup guide and the environment example say so (§4).
- **A rollover between a check and the engine's next day computation.** R3 checks before any request
  and again on the converged working copy, and the engine computes its day again at the reschedule
  (the pinned engine's `rslib/src/scheduler/reviews.rs:136`) and at the start of each normal sync
  that exchanges changes (`rslib/src/sync/collection/normal.rs:87`). A rollover in that moment makes
  the reschedule count the day spec's range from the next day, so the cards land a day later than R3
  intends. The window is the moment between a check and that computation; the impact is low. A
  change of the zone's offset in that moment is prevented: R3 refuses the preview, the take and the
  undo before any request or write when the process's zone observes daylight saving (A40) and when
  the service's environment does not pin the zone as a POSIX rule that names no zone file (A44), so
  no deployment that could hit this race ever starts one. An operator's change of the host's zone
  while a run is in flight cannot reach the process: the zone is pinned as a fixed rule, a POSIX rule that names no zone file (R3). The
  rollover is the one moment the checks cannot hold.
- **The private copy shows the moved cards as due until its next sync.** Visible: the take's answer
  counts the cards moved, and the owner's `/sync` refreshes the copy (ADR-037).
- **A consumer reads the skip days its own way** and drifts from the port. Detected by A4 and the
  rule that no other module queries `skip_days`, and by each consumer's own criteria over the port
  (SPEC-072, SPEC-076, SPEC-080, SPEC-081).
- **A retried take charges the tariff twice.** Prevented by the once-per-study-day key (A2), the
  charge on an applied take only (A37) and the coin ledger's unique (day, source, ref) key
  (SPEC-082); detected by A11.
- **An undo of an old skip surprises the owner** by turning a bridged day into a missed one at the
  next recompute, or by undoing an older skip while the study day's take is still `pending`.
  Detected by A18; the undo refuses while that take is `pending`, and its preview and
  confirmation name the study day it undoes and say so first (R5, A43).
- **Someone adds a write on another path.** Detected by A6 and A24, and by SPEC-022's no-upload
  census.
- **The preview's due count is as old as the study day's last recompute, and its card list as old as
  the private copy's last sync.** Visible: the take converges first and moves only previewed cards
  still due (R21), and the owner's `/sync` refreshes both (ADR-037).

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_083.py`; every day is an epoch day number and every
instant epoch milliseconds (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `skip_spec` | `skip.py:skip_spec` | function | nothing: equal, reversed and sub-one bounds included |
| `skip_search` | `sync.py:AnkiSyncer._skip_day_blocking` | adapter | a stand-in syncer whose guarded open yields a stand-in collection: its login answers a token, the converge is patched to succeed, and its card search records the query and answers no card, so the function returns before any write; it returns the recorded search, for the default search and synthetic custom ones |
| `skip_preview` | `pipeline_layers/skip.py:SkipDaysLayer.skip_preview` | adapter | a stand-in layer on the case's study day with a stub store answering the day's rollup `due_today` or none (class `no-rollup`), an active skip or none, the month's skip rows and the balance; the day returned as an epoch day |
| `skip_tariff` | `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` | adapter | a stub store answering the case's skip rows (study days, applied, undone) and balance, recording the coin delta and the detail it writes; classes `first-free`, `ladder-top`, `unfunded`, `month-boundary`, `undone-not-counted` |
| `skip_tariff_refund` | `pipeline_layers/economy.py:EconomyLayer._refund_skip_tariff` | adapter | a stub store answering what the skip paid and recording the credit and the day it lands on |
| `skip_summary` | `skip.py:summarize_skips` | adapter | the rows and `today` built from epoch days; the last day returned as an epoch day or null |
| `skip.constants` | `constants.SKIP_TARIFF_LADDER`, `SKIP_DEFAULT_SEARCH`, `SKIP_SPREAD_MIN_DAYS`, `SKIP_SPREAD_MAX_DAYS`, `SKIP_MAX_CARDS`, `SKIP_BRIDGE_MONTHLY_CAP` | constants | nothing |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `skip_days` | `ingest` | `migrations/008301_ingest_skip_days.sql` (SPEC-083) | `skip_days`: each applied row maps to one `applied` row with its day, its undone flag and instant, and its moved count; rows never applied are not imported | exported and erased |
| `skip_card_snapshot` | `ingest` | `migrations/008302_ingest_skip_card_snapshot.sql` (SPEC-083) | `skip_card_snapshot`: each row of an imported skip maps to one row with its prior fields; the predecessor recorded no state the reschedule left, so an undo writes no imported card and lists each to the owner (R31) | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08301-ONE-SKIP-PER-STUDY-DAY` | `migrations/008301_ingest_skip_days.sql` | the partial unique index on the study day over rows `pending` or `applied` and not undone (a script-mutation row) | `skip_record::the_migration_refuses_a_second_skip_not_undone_for_one_study_day` |
| `S08302-THE-SET-EXCLUDES-UNDONE` | `crates/ingest/src/skip.rs` | the skip set's filter on applied rows not undone | `skip_record::the_skip_set_holds_exactly_the_days_with_an_applied_skip_not_undone` |
| `S08303-UNDO-TAKES-THE-LATEST` | `crates/ingest/src/skip.rs` | the undo's order, the most recent skip first | `skip_record::undo_reverses_the_most_recent_skip_not_undone` |
| `S08304-THE-SEARCH-IS-WRAPPED` | `crates/ingest/src/skip.rs` | the wrap that keeps new and learning cards out of the search | `skip_record::the_search_and_day_spec_equal_the_parity_goldens` |
| `S08305-THE-LAST-PRICE-REPEATS` | `crates/economy/src/tariff.rs` | the ladder's index clamped to its last price | `skip_tariff::the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden` |
| `S08306-THE-TARIFF-IS-A-PURCHASE` | `crates/coordination/src/skip/mod.rs` | the charge taken through the purchase port, outside the daily loss cap | `skip_flow::the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap` |
| `S08307-AN-UNFUNDED-SKIP-APPLIES` | `crates/coordination/src/skip/mod.rs` | the amount paid clipped to the balance while the skip stands | `skip_flow::an_unfunded_skip_still_applies_and_records_the_shortfall` |
| `S08308-THE-REFUND-LANDS-ON-THE-UNDO-DAY` | `crates/coordination/src/skip/mod.rs` | the refund credited on the undo's study day | `skip_flow::undo_refunds_what_the_skip_paid_on_the_undo_day` |
| `S08309-A-FULL-SYNC-DEMAND-ABORTS` | `crates/ingest/src/skip_write.rs` | the abort on a full or one-way sync demand at the take's converge | `skip_write::a_full_sync_demand_at_the_converge_aborts_the_take_writing_nothing` |
| `S08310-ONLY-PREVIEWED-CARDS-MOVE` | `crates/ingest/src/skip_write.rs` | the moved set held to the previewed cards still due | `skip_write::a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows` |
| `S08311-THE-PRIOR-STATE-FIRST` | `crates/ingest/src/skip_write.rs` | the snapshot's commit before the reschedule | `skip_write::the_prior_state_is_recorded_before_any_card_changes` |
| `S08312-THE-CARD-GUARD` | `crates/ingest/src/skip_write.rs` | the refusal above `SKIP_MAX_CARDS` before any change | `skip_write::the_card_guard_refuses_a_large_set_and_an_empty_set_writes_nothing` |
| `S08313-A-CHANGED-PREVIEW-WRITES-NOTHING` | `crates/ingest/src/skip_write.rs` | the digest's comparison before any request | `skip_write::the_preview_lists_the_cards_and_a_changed_list_writes_nothing` |
| `S08314-THE-UNDO-WRITES-ONLY-UNCHANGED` | `crates/ingest/src/skip_write.rs` | the undo's comparison with the state the skip wrote | `skip_undo::an_undo_never_writes_a_card_changed_before_its_converge` |
| `S08315-THE-UNDO-RESTORES-THE-PRIOR-STATE` | `crates/ingest/src/skip_write.rs` | the restore of every recorded field | `skip_undo::an_undo_restores_exactly_the_prior_state_of_the_moved_cards` |
| `S08316-THE-UNDO-ABORTS-ON-A-FULL-SYNC` | `crates/ingest/src/skip_write.rs` | the undo's abort on a full or one-way sync demand | `skip_undo::a_full_sync_demand_aborts_the_undo_writing_nothing` |
| `S08317-THE-TARIFF-AFTER-APPLY` | `crates/coordination/src/skip/mod.rs` | the charge made only on an applied take | `skip_flow::a_take_that_does_not_apply_charges_nothing` |
| `S08318-THE-WORKING-COPY-IS-DISCARDED` | `crates/ingest/src/skip_write.rs` | the private copy left to SPEC-022's syncer | `skip_zero_upload::every_path_but_the_take_and_the_undo_records_zero_uploads` |
| `S08319-THE-READ-BACK-LISTS` | `crates/ingest/src/skip_write.rs` | the read-back's listing of a card changed during the take | `skip_write::a_card_reviewed_during_the_take_is_listed_to_the_owner` |

## 10. Amendments, 2026-10-03: the record and the game land first (#108), the skip day as ADR-301's first declared write class, and what E4b and E4c deliver

Section 3's table now holds only this part's criteria; every other row moved, verbatim, to section
3c with its fence line; A45 and A46 are added in section 11, and A47 to A51 in section 3c. The body
above is otherwise as planned; the amendments below are recorded here and are not applied to it;
each Old is the body's text and each New is what it reads from this pull request on. Where this
SPEC, ADR-083 or ADR-089 says otherwise than ADR-301, ADR-301 governs: they are "read under this
ADR" (ADR-301, Consequences). The file keeps its planned name, whose slug (`without-writing-to-anki`)
the title on line 1 contradicts; the title governs, and the name is kept so that every citation of
the path holds.

- **T1** (header, Decided by). Old: `path never uploads).` New: `path never uploads), read under
  ADR-301 (the owner's decision at #514: the collection is written only through declared write
  classes, and the skip day is the first, its ceiling the approval rung) and ADR-321 (this SPEC's
  three parts, the class's backup, restore drill, counts and stop, and its formal decision).`
- **T2** (header, Prerequisites, across the wrap). Old: `SPEC-082 (the wallet's purchase and credit
  ports).` New: `SPEC-082 (the wallet's floor-clipped debit and its refund, R7), SPEC-301 (declared
  write classes).`
- **T3** (R8, across the wraps). Old: `The charge is a purchase through economy's port, made only
  when the take's write is `applied` (R24, R26), clipped to the wallet and outside the daily loss
  cap;` New: `The charge is economy's floor-clipped debit (`debit_floored`, SPEC-082 R7) with source
  `skip_tariff` and the skip's id as its reference, on the skip's own study day, so a settlement on
  any later day finds the same key and charges once; it is made only when the take's write is
  `applied` (R24, R26), clipped to the wallet and outside the daily loss cap. A price of 0 makes no
  call. A debit the empty wallet clips to 0 writes its movement of 0, where the predecessor wrote
  none, and the golden's comparison reads both as nothing paid;`
- **T4** (R9). Old: `Undo refunds exactly the coins the skip paid, as a credit on the undo's study
  day,` New: `Undo refunds exactly the coins the skip paid, through economy's refund (`refund`,
  SPEC-082 R7) with source `skip_tariff_refund` and the skip's id as its reference, as a credit on
  the undo's study day as the row records it, so a retry on a later day credits once,`
- **T5** (section 4, after the row `crates/coordination/src/recompute/`). New rows:
  `crates/coordination/src/streak_views.rs`, `crates/coordination/src/lapse.rs` and
  `crates/coordination/src/progression/level_view.rs`, each `deck-streak-coordination`, `changed:
  reads the skip set from skip/days.rs in place of the empty set`. R4's "every consumer" holds seven
  sites at dev: `recompute/streaks.rs:115` and `:183`, `recompute/xp.rs:200-201`,
  `recompute/day_bonuses.rs:52`, `streak_views.rs:123-124`, `lapse.rs:48`,
  `progression/level_view.rs:62`.
- **T6** (section 4, row `crates/ingest/src/settings.rs`). Old: `and the process's zone refused at
  start when it observes daylight saving (R3)` New: `and nothing checked of the process's zone at
  start: the zone pin and the daylight-saving refusal run at each preview, take and undo (R3)`. R3
  says the pin "refuses only these three, never the service's start".
- **T7** (section 4, row ADR-083). Old: `changed: accepted at this delivery, with the owner's decision
  at #266 (ADR-089)` New: `changed: an insert-only note at E4a reading it under ADR-301; accepted at
  E4c, the part that makes the write reachable, with the owner's decision at #266 (ADR-089)`.
- **T8** (section 1, the owner's decision, across the wrap). Old: `superseded for this path only, and
  CHARTER constraint 4 stands as written.` New: `superseded for this path only. CHARTER constraint 4
  keeps its sentence and, since ADR-301 (#514), adds that DeckStreak writes to the collection only
  through declared write classes; the skip day is the first.`
- **T9** (section 5, across the wrap). Old: `It amends no charter constraint: CHARTER constraint 4
  stands as written, and ADR-089 records the owner's decision (#266).` New: `It amends no charter
  constraint: CHARTER constraint 4 reads as ADR-301 amended it (#514), and ADR-089 records the
  owner's decision (#266).`
- **T10** (R19, appended). New: `The take's write may run in the one task the take itself spawns, so
  that R26's wait can end without cancelling it; that task runs the write once and never calls the
  take again (A27's census admits it by name).`
- **T11** (section 4, added rows). `docs/decisions/ADR-321-the-skip-day-lands-in-three-parts-as-adr-301s-first-declared-write-class.md`
  added; `docs/decisions/ADR-089-the-skip-day-writes-its-reschedule-back-to-anki-and-every-other-path-never-uploads.md`
  changed (an insert-only note); `docs/schematics/skip-day-record-and-effects.md` changed (the
  backup, the drill, the counts and the stop, drawn before E4b's code); `formal/tla/SkipDayOnce/` and
  `formal/lean/Formal/SkipTariff.lean` added; and E4b's
  `migrations/008303_ingest_write_class_stop.sql`.
- **T12** (section 9, added rows). `S08320-THE-CHARGE-IS-KEYED-TO-THE-SKIPS-DAY` and
  `S08321-THE-REFUND-IS-KEYED-TO-THE-UNDOS-DAY` (`crates/coordination/src/skip/mod.rs`, killer A46's
  test); `S08322` to `S08326`, one per skip constant typed in `crates/ingest/src/skip.rs`
  (`SKIP_SPREAD_MIN_DAYS`, `SKIP_SPREAD_MAX_DAYS`, `SKIP_DEFAULT_SEARCH`, `SKIP_MAX_CARDS`,
  `SKIP_BRIDGE_MONTHLY_CAP`), killer A45's test: cargo-mutants never mutates a const literal, so
  each value owes a hand row.
- **T13** (added after R33; ADR-301 (b) and (c)). Five new requirements, each built by the part 3c
  names:
  - `R34. The backup and its drill (ADR-301 (b); E4b).` After the converge and before the
    snapshot's commit, the take closes the converged working copy and copies it whole to one backup
    file beside the private copy, mode 0600, named for its skip. A restore drill copies the backup
    to a throwaway collection, opens it with the engine, and compares its R35 counts with the
    converged working copy's. A backup that cannot be written, or a drill whose counts differ, ends
    the take `failed` with a bounded reason before any card changes or any second sync, and tells
    the owner. The backup holds the collection file only: the class writes no media. DeckStreak
    never restores it, because a restore reaches the server only by a full upload, which ADR-301 (a)
    4 forbids; the owner restores it by the owner's own import. One backup is kept: a take's backup
    replaces the last only after its own drill passes. The data-rights erase removes it; the export
    leaves it out, since it is the owner's own collection. It is a write batch's own act, not one of
    ADR-064's standing copies (ADR-301 (b), #518 note). The backup is host-only: it is never in a
    bucket, never in Litestream's replica and never in ADR-064's daily copy, and the older backup is
    removed only after the newer one passes its check. ADR-064's "never" covers standing copies, not
    a write batch's own backup.
  - `R35. The counts (ADR-301 (b); E4b, E4c).` On the converged working copy, and again after the
    reschedule and before the push, the take counts the cards, the notes, the review-log rows, the
    cards in each queue and type pair, and the review cards the wrapped search (R3) selects as due.
    Two counts may move: the review-log rows, up by exactly the cards moved, each new row of type 4
    with ease 0 (R18); and the wrapped search's due count, down by exactly the cards moved. Any other
    count that moves ends the take before its push, writing nothing, sets the class's stop (R36) and
    tells the owner which count moved. The undo counts the same way on its own working copy and may
    move only the wrapped search's due count, up by the cards it restores. The read-back after a
    push is not counted, because another client's changes reach it; R27 lists them.
  - `R36. The class's stop (ADR-301 (c); E4b, E4c).` One ledger row says whether every declared
    write class is stopped, why (the owner, or a count that moved) and since when. While it is set,
    the preview says so, and the take and the undo refuse with `writes_stopped` before any request
    or write; a take or an undo already running reads it again before its push and ends writing
    nothing when it is set. A count that moved sets it; only the owner clears it, by the bot's
    owner-only command after a confirm (E4c), and the owner may set it the same way. The row records
    who set it and why. Only the owner's authenticated command handler clears it, and a later part's
    test proves that no other path does. While it is set nothing writes, an undo included. The owner
    may reshape it by a later appended amendment.
  - `R37. Dwell, band and change points (ADR-301 (c) and (b)).` No dwell time between changes
    applies: the class's ceiling is the approval rung (ADR-089, #518 note), so every batch is the
    owner's own confirm, and R2 already holds a study day to one pending or applied skip not undone.
    No band applies: a band is what a result must clear before a change is reversed, and a skip is
    reversed only by the owner's undo (R29), never by a guard metric, since the class has no
    autonomous rung, no trial and no guard metric. No change point applies: the class edits no note.
    The change budget stays ADR-089's: R21's `SKIP_MAX_CARDS`.
  - `R38. The formal decision (ADR-301, its last line).` ADR-321 records it: TLA+ `SkipDayOnce`
    (the record, the charge and the refund; E4a), TLA+ `SkipDayWrite` (the take's write, E4b;
    extended with the undo, E4c) and Lean `SkipTariff` (the price and the amount paid; E4a), each
    citing #108.
- **T14** (section 3c, the amendment's criteria for later parts). A47 (E4b): `the take's backup
  passes its restore drill before any card changes, and a backup whose drill fails ends the take
  with nothing written and nothing pushed`; A48 (E4b): `the take's counts move only the review-log
  rows and the wrapped search's due count, by exactly the cards moved, and a planted extra change
  ends the take before its push and sets the class's stop`; A49 (E4b): `while the class's stop is
  set the take refuses with writes_stopped before any request or write, and a stop set during a take
  ends it before its push`; A50 (E4c): `the undo's counts move only the wrapped search's due count,
  and a planted extra change ends it writing nothing`; A51 (E4c): `only the owner's command sets and
  clears the class's stop, each after a confirm`. Each names its test as section 3's rows do; the
  part that delivers it writes the name.
- **T15** (section 9, row S08306). Old: `the charge taken through the purchase port, outside the daily
  loss cap` New: `the charge taken through the floor-clipped debit, outside the daily loss cap`.

Section 4's rows this part does not build, each delivered by the part named: `crates/ingest/src/skip_write.rs`
(E4b, the undo's arm E4c); `crates/ingest/src/engine.rs` (E4b); `crates/ingest/tests/skip_write.rs`
(E4b); `crates/ingest/tests/skip_undo.rs` (E4c); `crates/ingest/tests/skip_zero_upload.rs` (E4b,
the undo's arms E4c); `crates/ingest/tests/skip_census.rs` (E4b);
`crates/ingest/tests/support/synthetic.rs` (E4b); `crates/coordination/tests/skip_callers.rs`
(E4c); `crates/api/src/skip_routes.rs`, `crates/api/src/router.rs` and
`crates/api/tests/skip_routes.rs` (E4c); `crates/bot/src/skip_commands.rs`,
`crates/bot/src/commands.rs` and `crates/bot/tests/skip_commands.rs` (E4c);
`crates/daemon/src/wiring.rs` (E4c); every `web/app/` row (E4c); `.cargo/config.toml` (E4b);
`deploy/deck-streak.env.example`, the two units and `scripts/tests/test_deploy_templates.py` (E4b);
`docs/OWNER-SETUP.md` (E4c); and ADR-083's acceptance (E4c, T7). `crates/economy/Cargo.toml` stays
unchanged: SPEC-082 added `serde` and `serde_json` as dev-dependencies, so row 620's condition does
not arise. `crates/economy/src/tariff.rs` also reads what a skip paid, by its source and reference,
since the wallet's ports answer no such read and the refund owes exactly that amount (T4).

## 11. Acceptance criteria of the 2026-10-03 amendment

| id | criterion | decided by |
|---|---|---|
| A45 | every skip constant equals `goldens/skip.constants.json` | `the_skip_constants_equal_the_predecessors` |
| A46 | a settlement retried on a later study day charges the tariff once, and an undo's refund retried on a later day credits once | `the_tariff_and_its_refund_are_taken_once_per_skip` |

```acceptance
A45: cargo test -p deck-streak-ingest --test skip_record -- --exact the_skip_constants_equal_the_predecessors
A46: cargo test -p deck-streak-coordination --test skip_flow -- --exact the_tariff_and_its_refund_are_taken_once_per_skip
```

## 12. Amendments, 2026-10-03, continued: the vectors' consumer, two census lines and one correction

Appended after section 11 and insert-only, as section 10 is: nothing above this heading changes.

- **T16** (section 4, added row). `crates/economy/tests/formal_vectors_skip_tariff.rs` added,
  `deck-streak-economy`: the Rust consumer of `formal/vectors/skip-tariff.jsonl`, in the house form
  of `crates/economy/tests/formal_vectors_wallet.rs`. For every vector `lean/SkipTariff` writes,
  economy's `price`, the floor-clipped debit of that price over a wallet holding the vector's
  balance, and the refund of what `paid_on` reads back answer as the port does.

Correction: T3's New text says "A price of 0 makes no call.", which the code does not do.
`settle_applied` asks `debit_floored_on` for the price even when it is 0; the wallet answers a
request of 0 as nothing requested and writes no movement, so a free skip pays nothing and moves no
coin. `settle_undone` likewise asks `refund_on` for what the skip paid, and a refund of 0 is
answered as not positive and credits nothing.

- **T17** (section 4, edited row). `crates/economy/tests/wallet_census.rs`, `deck-streak-economy`: the census of the
  files that name the coin ledger admits `crates/economy/src/tariff.rs`, whose `paid_on` reads what a skip paid by its
  source and reference, as section 10 says it does. Without that one line the census fails at the green code.
- **T18** (section 4, edited row). `crates/coordination/tests/relight_order.rs`, `deck-streak-coordination`: its census
  of linked statics counts 18, not 17, for the tariff's `static LADDER: LazyLock<Vec<i64>> = LazyLock::new(parse);`,
  counted as progression's `static XP` is. Without that one change the census fails at the green code.

Section 4's rows this part does not build, one line each, as the preflight's manifest reading wants them (section 10 names the same
rows in prose, with the part that delivers each):

- `crates/ingest/tests/skip_zero_upload.rs`: unchanged in this part; delivered by E4b.
- `crates/ingest/tests/skip_census.rs`: unchanged in this part; delivered by E4b.
- `crates/ingest/tests/support/synthetic.rs`: unchanged in this part; delivered by E4b.
- `.cargo/config.toml`: unchanged in this part; delivered by E4b.
- `deploy/deck-streak.env.example`: unchanged in this part; delivered by E4b.
- `crates/economy/Cargo.toml`: unchanged, as section 10 says.
- `crates/coordination/tests/skip_callers.rs`: unchanged in this part; delivered by E4c.
- `crates/api/src/skip_routes.rs`: unchanged in this part; delivered by E4c.
- `crates/api/src/router.rs`: unchanged in this part; delivered by E4c.
- `crates/api/tests/skip_routes.rs`: unchanged in this part; delivered by E4c.
- `crates/bot/src/skip_commands.rs`: unchanged in this part; delivered by E4c.
- `crates/bot/src/commands.rs`: unchanged in this part; delivered by E4c.
- `crates/bot/tests/skip_commands.rs`: unchanged in this part; delivered by E4c.
- `crates/daemon/src/wiring.rs`: unchanged in this part; delivered by E4c.
- `web/app/src/routes/skip/+page.svelte`: unchanged in this part; delivered by E4c.
- `web/app/src/lib/skip/SkipSheet.svelte`: unchanged in this part; delivered by E4c.
- `web/app/src/lib/skip/api.ts`: unchanged in this part; delivered by E4c.
- `web/app/src/lib/skip/skip-sheet.test.ts`: unchanged in this part; delivered by E4c.
- `web/app/src/lib/routes.ts`: unchanged in this part; delivered by E4c.
- `docs/OWNER-SETUP.md`: unchanged in this part; delivered by E4c.

Files this part edits that section 4 does not list, each a consequence of a row above:

- **T19** (section 4, added rows). `crates/coordination/tests/lapse.rs` passes the empty skip set to `open_lapse`, whose
  signature now takes it. `crates/ingest/tests/settings.rs` gains the test of the skip search's one-expression
  refusal by name. `formal/lean/Formal.lean` imports `Formal.SkipTariff`, and `formal/lean/Formal/Vectors.lean` imports
  `Formal.SkipTariffVectors` and routes the entry `SkipTariff` to it; `formal/lean/Formal/SkipTariffVectors.lean` is the
  writer of `formal/vectors/skip-tariff.jsonl`, in the house form of the wallet's. `formal/tla/RelightOrder/RelightOrder.tla`
  carries the restamped digest of its `govern` cover, after the skip set entered `govern`.
