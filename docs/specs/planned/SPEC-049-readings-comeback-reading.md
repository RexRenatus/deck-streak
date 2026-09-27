# SPEC-049: in a lapse, the last ready reading is offered once per lapse inside the comeback cap, and carried nights show

- **Wave:** W1. **Issue:** #35 (epic #2). **Context(s):** `deck-streak-readings` (the choice and its table); `deck-streak-coordination` (the lapse branch of the morning readings job, and the message); the Mini App (`web/app`).
- **Decided by:** ADR-011 (side by side: DeckStreak sends only kinds the predecessor does not),
  ADR-019 (one comeback reading per lapse, inside the three-message cap), and ADR-049 (which reading,
  chosen once, who sends it before the comeback protocol exists, and its default during side by side).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-049.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's re-offer was never built.** Its design offered the last ready reading once per
  lapse when the owner came back; there was never a reading to offer, and its per-topic rollover
  count, the one signal of a reading carried unread, had no consumer at all
  (`preread_tracking.py:rollover_signal`, the inert feature `inert-reading-rollover-signal`).
- **The owner's decision.** After two or more days without study the daily readings pause, and the
  owner gets one comeback reading per lapse, inside the three-message comeback cap.
- **Two words, held apart.** A pause is the readings' rule: two study days without study (SPEC-045).
  A lapse is the governor's episode of three or more zero-review study days, with a lapse id, which
  closes when a study day has three or more reviews (docs/CONTEXT-MAP.md "Overloaded words";
  docs/schematics/streaks-and-governor-state-machine.md). A lapse therefore always opens on a day the
  readings are already paused, and the comeback reading is the one thing the readings send in it.
- **A side-by-side trap.** The predecessor still sends its own comeback messages in a lapse (its
  feature `comeback-protocol`). A second stream from the new bot would push the owner past the
  three-message cap across the two bots.
- **Prerequisites.** SPEC-041 (the router's comeback budget), SPEC-046 and SPEC-047 (readings and
  their read state), SPEC-051 (the reader and history screens this delivery adds the carried-nights
  badge to), SPEC-052 (the morning use case this delivery gives its lapse branch; SPEC-053 schedules
  it afterwards), and the governor's lapse episode and its id
  (#83), which the wave plan places in W3: this SPEC cannot be
  delivered before that episode exists, or before a slice of it is brought forward.

## 2. Requirements

R1. While the governor's lapse is open, the lapse's comeback reading is chosen once: the ready
    reading with the latest generation instant, whatever its studied verdict. When no ready reading
    exists, the lapse has no comeback reading and nothing is sent. No reading is generated for a
    comeback.
R2. The choice is stored once per lapse id in `reading_comebacks` (lapse id, reading id,
    `created_at`; unique on the lapse id), and every comeback message of that lapse names that one
    reading.
R3. The lapse id is the governor's, read through the streaks context in coordination; the readings
    never mint, derive or store any other lapse id.
R4. While a lapse is open and its comeback reading is unread, the morning readings job raises a
    `comeback` occasion through the router: kind `comeback`, tier `T2`, budget key `comeback`, the
    lapse id and the reading id. The router's cap (3 a lapse) and gap (3 study days) decide, so
    comeback messages never go out on consecutive days and never more than 3 a lapse. Once the reading
    is read, no comeback offers it.
R5. The message is the nudge-duties comeback template's variant 1, 2 or 3, chosen by how many
    comebacks this lapse has sent, with exactly one button: a URL into the Mini App's reader whose
    `startapp` value is `r_<reading id>`. It carries no date, no count of missed days and no streak or
    loss wording, and link previews are off.
R6. Each rendered variant's envelope is `phx.duty.message.v1` (duty `comeback`, kind `comeback`, tier
    `T2`, `budget_key` `comeback`, an opaque `dedupe_key` per send, the lapse id and the reading id).
    Golden envelopes of the three variants are committed, and nudge-duties' blocking rows,
    notifications-policy's `message-metadata` and telegram-platform's payload rows are green on them.
R7. While the predecessor still sends its own comeback messages, DeckStreak's `comeback_enabled`
    setting defaults to `"0"`, so the router withholds the comeback message with `nudges_disabled`;
    the Mini App's Today offers the lapse's comeback reading whatever the setting. The cutover
    checklist switches the setting on.
R8. The pause ends with the first nightly generation after a study day; the lapse ends when the
    governor closes its id, and no comeback is raised for a closed lapse.
R9. A reading's carried nights equal its vault note's `rolls`. The reader view and each history entry
    carry `carried_nights`, and the Mini App's reader and history show "Carried N nights" when N is
    above zero, and nothing when it is zero.
R10. Reading the comeback reading is the ordinary read tap (SPEC-047), worth its 40 XP if not yet read.
R11. `reading_comebacks` is registered in the context map's ownership register, declared in
    `privacy.json`, and exported and erased.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every morning of one lapse offers the same chosen reading, and a new lapse id chooses again | `one_comeback_reading_is_chosen_per_lapse_id` |
| A2 | comeback messages are never sent on consecutive study days (injected clock over a lapse) | `comeback_messages_never_go_out_on_consecutive_days` |
| A3 | no comeback is raised for a reading the owner has read | `no_comeback_offers_a_reading_already_read` |
| A4 | the fourth comeback raised in one lapse is withheld with `budget_spent` | `a_comeback_counts_against_the_three_message_cap` |
| A5 | the three golden comeback envelopes pass nudge-duties' blocking rows, notifications-policy's `message-metadata` and telegram-platform's payload rows, with non-zero examined counts | nudge-duties, notifications-policy and telegram-platform message rows; `test_the_comeback_envelopes_are_green_under_every_message_row` |
| A6 | choosing and offering a comeback reading calls the agent's runner zero times | `no_reading_is_generated_for_a_comeback` |
| A7 | with no open lapse, or no ready reading, nothing is raised and nothing is stored | `no_comeback_is_raised_without_an_open_lapse_or_a_ready_reading` |
| A8 | with the default side-by-side setting, the comeback message is withheld with `nudges_disabled` while Today still offers the reading | `the_comeback_message_is_off_during_side_by_side` |
| A9 | the reader view and the history entries carry `carried_nights` equal to the vault note's `rolls` | `the_reading_views_carry_carried_nights_equal_to_the_vault_rolls` |
| A10 | the reader and the history show "Carried N nights" when N is above zero and nothing at zero | `shows carried nights in the reader and the history` |

```acceptance
A1: cargo test -p deck-streak-coordination --test readings_comeback -- --exact one_comeback_reading_is_chosen_per_lapse_id
A2: cargo test -p deck-streak-coordination --test readings_comeback -- --exact comeback_messages_never_go_out_on_consecutive_days
A3: cargo test -p deck-streak-coordination --test readings_comeback -- --exact no_comeback_offers_a_reading_already_read
A4: cargo test -p deck-streak-coordination --test readings_comeback -- --exact a_comeback_counts_against_the_three_message_cap
A5: python3 -m unittest discover -s scripts/tests -p test_comeback_messages.py -k test_the_comeback_envelopes_are_green_under_every_message_row
A6: cargo test -p deck-streak-coordination --test readings_comeback -- --exact no_reading_is_generated_for_a_comeback
A7: cargo test -p deck-streak-coordination --test readings_comeback -- --exact no_comeback_is_raised_without_an_open_lapse_or_a_ready_reading
A8: cargo test -p deck-streak-coordination --test readings_comeback -- --exact the_comeback_message_is_off_during_side_by_side
A9: cargo test -p deck-streak-coordination --test readings_views -- --exact the_reading_views_carry_carried_nights_equal_to_the_vault_rolls
A10: pnpm exec vitest run web/app/src/lib/readings/carried-nights.test.ts -t "shows carried nights in the reader and the history"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/src/comeback.rs` | `deck-streak-readings` | added: the choice rule |
| `crates/readings/src/store.rs` | `deck-streak-readings` | changed: `reading_comebacks` |
| `crates/readings/src/rights.rs` | `deck-streak-readings` | changed |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed |
| `crates/readings/migrations/0004_comebacks.sql` | `deck-streak-readings` | added |
| `crates/coordination/src/readings/comeback.rs` | `deck-streak-coordination` | added: the lapse branch and the message |
| `crates/coordination/src/readings/morning.rs` | `deck-streak-coordination` | changed: in a lapse, the morning job takes the comeback branch |
| `crates/coordination/src/readings/views.rs` | `deck-streak-coordination` | changed: `carried_nights` on the reader and history views |
| `crates/coordination/tests/readings_comeback.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/readings_views.rs` | `deck-streak-coordination` | changed |
| `crates/coordination/tests/messages/comeback-variant-1.msg.json` | `deck-streak-coordination` | added: a golden envelope |
| `crates/coordination/tests/messages/comeback-variant-2.msg.json` | `deck-streak-coordination` | added |
| `crates/coordination/tests/messages/comeback-variant-3.msg.json` | `deck-streak-coordination` | added |
| `crates/notifications/migrations/0002_comeback_side_by_side_default.sql` | `deck-streak-notifications` | added: `comeback_enabled` seeded `"0"` |
| `web/app/src/lib/readings/CarriedNights.svelte` | miniapp | added |
| `web/app/src/lib/readings/carried-nights.test.ts` | miniapp | added |
| `web/app/src/routes/readings/[id]/+page.svelte` | miniapp | changed: the badge |
| `web/app/src/routes/readings/history/+page.svelte` | miniapp | changed: the badge |
| `web/app/src/lib/readings/TodayReadings.svelte` | miniapp | changed: the lapse's comeback reading on Today |
| `scripts/tests/test_comeback_messages.py` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register gains `reading_comebacks` |
| `privacy.json` | repo | changed |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-049-readings-comeback-reading.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-049-the-comeback-reading-chosen-once-per-lapse.md` | docs | added |
| `docs/red-first/SPEC-049.md` | docs | added |

## 5. What this does NOT do

- It decides no lapse and mints no lapse id; the governor does (#83).
- It sends none of the comeback protocol's own messages (the acknowledgement, the landmark
  intention, the last notice) and no lapse digest (#123).
- It celebrates no return: the relight is the streaks' (#84).
- It changes no morning brief (#122).
- It switches the comeback message on for no one: the cutover checklist does (#62).

## 6. Risks

- **The governor's lapse episode does not exist in W1.** This SPEC names it as a prerequisite; its
  lapse branch cannot compile against a missing streaks query, so the plan must bring a slice of the
  governor forward or deliver this SPEC after W3's governor.
- **Two bots' comeback messages exceed the cap** during side by side. Prevented by R7 and tested by A8.
- **The chosen reading is weeks old by the time the owner returns.** It is still the last reading
  the owner had; its carried nights say so honestly, and the next nightly generation after a study
  day brings fresh readings.
- **A comeback text drifts into shame or loss wording.** Refused by nudge-duties' `no-shame-framing`
  on the golden envelopes (A5); the text is the pack's template, not generated.
