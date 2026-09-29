# SPEC-104: the doomscroll rail judges each ping, and fines only what the evidence still holds

- **Wave:** W5. **Issue:** #110 (the doomscroll rail), #111 (the confession), #105 (the instant
  loop), in epic #6. **Context(s):** `deck-streak-discipline` (the ping grammar and its token, the
  rail's events, state and sprints, the verdict, the fine's rung and its de-escalation, the canary,
  the chest lock and its ransom, the acknowledgement and the free spin, and the revision rules of
  ADR-104); `deck-streak-coordination` (the use cases that join the rail to the governor, the skip
  set, the wallet, the fine port, the grant port and the router, and the rail's step of the sync
  cycle); `deck-streak-bot` (the source's posts, `/tripwire`, `/confess` and the reply's buttons);
  `deck-streak-api` (the rail's routes); the Mini App (`web/app`, the discipline screen).
- **Decided by:** ADR-104 (this SPEC's: a discipline verdict is provisional; a fine is re-judged
  within the revision window and refunded when its evidence fails, and a reward is paid only once
  its evidence confirms it), ADR-103 (the fine port), ADR-012 (the parity oracle), ADR-041 (the one
  router) and ADR-038 (credentials come from the secret manager at start).
- **Prerequisites:** SPEC-103 (the fine and reversal ports), SPEC-105 (the committed window the
  verdict reads), SPEC-082 (the wallet, the scroll pass and the surcharge's state), SPEC-076 (the
  governor's verdict), SPEC-083 (the skip set), SPEC-071 (the day's reviews), SPEC-040 (the grant
  port), SPEC-041 and SPEC-084 (the router and its tiers), SPEC-026 (the bot's updates) and SPEC-023
  (the obligation port). **Mutation band:** `S10400-S10499`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-104.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` 26263de, `crates/discipline/src/` holds only `lib.rs`. SPEC-026 left the
  reading of the rail's posts to this wave (#110), SPEC-082 left the surcharge's setter and the
  pass's effect on a ping to it, and SPEC-081 left the chest lock and its ransom to it.
- **The sensor is a port.** A ping is a line of text a bound source posts where the bot reads it.
  The grammar is `KIND|...|<seconds>|<token>`: `DS|example_app|1700000000|abcd1234` opens a
  distracting app, `DE|example_app|1700000900|abcd1234` closes it, `AO|1700000000|abcd1234` opens
  the study app, and `TW|test|1700000000|abcd1234` is the owner's test. How a source produces pings
  is the owner's configuration and outside DeckStreak. Binding a source is the owner's act (#170):
  until a source is bound, no ping but a valid test is read, nothing is recorded and nothing fines.
- **What is ported** (at `27ee2bc`):
  - the grammar and the token, `tripwire.py:parse`, `token_of` and `valid`;
  - the post's handling, `pipeline_layers/tripwire.py:TripwireLayer.handle_tripwire_post` (the
    handshake that binds a source, a spoof, the per-app rate, the daily cap, the test);
  - the verdict, `TripwireLayer._doomscroll_verdict`, and the session's close and its grace,
    `_close_session` and `_grace_defection`;
  - the fine's rung, `book_defection_fine`, the sprint, `start_sprint` and
    `_resolve_sprints_and_fines`, the snooze, `snooze_tripwire`, the canary, `_tripwire_canary`, and
    the rung's de-escalation, `_deescalate_rung`;
  - the confession, `TripwireLayer.confess`;
  - the instant loop, `_instant_loop_ack` and `_confirm_free_spin`;
  - the chest lock's ransom, `pipeline_layers/loot.py:LootLayer._chest_lock_active`.
- **Measured outcome sets.**
  - A post: not ours (no answer, nothing recorded), `spoof`, `rate_dropped`, `test`, `ack`, `free`,
    `defection`, `duplicate`, and for a close `orphan_close`, `grace` or `session`.
  - The verdict is `free` when any holds: the governor is not armed, a skip is active, discipline is
    off, quiet hours, an active scroll pass, a snooze, the rail suspended, the rail unverified, or
    neither an idle defection (scope `reviews` or `both`, and the study day's reviews below 15, the
    predecessor's `quests.Q1_TARGET`) nor a window defection (scope `windows` or `both`, and the
    instant inside a committed window).
  - The fine's booking refuses with `engine disarmed` (not armed, unverified, suspended or off),
    `skip day` or `already settled` (no such event today, already fined, or graced).
  - A sprint is `kept` (at least 10 distinct cards from its tap to its deadline, judged 15 minutes
    after the deadline) or `failed`, which books the fine.
  - A confession is fined (10 coins), free (the rail disarmed, discipline off, a skip, or a fine
    already landed on the study day) or `already logged`. An ack is silent (quiet hours or no
    transport), draws the day's free spin (the first ack), answers "caught you studying" (while at
    most 3 acks are counted) or answers nothing. A spin is confirmed (a review within 45 minutes),
    pending, or closed ungranted (60 minutes after it, unconfirmed).
  - The de-escalation waits (a defection inside its 45 minutes or a pending sprint), credits nothing
    (disarmed), resets on a breach, counts a clean day, or steps the rung down at 7 clean days. The
    canary suspends a verified rail silent for more than 72 hours, or does nothing. A chest lock is
    ransomed (20 distinct cards) or stands.
- **Corrections to the issues and the predecessor.**
  - The predecessor reads reviews every few minutes. DeckStreak reads them once a study day plus the
    owner's `/sync` (ADR-037), so at an open the day's review count is often the morning's. The
    open's verdict is therefore provisional: a defection settles at the first sync cycle that begins
    after its evidence window, on the reviews that cycle read, and an idle defection whose reviews
    before it reach 15 is cleared and fines nothing (ADR-104).
  - #105's free spin is granted in the predecessor when the owner opens the study app and set to 0
    if no review follows within 45 minutes. DeckStreak's grant port never lowers an amount
    (SPEC-040), so the spin is drawn at the ack, stored, and granted only once a review confirms it
    (ADR-104); an unconfirmed spin is never granted.
  - The predecessor sends a dice beside the free spin outside the celebration ladder. Here the spin
    is a celebration on the ladder (SPEC-084), so its weekly budget holds it.
  - The predecessor's copy names the owner's device and its automation. DeckStreak's copy names the
    rail and its source only.
  - A fine the evidence no longer supports is refunded by ADR-104's revision, a rule the predecessor
    applies only to the grace.
- **What the parity oracle proves.** The grammar, the token and its skew; every post's outcome; the
  verdict over every input; the close and the grace; the rung, the base, the long session's scale
  and the escalation; the sprint; the confession; the canary; the de-escalation; the ransom; the ack
  and the spin's confirmation; every constant (section 7).

## 2. Requirements

The rail's source and its pings (#110, #170)

R1. The bot asks for channel posts beside the owner's messages and taps. Its gate admits a channel
    post only as a rail post, the text and the source's id, handed to the rail's port and never to a
    command, a callback or the owner's dispatch; a post over the inbound cap is dropped. A text that
    does not parse is not the rail's: nothing is recorded and nothing answers. The grammar, the
    app's name (lowercased, its first 24 characters, letters, digits, `_` and `-`) and the kinds
    `ds`, `de`, `ao` and `test` equal the golden of `parse`.
R2. A ping is valid when its token equals the first 8 characters of the rail's secret and its
    instant lies within 15 hours of now (the golden of `valid`). The secret is the credential
    `tripwire-secret`, read at start by the bot role through its credential loader (ADR-038). A
    missing credential reads as none, so no ping is valid; an empty one refuses start (SPEC-066 R1).
    No unit names it until a source is bound (#170): the delivery that binds one adds the bot unit's
    `LoadCredential=` line and the rail's answer.
R3. While no source is bound, a valid test ping binds its source and every other ping is ignored:
    nothing is recorded, nothing answers and nothing fines. A ping from any other source than the
    bound one is ignored. An invalid ping from the bound source records `spoof` and answers nothing.
R4. A valid ping from the bound source records its instant as the last ping and lifts a suspension.
    A close pairs with its open; a test marks the rail verified on the study day and records `test`.
    An open or an ack from one app within 300 seconds of its last counted ping, or once 40 pings are
    counted on the study day (free, defection and ack), records `rate_dropped`. Every outcome equals
    the golden of `handle_tripwire_post`.
R5. `tripwire_events` is unique on (app, instant): a second ping of one app at one instant records
    nothing and answers `duplicate`.

The verdict and the reply (#110)

R6. An open is judged by a pure function of these inputs: the governor armed, a skip active on the
    study day, discipline on, inside quiet hours, a scroll pass active, a snooze active, the rail
    suspended, the rail verified, the scope, the study day's reviews and the instant inside a
    committed window. It equals the golden of `_doomscroll_verdict` over every combination the
    golden lists.
R7. Coordination gathers the inputs: the governor's verdict (SPEC-076), the skip set (SPEC-083),
    discipline's switch (`discipline_state`, SPEC-105), the router's quiet window (SPEC-041), the
    pass (SPEC-082), the rail's state, the study day's review count as of the latest recompute
    (SPEC-071) and discipline's window check (SPEC-105).
R8. A defection is answered at once through the router under the kind `discipline`, naming the app
    and, when the open lies inside a committed window, that window, with three buttons: a sprint
    (`tw:sprint:<event>`), the fine (`tw:fine:<event>`) and the snooze (`tw:snooze`). The reply
    states no review count, because none is final before settlement (R10). A free verdict answers
    nothing.

The session, the grace and the sprint (#110)

R9. A close pairs with the latest unpaired open of its app within 6 hours before it; a pair shorter
    than 0 or longer than 4 hours, or no open, is `orphan_close`, and no duration is assumed. A
    defection closed within 60 seconds is graced: its fine, if one was booked, is reversed through
    the fine port with the reason `grace` (SPEC-103), and the rung stands. Each equals the golden of
    `_close_session`.
R10. A tap on the sprint opens one sprint per event, whose deadline is 30 minutes after the tap. At
    the first sync cycle (SPEC-023 R12) that begins 15 minutes or more after the deadline, the
    sprint is kept when at least 10 distinct cards were reviewed from the tap to the deadline, and
    otherwise it fails and books the fine. A defection with no sprint and no fine settles at the
    first sync cycle that begins 45 minutes or more after its instant: it is cleared, and fines
    nothing, when it is idle only (not inside a window) and the study reviews timestamped before its
    instant reach 15; otherwise it books the fine. Over the reviews a cycle read, each equals the
    golden of `_resolve_sprints_and_fines`, and the clearing is ADR-104's.
R11. The snooze makes every open free for 30 minutes, once per study day; a second is refused.

The fine and its rung (#110)

R12. Booking a defection's fine refuses, in this order, with `engine disarmed`, `skip day` or
    `already settled` (R-outcomes in section 1), changing nothing. Otherwise the rung is the larger
    of the stored rung and the study day's fined defections that are not graced; the request is the
    scaled fine of 20 over the wallet (SPEC-082 R6), times 1.5 when the session is known to have
    lasted 30 minutes or more; the fine port debits it under the reference `tw:<event>`; the event
    records the debited amount, at least 1; and the stored rung becomes the smaller of 2 and the
    rung plus one. Each equals the golden of `book_defection_fine`.
R13. At rung 1 or more the fine sets a chest lock on the next study day, stamped with the fine's
    instant; at rung 2 it also sets the scroll pass's surcharge to end 48 hours after the fine,
    through economy's surcharge port (SPEC-103). The fine's message names the coins and each effect.
R14. A chest lock stands unless at least 20 distinct cards were reviewed from its instant to the
    start of its study day (the golden of `_chest_lock_active`): a ransomed lock is lifted and says
    so once. The lock's port answers whether it stands for a study day, and consumes it; its effect
    on a chest is SPEC-108's.
R15. Once a study day, the rung judges the day before: it waits while a defection of that day is
    still inside its 45 minutes or a sprint of that day is pending; it credits nothing while the
    rail is disarmed; a fined defection resets the clean count to 0; 7 clean days step the rung down
    by one and reset the count, and the step-down is announced once. It equals the golden of
    `_deescalate_rung`, and a skip day stays creditable.
R16. When the rail is verified and not suspended, and its last ping is more than 72 hours old, it is
    suspended and the owner is told once a week (the golden of `_tripwire_canary`). A suspended rail
    fines nothing; a valid ping lifts the suspension (R4). A silent source never fines.

The confession (#111)

R17. A confession records a `confess` event at its instant with the chosen duration (15m, 30m, 1h or
    2h+), which the answer echoes and the event does not store. It fines 10 coins through the fine
    port under `confess:<event>` when the governor is armed, discipline is on and no skip is active,
    and nothing when a defection fine already landed on the study day; a second confession at one
    instant answers `already logged`. It equals the golden of `confess`.

The instant loop (#105)

R18. A valid `ao` ping records an `ack`. Outside quiet hours and with a transport, the first ack of
    the study day draws the free spin, a whole number of XP from 10 to 30 inclusive from the
    operating system's generator, stores it with its instant, and raises the celebration `free_spin`
    (the ladder's unknown event, T2); each later ack while at most 3 acks are counted answers
    "caught you studying". It equals the golden of `_instant_loop_ack`, the draw replaced by the
    case's value.
R19. The spin is confirmed when a review lands within 45 minutes of it, and only then granted, once,
    on its study day, through the grant port (scope `per-day`, source `freespin`). An unconfirmed
    spin, at the first sync cycle that begins 60 minutes or more after its instant, is closed
    ungranted and never granted later. The confirmation equals the golden of `_confirm_free_spin`,
    with its void read as "never granted" (ADR-104).

Revision (ADR-104)

R20. At each sync cycle, every fine of a study day inside the revision window (the 7 closed study
    days before the current one, and the current one) that is not reversed is judged again on the
    evidence as it now stands, and reversed with the reason `revision` when it no longer holds:
    - `tw:<event>`: the event's study day now holds an applied skip; or the defection was idle only
      (not inside a window) and the study reviews timestamped before its instant now reach 15; or
      its sprint, measured again, is kept;
    - `confess:<event>`: the event's study day now holds an applied skip.
    A reversed fine is never booked again (SPEC-103 R7), and its rung, lock and surcharge stand.
R21. The rail's deadlines (a sprint's deadline plus 15 minutes, an unanswered defection's 45
    minutes, a spin's 60 minutes, and the canary's 72 hours) register as one source with SPEC-023's
    obligation port, so the first cycle after each recomputes. The rail's step runs in the cycle
    after the recompute, in this order: the settlement (R10), the revision (R20), the de-escalation
    (R15), the canary (R16) and the spin's confirmation (R19).

Tables, surfaces and rights

R22. Discipline owns `tripwire_events` (the app, the instant, the verdict, the study day, the fine,
    the session's seconds, graced), `tripwire_state` (one row: the bound source, the verified study
    day, suspended, the last ping, the snooze's end and study day, the rung, its clean count and its
    last judged and stepped study days, the spin's study day, instant, amount and outcome, and the
    scope), `sprints` (one per event: tap, deadline, status) and `chest_locks` (one per study day:
    the instant, lifted or consumed), each `STRICT` with `created_at`, created by
    `migrations/010401_discipline_tripwire.sql`.
R23. Every rail message goes through the router under the policy's kind `discipline` (SPEC-105,
    ADR-104), except the free spin's celebration (R18). The sync's rail step (R15's step-down
    notice, R16's canary notice) records each message pending in the ledger, as SPEC-106 R17 does:
    when the cycle carries a router (the owner's sync), the cycle's flush delivers it at once;
    otherwise it waits for `discipline_tick` (SPEC-105 R12), which delivers it through the router on
    its next run. The rail's defections inside a committed
    window or a hard-mode night are passed to their evaluation (SPEC-105), so a window's `red` and a
    night's pings count them from this delivery on; neither fines them again.
R24. `/tripwire` shows the rail's state (bound, verified, suspended, the rung, today's counts) and
    how to send a test; `/confess` offers the four durations as buttons (`cf:<bucket>`). The buttons
    of R8 and R17 run the same use cases as the routes. `GET /api/discipline/rail` and `POST
    /api/discipline/confess` answer the owner's session only. The Mini App's discipline screen
    (SPEC-105) gains the rail's state and the confession's four chips.
R25. Discipline's data-rights port exports and erases the four tables (`tripwire_state` reset in
    place: unbound, unverified, rung 0), each declared in `privacy.json` and `PRIVACY.md` with its
    own-tables row and a seeded symmetry row (SPEC-021). An erase unbinds the source.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the grammar, the token and its skew equal the goldens of `parse` and `valid` | `the_ping_grammar_and_token_match_the_parity_goldens` |
| A2 | every post's outcome, the handshake, the spoof, the rate and the daily cap included, equals the golden of `handle_tripwire_post` | `every_post_outcome_matches_the_parity_golden` |
| A3 | with no source bound, an open, a close, an ack and an invalid test record nothing, answer nothing and fine nothing, and a valid test binds its source | `an_unbound_rail_reads_nothing_but_a_valid_test` |
| A4 | the verdict equals the golden of `_doomscroll_verdict` for every listed combination of its eleven inputs | `the_verdict_matches_the_parity_golden` |
| A5 | the use case passes each of the eleven inputs from its source, each proved by flipping that source alone | `the_rail_gathers_every_verdict_input` |
| A6 | a defection is answered through the router under `discipline` with its three buttons and no review count, and a free open answers nothing | `a_defection_is_answered_with_its_three_buttons` |
| A7 | a close, an orphan and the grace equal the golden of `_close_session`, and a graced fine is refunded once with the rung kept | `the_close_and_the_grace_match_the_parity_golden` |
| A8 | sprints and unanswered defections settle as the golden of `_resolve_sprints_and_fines`, each once | `sprints_and_unanswered_defections_match_the_parity_golden` |
| A9 | the snooze frees opens for 30 minutes once a study day, and a second is refused | `the_snooze_is_once_a_day` |
| A10 | the refusals, the rung, the base, the long session's scale, the recorded fine and the escalation equal the golden of `book_defection_fine` | `the_fine_rung_matches_the_parity_golden` |
| A11 | rung 1 sets the next day's chest lock, rung 2 also the 48-hour surcharge, and a second booking of one event sets nothing | `the_rung_sets_its_lock_and_surcharge_once` |
| A12 | the ransom equals the golden of `_chest_lock_active`, and a lifted or consumed lock no longer stands | `the_ransom_matches_the_parity_golden` |
| A13 | the de-escalation equals the golden of `_deescalate_rung`, waiting on an unsettled day and announcing a step once | `the_de_escalation_matches_the_parity_golden` |
| A14 | the canary suspends a silent verified rail as the golden of `_tripwire_canary`, and a suspended rail fines nothing | `a_silent_rail_is_suspended_and_fines_nothing` |
| A15 | the confession's fine and its free cases equal the golden of `confess`, and the event is counted on its study day | `the_confession_matches_the_parity_golden` |
| A16 | the ack, the spin's draw and the acknowledgement cap equal the golden of `_instant_loop_ack` with the draw injected | `the_ack_matches_the_parity_golden` |
| A17 | a confirmed spin is granted once through the grant port, and an unconfirmed spin is never granted, even by a review after its hour | `a_spin_is_granted_only_when_confirmed` |
| A18 | each revision rule reverses its fine once with the reason `revision`, a window defection is not revised by reviews, and a fine outside the window is not re-judged | `a_fine_is_refunded_when_its_evidence_fails` |
| A19 | a fine reversed by revision is not booked again when its event is settled again | `a_revised_fine_is_never_booked_again` |
| A20 | the rail's deadlines hold the change gate open once each | `the_rails_deadlines_open_the_gate_once` |
| A21 | the rail's constants equal the constants golden | `the_rail_constants_match_the_predecessors` |
| A22 | a window's and a hard-mode night's evaluation receive the rail's defections inside them, and a red window writes no fine | `the_rails_defections_reach_the_window_evaluation` |
| A23 | the rail's routes answer the owner's session only | `the_rail_routes_answer_only_the_owner` |
| A24 | `/confess` offers four durations whose buttons run the confession, and `/tripwire` shows the rail's state | `confess_and_tripwire_commands_run_the_use_cases` |
| A25 | the discipline screen shows the rail's state and the four confession chips | `shows the rail and the confession chips` |
| A26 | an erase empties the four tables, resets the state row and unbinds the source | `an_erase_empties_the_rail_and_unbinds_it` |
| A27 | the poll asks for channel posts, and the gate admits a channel post as a rail post only, never as the owner's message or command, and drops one over the cap | `a_channel_post_reaches_only_the_rail` |
| A28 | an idle-only defection whose study reviews before its instant reach 15 is cleared at settlement and fines nothing, one at 14 is fined, and a window defection is fined whatever the reviews; nothing settles before its cycle | `an_idle_defection_with_its_reviews_is_cleared_at_settlement` |
| A29 | a bot role started with no `tripwire-secret` starts and refuses every ping, and one started with an empty one refuses start, each through the credential loader and never the environment: a decoy environment variable `TRIPWIRE_SECRET` carrying a different value is never read, and the loaded file's value, or its absence, decides | `the_bot_role_reads_the_rails_secret_through_the_loader` |
| A30 | a scheduled sync (no router) records the rail step's message pending and sends nothing, the next `discipline_tick` delivers it exactly once, and an owner's sync delivers it at once and leaves nothing pending | `a_scheduled_sync_leaves_the_rails_messages_pending_for_the_tick` |

```acceptance
A1: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_ping_grammar_and_token_match_the_parity_goldens
A2: cargo test -p deck-streak-discipline --test rail_goldens -- --exact every_post_outcome_matches_the_parity_golden
A3: cargo test -p deck-streak-discipline --test rail_posts -- --exact an_unbound_rail_reads_nothing_but_a_valid_test
A4: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_verdict_matches_the_parity_golden
A5: cargo test -p deck-streak-coordination --test discipline_rail -- --exact the_rail_gathers_every_verdict_input
A6: cargo test -p deck-streak-coordination --test discipline_rail -- --exact a_defection_is_answered_with_its_three_buttons
A7: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_close_and_the_grace_match_the_parity_golden
A8: cargo test -p deck-streak-discipline --test rail_goldens -- --exact sprints_and_unanswered_defections_match_the_parity_golden
A9: cargo test -p deck-streak-discipline --test rail_posts -- --exact the_snooze_is_once_a_day
A10: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_fine_rung_matches_the_parity_golden
A11: cargo test -p deck-streak-coordination --test discipline_fines -- --exact the_rung_sets_its_lock_and_surcharge_once
A12: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_ransom_matches_the_parity_golden
A13: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_de_escalation_matches_the_parity_golden
A14: cargo test -p deck-streak-discipline --test rail_goldens -- --exact a_silent_rail_is_suspended_and_fines_nothing
A15: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_confession_matches_the_parity_golden
A16: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_ack_matches_the_parity_golden
A17: cargo test -p deck-streak-coordination --test discipline_spin -- --exact a_spin_is_granted_only_when_confirmed
A18: cargo test -p deck-streak-discipline --test rail_revision -- --exact a_fine_is_refunded_when_its_evidence_fails
A19: cargo test -p deck-streak-coordination --test discipline_fines -- --exact a_revised_fine_is_never_booked_again
A20: cargo test -p deck-streak-coordination --test discipline_rail -- --exact the_rails_deadlines_open_the_gate_once
A21: cargo test -p deck-streak-discipline --test rail_goldens -- --exact the_rail_constants_match_the_predecessors
A22: cargo test -p deck-streak-coordination --test discipline_rail -- --exact the_rails_defections_reach_the_window_evaluation
A23: cargo test -p deck-streak-api --test discipline_routes -- --exact the_rail_routes_answer_only_the_owner
A24: cargo test -p deck-streak-bot --test discipline_commands -- --exact confess_and_tripwire_commands_run_the_use_cases
A25: pnpm exec vitest run web/app/src/lib/discipline/rail.test.ts -t "shows the rail and the confession chips"
A26: cargo test -p deck-streak-discipline --test rail_rights -- --exact an_erase_empties_the_rail_and_unbinds_it
A27: cargo test -p deck-streak-bot --test channel_posts -- --exact a_channel_post_reaches_only_the_rail
A28: cargo test -p deck-streak-discipline --test rail_revision -- --exact an_idle_defection_with_its_reviews_is_cleared_at_settlement
A29: cargo test -p deck-streak-daemon --test roles -- --exact the_bot_role_reads_the_rails_secret_through_the_loader
A30: cargo test -p deck-streak-coordination --test discipline_rail -- --exact a_scheduled_sync_leaves_the_rails_messages_pending_for_the_tick
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, notifications-policy, game-economy,
ux-laws and accessibility packs stay enforced, and no row is deferred or lifted for this delivery,
so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | the four tables are declared under their categories with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010401_discipline_tripwire.sql` and `crates/discipline/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | no delivery call is made outside the router module, over every source file under `crates/` | the notifications-policy pack |
| B3 | only coins are confiscated and the fine's share and the loss cap still equal the reference economy, over `economy.json`'s `coins` and `unconfiscable` sections | the game-economy pack |
| B4 | the rail's copy carries no confirmshaming, no shame and no false urgency, over every file under `web/app/src/lib/discipline/` and `crates/coordination/src/discipline/` | the ux-laws pack |
| B5 | the discipline screen passes the accessibility audit in both colour schemes, over the `/discipline` route `web/app/src/lib/routes.ts` lists | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/discipline/src/lib.rs` | `deck-streak-discipline` | changed: the modules below |
| `crates/discipline/src/constants.rs` | `deck-streak-discipline` | changed: the rail's constants join SPEC-105's, proved by the golden |
| `crates/discipline/src/ping.rs` | `deck-streak-discipline` | added: the grammar and the token, pure |
| `crates/discipline/src/verdict.rs` | `deck-streak-discipline` | added: the verdict's inputs and its fusion, pure |
| `crates/discipline/src/rail.rs` | `deck-streak-discipline` | added: the events, the state, the binding, the rate and the post's outcomes |
| `crates/discipline/src/sprint.rs` | `deck-streak-discipline` | added: sprints, the snooze and the unanswered defections |
| `crates/discipline/src/rung.rs` | `deck-streak-discipline` | added: the booking's refusals, the rung, the base, the de-escalation and the canary |
| `crates/discipline/src/lock.rs` | `deck-streak-discipline` | added: the chest lock, its ransom and its port |
| `crates/discipline/src/instant.rs` | `deck-streak-discipline` | added: the ack, the spin's draw and its confirmation |
| `crates/discipline/src/revision.rs` | `deck-streak-discipline` | added: the revision window and the rules of R20 |
| `crates/discipline/src/data_rights.rs` | `deck-streak-discipline` | changed: the four tables join discipline's data-rights port |
| `crates/discipline/Cargo.toml` | `deck-streak-discipline` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tokio`, `getrandom`); `serde` and `serde_json` as dev-dependencies for the golden reader |
| `migrations/010401_discipline_tripwire.sql` | `deck-streak-discipline` | added: the four tables, `STRICT`, the state row seeded |
| `crates/discipline/tests/rail_goldens.rs` | `deck-streak-discipline` | added: A1, A2, A4, A7, A8, A10, A12 to A16, A21 |
| `crates/discipline/tests/rail_posts.rs` | `deck-streak-discipline` | added: A3, A9 |
| `crates/discipline/tests/rail_revision.rs` | `deck-streak-discipline` | added: A18, A28 |
| `crates/discipline/tests/rail_rights.rs` | `deck-streak-discipline` | added: A26 |
| `crates/coordination/src/discipline/mod.rs` | `deck-streak-coordination` | changed: the rail's modules |
| `crates/coordination/src/discipline/windows.rs` | `deck-streak-coordination` | changed: the rail's defections passed to the window's and the night's evaluation |
| `crates/coordination/src/discipline/rail.rs` | `deck-streak-coordination` | added: the post's use case, its inputs and its reply |
| `crates/coordination/src/discipline/fines.rs` | `deck-streak-coordination` | added: the booking, the confession, the grace and the revision step |
| `crates/coordination/src/discipline/spin.rs` | `deck-streak-coordination` | added: the spin's celebration and its grant at confirmation |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the rail's step after the recompute |
| `crates/coordination/src/obligations.rs` | `deck-streak-coordination` | changed: the rail's deadline source |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module above |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: discipline's port is registered by SPEC-105; listed under SPEC-021's six-file rule |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the four tables |
| `crates/coordination/tests/discipline_rail.rs` | `deck-streak-coordination` | added: A5, A6, A20, A22, A30 |
| `crates/coordination/tests/discipline_fines.rs` | `deck-streak-coordination` | added: A11, A19 |
| `crates/coordination/tests/discipline_spin.rs` | `deck-streak-coordination` | added: A17 |
| `crates/bot/src/poll.rs` | `deck-streak-bot` | changed: the channel post joins the kinds of update asked for |
| `crates/bot/src/gate.rs` | `deck-streak-bot` | changed: a channel post is admitted as a rail post only |
| `crates/bot/src/channel.rs` | `deck-streak-bot` | added: a rail post handed to the rail's port |
| `crates/bot/tests/channel_posts.rs` | `deck-streak-bot` | added: A27 |
| `crates/bot/src/discipline_commands.rs` | `deck-streak-bot` | changed: /tripwire, /confess and the `tw:` and `cf:` buttons join SPEC-105's commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: both commands join the command table |
| `crates/bot/tests/discipline_commands.rs` | `deck-streak-bot` | added: A24 |
| `crates/api/src/discipline_routes.rs` | `deck-streak-api` | changed: the rail's routes join SPEC-105's |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes, behind the owner's session |
| `crates/api/tests/discipline_routes.rs` | `deck-streak-api` | added: A23 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the rail joined to coordination, the bot and the secret |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role reads the rail's secret at start, a missing one read as none (R2), and hands the rail's port to the bot |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A29 |
| `deploy/README.md` | deploy | changed: the bot's optional credential id `tripwire-secret`, named by no unit until #170 |
| `web/app/src/lib/discipline/RailCard.svelte` | miniapp | added: the rail's state |
| `web/app/src/lib/discipline/ConfessChips.svelte` | miniapp | added: the four chips |
| `web/app/src/lib/discipline/discipline.ts` | miniapp | changed: the rail's routes join the client |
| `web/app/src/lib/discipline/rail.test.ts` | miniapp | added: A25 |
| `web/app/src/routes/discipline/+page.svelte` | miniapp | changed: the rail's card and the confession's chips join the discipline screen |
| `tools/parity-oracle/registry/spec_104.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/tripwire_parse.json` | repo | added: the golden of `tripwire.py:parse` and `token_of` (function) |
| `tools/parity-oracle/goldens/tripwire_valid.json` | repo | added: the golden of `tripwire.py:valid` (function) |
| `tools/parity-oracle/goldens/tripwire_post.json` | repo | added: the golden of `pipeline_layers/tripwire.py:TripwireLayer.handle_tripwire_post` (adapter) |
| `tools/parity-oracle/goldens/doomscroll_verdict.json` | repo | added: the golden of `TripwireLayer._doomscroll_verdict` (adapter) |
| `tools/parity-oracle/goldens/tripwire_close.json` | repo | added: the golden of `TripwireLayer._close_session` and `_grace_defection` (adapter) |
| `tools/parity-oracle/goldens/sprint_resolution.json` | repo | added: the golden of `TripwireLayer._resolve_sprints_and_fines` (adapter) |
| `tools/parity-oracle/goldens/defection_fine.json` | repo | added: the golden of `TripwireLayer.book_defection_fine` (adapter) |
| `tools/parity-oracle/goldens/chest_lock_ransom.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._chest_lock_active` (adapter) |
| `tools/parity-oracle/goldens/rung_deescalation.json` | repo | added: the golden of `TripwireLayer._deescalate_rung` (adapter) |
| `tools/parity-oracle/goldens/tripwire_canary.json` | repo | added: the golden of `TripwireLayer._tripwire_canary` (adapter) |
| `tools/parity-oracle/goldens/confess.json` | repo | added: the golden of `TripwireLayer.confess` (adapter) |
| `tools/parity-oracle/goldens/instant_ack.json` | repo | added: the golden of `TripwireLayer._instant_loop_ack` (adapter) |
| `tools/parity-oracle/goldens/free_spin_confirm.json` | repo | added: the golden of `TripwireLayer._confirm_free_spin` (adapter) |
| `tools/parity-oracle/goldens/tripwire.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S10400-S10499.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the four tables |
| `privacy.json` | repo | changed: the categories `discipline-rail` and `discipline-state` |
| `PRIVACY.md` | repo | changed: one line for each category |
| `docs/schematics/fine-verdict-and-refund.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-104-the-doomscroll-rail-judges-each-ping-and-fines-only-what-the-evidence-still-holds.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-104.md` | docs | added |
| `changelog.d/feat-discipline-rail-104.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It binds no source: the owner binds one (#170), and until then the rail reads nothing.
- It shows no setup screen for the scope, the switch or the source: the settings screen does (#57).
- It changes no chest roll: a standing lock renders the day's first chest common in SPEC-108 (#110).
- It books no committed window and judges none (#109), and no hard-mode night (#115).
- It pauses, pardons or panics nothing (#114), and it spends no smoke bomb (#279).
- It changes no sync cadence: the verdict settles at whatever cadence the owner keeps (#164).
- It imports none of the predecessor's rail rows (#61).

## 6. Risks

- **A false fine.** Every fine passes the booking's refusals, the loss cap and the revision (A18); a
  silent source suspends the rail (A14); an unbound rail reads nothing (A3).
- **A spoof reveals the rail.** A spoof answers nothing (A2), and a foreign source is ignored.
- **A retried tap fines twice.** The event's fine and the fine port's key hold one fine per event
  (A11, SPEC-103 A3).
- **A stranger's post reaches a command.** The gate admits a channel post as a rail post only (A27),
  the rail reads only its grammar from its bound source (A3), and a spoof answers nothing.
- **The reply arrives late.** The post's use case answers on receipt, not at the next sync (A6).
- **A spin becomes a faucet.** It is granted only on a confirming review, once a study day (A17).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_104.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch seconds or milliseconds as the function takes it; no golden holds a calendar date, a
real source id or a real token.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/tripwire_parse.json` | `tripwire.py:parse`, `token_of` | function | none; every kind, a 25-character app, a capital and a forbidden character, a non-numeric instant, three and five fields |
| `goldens/tripwire_valid.json` | `tripwire.py:valid` | function | none; an empty secret, a short token, a wrong token, and a skew of 15 hours exactly and one second over |
| `goldens/tripwire_post.json` | `TripwireLayer.handle_tripwire_post` | adapter | a stand-in layer over a temporary store database with a synthetic source id and secret; cases for unbound, foreign, spoof, test, close, rate at 299 and 300 seconds, the 40th and 41st counted ping, and a second ping of one app at one instant |
| `goldens/doomscroll_verdict.json` | `TripwireLayer._doomscroll_verdict` | adapter | the layer with each input stubbed: every input alone flipped from a defecting base, each scope, and reviews at 14 and 15 |
| `goldens/tripwire_close.json` | `TripwireLayer._close_session`, `_grace_defection` | adapter | pairs at 59, 60 and 61 seconds, an orphan, a pair of 4 hours and one second, and a second close of one pair |
| `goldens/sprint_resolution.json` | `TripwireLayer._resolve_sprints_and_fines` | adapter | synthetic reviews: 9 and 10 distinct cards in the window, a review one second after the deadline, a sync 14 and 15 minutes after it, and an unanswered defection at 45 minutes and one second over |
| `goldens/defection_fine.json` | `TripwireLayer.book_defection_fine` | adapter | wallets around the scaled fine's crossing, sessions of 29 and 30 minutes, each refusal, rungs 0, 1 and 2 and a stored rung above the day's count |
| `goldens/chest_lock_ransom.json` | `pipeline_layers/loot.py:LootLayer._chest_lock_active` | adapter | 19 and 20 distinct cards between the lock and the day's start, and a review at the start itself |
| `goldens/rung_deescalation.json` | `TripwireLayer._deescalate_rung` | adapter | a clean run of 6 and 7 days, a breach, a disarmed day, an unsettled day and a pending sprint |
| `goldens/tripwire_canary.json` | `TripwireLayer._tripwire_canary` | adapter | silences of 72 hours and one second over, unverified and suspended rails |
| `goldens/confess.json` | `TripwireLayer.confess` | adapter | armed and disarmed, a skip, an earlier fine, and a second confession at one instant |
| `goldens/instant_ack.json` | `TripwireLayer._instant_loop_ack` | adapter | the draw injected; the first, third and fourth ack, quiet hours and no transport |
| `goldens/free_spin_confirm.json` | `TripwireLayer._confirm_free_spin` | adapter | a review at 44 and 45 minutes, none, and a sync at 60 minutes and one second over |
| `goldens/tripwire.constants.json` | `tripwire.py`, `constants.py`, `gamification/quests.py` | constants | `TOKEN_LEN`, `MAX_SKEW_S`, `RATE_PER_APP_S`, `DAILY_EVENT_CAP`, `CANARY_SILENT_HOURS`, `PAIR_WINDOW_S`, `MAX_SESSION_S`, `GRACE_SECONDS`, `LONG_SESSION_MIN`; `TRIPWIRE_RUNG0_FINE`, `TRIPWIRE_CONFESS_FINE`, `TRIPWIRE_SPRINT_REVIEWS`, `TRIPWIRE_SPRINT_WINDOW_MIN`, `TRIPWIRE_RANSOM_REVIEWS`, `TRIPWIRE_RUNG_MAX`, `TRIPWIRE_DEESC_CLEAN_DAYS`, `TRIPWIRE_LONG_SESSION_MULT`, `PASS_SURCHARGE_HOURS`, `INSTANT_ACK_DAILY_CAP`, `FREESPIN_XP_LO`, `FREESPIN_XP_HI`, `FREESPIN_CONFIRM_MIN`; `Q1_TARGET` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `tripwire_events` | `discipline` | `migrations/010401_discipline_tripwire.sql` (SPEC-104) | `tripwire_events`, one row per ping or confession, its day as an epoch day | exported and erased |
| `tripwire_state` | `discipline` | the same migration | `tripwire_state`, with the bound source, the scope, the snooze's day and the spin's day and instant, which the predecessor keeps as runtime settings | reset in place: unbound, unverified, rung 0 |
| `sprints` | `discipline` | the same migration | `sprints`, one per event | exported and erased |
| `chest_locks` | `discipline` | the same migration | the chest-lock rows of `buffs`, one per study day with its instant | exported and erased |

## 9. Mutation rows

The band is `S10400-S10499`, in `scripts/mutation-rows.d/S10400-S10499.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10401-THE-TOKEN-IS-8` | `crates/discipline/src/constants.rs` | the token's length | `rail_goldens::the_ping_grammar_and_token_match_the_parity_goldens` |
| `S10402-THE-SKEW-IS-15-HOURS` | `crates/discipline/src/ping.rs` | the skew's inclusive bound | `rail_goldens::the_ping_grammar_and_token_match_the_parity_goldens` |
| `S10403-AN-UNBOUND-RAIL-READS-NOTHING` | `crates/discipline/src/rail.rs` | only a valid test binds | `rail_posts::an_unbound_rail_reads_nothing_but_a_valid_test` |
| `S10404-A-SUSPENDED-RAIL-IS-FREE` | `crates/discipline/src/verdict.rs` | a suspended rail fines nothing | `rail_goldens::the_verdict_matches_the_parity_golden` |
| `S10405-IDLE-BELOW-15-REVIEWS` | `crates/discipline/src/verdict.rs` | the idle threshold's boundary | `rail_goldens::the_verdict_matches_the_parity_golden` |
| `S10406-THE-GRACE-IS-60-SECONDS` | `crates/discipline/src/constants.rs` | an accidental open is free | `rail_goldens::the_close_and_the_grace_match_the_parity_golden` |
| `S10407-THE-RUNG-CAPS-AT-2` | `crates/discipline/src/rung.rs` | the rung's ceiling | `rail_goldens::the_fine_rung_matches_the_parity_golden` |
| `S10408-THE-BASE-FINE-IS-20` | `crates/discipline/src/constants.rs` | the rung-0 fine | `rail_goldens::the_fine_rung_matches_the_parity_golden` |
| `S10409-SEVEN-CLEAN-DAYS-EASE-A-RUNG` | `crates/discipline/src/constants.rs` | the de-escalation's count | `rail_goldens::the_de_escalation_matches_the_parity_golden` |
| `S10410-THE-RANSOM-IS-20-CARDS` | `crates/discipline/src/constants.rs` | the ransom's threshold | `rail_goldens::the_ransom_matches_the_parity_golden` |
| `S10411-A-SILENT-RAIL-SUSPENDS` | `crates/discipline/src/rung.rs` | the canary's 72 hours | `rail_goldens::a_silent_rail_is_suspended_and_fines_nothing` |
| `S10412-A-SPIN-WAITS-FOR-A-REVIEW` | `crates/coordination/src/discipline/spin.rs` | no grant without confirmation | `discipline_spin::a_spin_is_granted_only_when_confirmed` |
| `S10413-A-FAILED-FINE-IS-REFUNDED` | `crates/discipline/src/revision.rs` | a fine whose evidence fails is reversed | `rail_revision::a_fine_is_refunded_when_its_evidence_fails` |
| `S10414-ONE-EVENT-PER-APP-INSTANT` | `migrations/010401_discipline_tripwire.sql` | the unique key on (app, instant), a key held in the migration (a script-mutation row with a cargo killer) | `rail_goldens::every_post_outcome_matches_the_parity_golden` |
| `S10415-A-CHANNEL-POST-IS-NEVER-A-COMMAND` | `crates/bot/src/gate.rs` | a channel post is admitted as a rail post only | `channel_posts::a_channel_post_reaches_only_the_rail` |
