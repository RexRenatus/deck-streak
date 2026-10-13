---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-423: one tap runs the client's own sync once a late review settles, and the daily sync stays as it is

Decides SPEC-409. A review of a card past its due day already says, in one line, that it does not
count toward the streak for that day (SPEC-376, ADR-387). The learner who has just answered such a
card is offered one tap that syncs this device's reviews now, beside the session's own syncs and
the server's daily sync, which stay exactly as they are.

## Context and Problem Statement

- The web review marks a late card from the engine core's predicate, `late` on the card view
  (`web/app/src/lib/engine/protocol.ts:145`), and shows the late line above the card
  (`web/app/src/lib/study/ReviewScreen.svelte:190-192`).
- The web client syncs twice a session on its own: once after the open and once before the close,
  neither awaited, through the Worker's one queue (`web/app/src/lib/study/engine.ts:5-6`, `:37-39`,
  `:76-83`, `:108-114`; SPEC-377 R7, ADR-388 D12). The Sync page's "Sync now" runs the same sync on
  demand and then reads the unsynced count (`web/app/src/lib/sync/SyncScreen.svelte:111-120`).
- The server syncs the collection once a study day, claimed per study day, and the
  owner can ask for one more through the bot's `/sync` (ADR-037, decision section;
  `crates/coordination/src/jobs.rs:62-69`; `crates/bot/src/commands.rs:244-254`). No Mini App route
  reaches that trigger.
- The native app has no late line (#738) and no sync call at all (#633; SPEC-358 part c).

The question is what one tap after a late review runs, when and where it is offered, which clients
offer it, in which words, how it is proved, and in which delivery.

## Decision Drivers

- The server's sync cadence and the session's two syncs are decided records; this delivery adds a
  path and changes neither.
- One act has one name across the app.
- The offer never blocks the review: no dialog, no focus move, no wait before the next card.
- No pressure: no streak-loss words, no deadline, no alarm.
- Reach by touch, keyboard and screen reader, as the rest of the review screen.
- Every criterion is decided by a test the base fails for the criterion's own reason.

## Considered Options (the alternatives each was chosen against)

### D1. What one tap runs

- Chosen: the client's existing normal sync through the Worker's one queue, then one read of the unsynced count, the same two calls the Sync page's "Sync now" makes, because it moves this device's reviews to the sync server with code already proved, adds no route and no server path, and its answer already names every outcome the learner must see.
- Chosen against: asking the server to run its own sync now from the Mini App, because no Mini App route reaches the owner's trigger (its request directory is writable by the bot's unit alone, `docs/schematics/sync-request-doorbell.md`), and the server's once-a-study-day cadence is ADR-037's, which this delivery keeps.
- Chosen against: a re-read of the streak or progress alone, because the server's views change only after the server's own sync, so a re-read without a sync shows the same numbers and tells the learner nothing new.
- Chosen against: offering the full-sync choice inside the review, because that choice can replace one copy of the collection with the other and belongs on the Sync page, where its warnings already are.

What the learner sees, by the sync's answer: while it runs, the line "Syncing…" and the button
disabled; a held answer that asks for no choice, "Sync finished." and "Reviews waiting to sync: N";
offline, the Sync page's offline line, with "Sync now" enabled again for a retry; signed out or a
sign-in that ended, that status line and a link to the Sync page; a full sync required, the Sync
page's line and the same link; a refusal, the review's engine-refusal line, with "Sync now" enabled
again.

### D2. When and where the offer shows

- Chosen: once the first late review of the screen's life settles (a rate of a card the engine marks late, answered by the engine), below the control bar, until the learner dismisses it, because one offer per sitting is enough to be found, a learner answering a backlog of late cards is not asked again and again, and a block below the controls moves neither the card nor the answer buttons when it appears.
- Chosen against: raising it again at each later late review after a "Not now", because a learner catching up answers many late cards in a row, and an offer repeated after a no is nagging.
- Chosen against: once per study day, because the review screen holds no clock of the study day and no stored state, and adding either is new state kept for a prompt.
- Chosen against: at the session's end, because the session already syncs as it closes, so an offer then adds nothing the close does not do.
- Chosen against: above the card, beside the late line, because the late line belongs to the card shown and changes with it, while the offer is about answers already given, and a block above the card shifts the card and the answer buttons when it appears.
- Chosen against: a dialog, or the host app's own popup or main button, because each takes the focus or the screen, and so blocks the review the offer must never block.
- Chosen against: raising it when a late card is shown but not yet answered, because the offer is about a review that exists to sync, and a card shown but not answered has none.

Its reach: two native buttons, "Sync now" and "Not now", at the screen's existing minimum control
height; its line in a status region present from the screen's mount, so the offer and each result
are announced politely without taking the focus; "Not now" hides the offer for the rest of the
screen's life. Keys typed while the focus is on either button belong to the button, as on every
control of the screen (`web/app/src/lib/remote/keys.ts:21-30`).

### D3. Clients in scope

- Chosen: the web Mini App now, and the native app out of scope, carried by #738 (its late line) and #633 (its sync), because at the base the native app has neither a late line nor any sync call (`ios/App/Sources/EngineSession.swift:8-15`), so it has no late review to follow and no sync for a tap to run.
- Chosen against: building the native offer in this delivery, because it would first need the native sync and the native late line, two deliveries already tracked, and it would turn one small web delivery into three.

### D4. The words, and every locale

- Chosen: the button reuses the Sync page's `sync_now` ("Sync now"), the running and count lines reuse `sync_working` and `sync_unsynced`, the status lines reuse the Sync page's own, and four new keys carry the rest in all seven locales, because one act keeps one name across the app and the reused strings are already translated and pinned (`web/app/src/lib/sync/sync-locales.test.ts:11-40`).
- Chosen against: the label "Update now", because the app already names this act "Sync now" on the Sync page, and two names for one act break consistent identification; renaming the Sync page's button is outside this delivery.
- Chosen against: words about the streak, a deadline or a loss, because the late line already says plainly what the streak counts, and an offer that adds pressure is a dark pattern this product refuses.
- Chosen against: the English text as a fallback in the other six locales, because every screen string of the app ships in all seven, and a census already holds that for the late line (`web/app/src/lib/study/late-line.test.ts:186-214`).

The four new keys, in English: `study_update_prompt` "Your reviews stay on this device until they
sync. You can sync them now."; `study_update_dismiss` "Not now"; `study_update_done` "Sync
finished."; `study_update_open_sync` "Open the sync page". Each other locale writes its own
translation in the register its own `sync_status_offline` uses, holds no word for the streak and
no exclamation mark, and is never the English text.

### D5. The tests and the model

- Chosen: red-first vitest tests, one per criterion, through the review screen with a fake client that counts each call, beside unit tests of the one new module; a locale census and a source census, each with planted refusals; the session's sync test and the job table's daily-sync test kept as not-red guards; StrykerJS over every changed production file at break 100, because a web change with no row table is proved by StrykerJS, and each criterion is then decided by a test the base fails for the criterion's reason.
- Chosen against: hand mutation rows in the band, because the rows table holds Rust, and a hand row over TypeScript repeats what StrykerJS judges whole on the pull request.
- Chosen against: the shipped browser study suite as the decider, because its seeded cards are all new and none is late, so it cannot show the offer without a new seeded collection; CI's run of it stays a guard.
- Chosen against: a new TLA+ model of the tap, because the tap is one more caller of the client's sync, whose every request passes the Worker's one queue in order, and the one model whose state a sync moves already fires its sync action at every interleaving (`formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla:116-119`, `:199`); no file a model covers is edited.

### D6. The shape

- Chosen: one delivery under SPEC-409 and ADR-423, with `docs/schematics/web-study-screens.md` amended by one appended section, because the offer is a new behaviour with its own criteria, and SPEC-376's criteria for the line stay as they are.
- Chosen against: an insert-only amendment of SPEC-376 and ADR-387, because it would add a sync path to a delivery whose record moves no data, and its criteria table would then mix two behaviours.
- Chosen against: an amendment of SPEC-377 and ADR-388, because those decide when the session syncs on its own, and the offer adds a learner's act while changing none of their rules.

### D7. The seam between the review and the sync

- Chosen: the review screen takes the sync client as its own optional property, which the review route gives from the same engine, and a screen given none shows no offer, because every fake client in the shipped review tests implements no sync, and they then keep passing unchanged.
- Chosen against: widening the review's `StudyClient` with the sync calls, because every fake of it in the shipped tests would have to grow a sync it never uses, and the review's machine would then hold an act that is not a review.

## Decision Outcome

One tap after the first late review of the screen's life runs the client's own sync and then reads
the unsynced count, and the learner reads the result in the offer's line. The offer sits below the
control bar, takes no focus, and goes away for the screen's life on "Not now". The web Mini App
ships it now; the native app follows its own late line (#738) and sync (#633). The server's daily
sync and the session's two syncs keep every rule they have.

## Consequences

- Good: a learner who has just answered a late card can send the session's reviews to the sync
  server at once, in one tap, without leaving the review.
- Good: no new route, no server change, and no new state beyond the screen's own.
- Neutral: once a tap has synced, the answer just given can no longer be undone, as after any sync;
  the undo offer already says so (SPEC-371).
- Neutral: an offer raised by a late answer that is then undone stays up, and a tap then moves the
  rest of the session's reviews.
- Bad: one more region on the review screen, and four more keys in seven locales.

### Confirmation

SPEC-409 section 3's tests, red first at the base; StrykerJS on the pull request with no survivor;
the owner's check on a device (SPEC-409 section 7, V1).

## What would make this wrong

- If the learner needs the server's own views (streak, progress) to change on the same day: that
  needs the server's sync, which stays the daily run and the owner's trigger (ADR-037).
- If a sync during a review measurably delays the next card: the Worker's queue orders requests,
  and SPEC-409 A9 holds the review moving while a sync is pending.
- If the native app gains its sync and late line and the two clients' offers drift apart: #738 and
  #633 carry the native half under this record's words.
