---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, ruling on the design pass for #572"
---

# Coordination reads the stored streak through the router's database, and a failed read fails closed

## Context and Problem Statement

SPEC-084 R5 caps every celebration of a study day on which the language streak broke, and R11
re-caps each held celebration at the flush's study day. No production path hands the ladder a
streak fact (#572): `ladder_facts::streak_facts()` is `None`, `Occasion::with_streak` has no
production caller, and every production flush passes no facts. Since SPEC-319 every production
celebration is held at route time, so the senders' flush is where the cap must bite. The streak is
stored: the streaks context writes `streak_state`, and coordination already reads it
(`progression/badges_view.rs`).

## Decision Drivers

- No new dependency edge: coordination already depends on streaks and notifications, and the
  daemon on coordination (`docs/CONTEXT-MAP.md`).
- The fewest covered spans move. The routes and flushes are covered by HeldFlush, AwardOnce,
  RelightOrder, BandUpOnce and LandmarkOnce.
- The cap must apply on exactly the day it is for, including at the flush that sends what the
  recompute held.
- `crates/daemon/src/role_job.rs` and `crates/daemon/src/role_bot.rs` are not edited.

## Considered Options (the alternatives it was chosen against)

- D1, `ladder_facts` reads the stored `language` row through a new `Router::db` accessor (chosen): chosen because it is one reader in the context that already depends on both, with no new edge and no covered router item moved.
- D1 (a), a facts port on `Router`: rejected because notifications would read another context inside its own write, and it changes `route` and `flush_with`, which four formal entries cover.
- D1 (b), a `streak` field on `Celebration` filled by each offer: rejected because it edits the covered `offer_badges`, `offer_records`, `offer_band_ups` and `landmarks::offer`, and still leaves the level-up, the relight and the flushers without facts.
- D1 (c), a `Db` handed beside the router (`AwardOffers::new`, `HeldFlushWork::new`): rejected because it edits the covered `cycle_offers` and the daemon's `role_job.rs`.
- D1 (d), facts per occasion day replayed from the log in coordination: rejected because it is a streaks rule written outside the streaks context.
- D1 (e), a widened `Offers::offer`: rejected because the trait's only caller is the covered `offer_owed`, and the level-up, relight and flushers are still left out.
- D2, one inserted statement before each of the three routes, never a wrapper (chosen): chosen because the `Celebrate` impl is the one door every award, band-up and landmark uses, and a wrapper would re-indent covered spans.
- D3, a failed read fails closed with the existing reason `flush_failed` (chosen): chosen because nothing renders past the cap and the covered `perform` keeps its four arms byte-equal.
- D3, fail open (route or flush with no facts on a read error): rejected because it renders past the cap on exactly the day the cap is for.
- D3, a new reason code such as `streak_unread`: rejected because it adds an arm to the covered `perform`.
- D3, a facts parameter on `Work` or `Flush`: rejected because it edits `role_job.rs` and `role_bot.rs`.
- D4, re-read and re-stamp the three moved entries with the read as a named stutter (chosen): chosen because the read moves none of their variables and its error is an arm each model has or names.
- D4, extend HeldFlush with tiers and a `broke` variable: rejected because the entry deliberately models no tier, and with no lease over `streak_state` the property is violated at the code's own configuration, which is a finding (#589), not a closure.
- D4, formal NOT APPLICABLE: rejected because four covered spans move and the read is a check followed by an act on a row another actor writes.
- R8, exempt the relight from R5: rejected because SPEC-084 R5 caps every celebration, and an exemption would need its own amendment of the predicate.

## Decision Outcome

Chosen options: D1, D2, D3 (fail closed) and D4 (re-stamp), because together they put the one
reader where both contexts are already reachable, move only the spans the three routes and two
flushers already own, and never render past the cap.

- `Router::db(&self) -> &Db` answers the router's own database (notifications).
- `ladder_facts::streak_facts(db)` reads `deck_streak_streaks::store::state` for `language` on a
  reader connection and maps `last_study_day`, `current` and `longest`; no stored row is `None`.
  `with_streak_facts(db, occasion)` carries them; `flush_re_capped(router)` reads, then
  `router.flush_with(facts)`.
- The `Celebrate` impl, `announce_level_up` and `announce_relight` each gain
  `let occasion = ladder_facts::with_streak_facts(<the router's db>, occasion).await?;` before the
  route. `HeldFlushWork::perform`, `sync_cycle.rs::flush` and the bot's `Flush` for `Arc<Router>`
  call `flush_re_capped`. `Router::flush` stays `flush_with(None)`.
- D5, the doc lines: each "until SPEC-076" line says what is now true, in the texts SPEC-326 R9
  names.

### Consequences

- Good, because R5 and R11 apply in production: a break day's celebrations are capped at the
  route and re-capped at every flush, at the flush's own study day.
- Good, because no dependency edge, migration, policy key or covered router item changes.
- Bad, because a fold that commits a break between a flusher's read and its render renders one held
  celebration uncapped once; #589 records it.
- Bad, because a failed read loses a level-up, which has no mark, as any router error does today.
- Neutral, because the relight's own line is always capped at T1 (R8), disclosed in SPEC-326.

### Confirmation

`crates/coordination/tests/break_day_cap.rs` (A1 to A4, A7), `crates/daemon/tests/break_day_flush.rs`
(A5) and `crates/coordination/tests/level_up_cycle.rs` (A6); rows S32600 to S32613; the re-read
notes of `formal/tla/HeldFlush`, `formal/tla/AwardOnce` and `formal/tla/RelightOrder`.

## More Information

SPEC-326; SPEC-084 section 11; SPEC-319 and ADR-319; ADR-303; issues #572, #589, #590.
