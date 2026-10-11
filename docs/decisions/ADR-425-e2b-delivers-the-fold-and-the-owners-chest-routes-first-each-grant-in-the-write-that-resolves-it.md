---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# E2b delivers the fold and the owner's chest routes first, each grant in the write that resolves it

## Context and Problem Statement

SPEC-081 (#102, #103) lands in parts. Its section 3c names E2b as the delivery that wires the quests
core into the fold, the use cases, the API, the bot, the Mini App and the odds page; E2c is #104, and
E3 (#107) is the shop's arms. At dev `ef4d9077` the core is in the tree and nothing calls it. The six
quests ports (`grant_session_chests_on`, `open_chest_on`, `sweep_stale_chests_on`,
`activate_token_on`, `settle_token_bonuses_on`, `settle_epic_choice_on`) are named in six files of
`crates/quests`, one decision record and three formal entries, and in no file of coordination, the
API, the bot, the daemon or the web app. SPEC-411 section 1 holds the measurement, file by file.

Three of SPEC-081's amendments are still undelivered at `ef4d9077`:

- T1: the settle registry admits no `2x:` source (`crates/progression/src/settle.rs:32`).
- T3: no store function sets a chest's announced mark (`crates/quests/src/chest_store.rs:178`).
- T4: there is no `grant_freeze_on`; `grant_freeze` opens its own write
  (`crates/coordination/src/freeze.rs:27`).

The question is how much of E2b one pull request delivers, in what order, under which record, and
how each grant stays paid once when the owner's taps and the fold's recompute write the same rows.

## Decision Drivers

- A grant is paid once whoever writes first: a replay, a double tap, or a recompute between two
  taps grants nothing more.
- The API depends on kernel, identity, notifications and coordination only, never on quests, so
  its answers are coordination's own types. Coordination already depends on quests, streaks,
  progression, notifications, analytics and economy, so E2b needs no new dependency edge.
- SPEC-081's Status changes only when E2 is whole.
- The odds have one source.
- Every criterion is red first for its own reason, measured locally before the one push.
- Four open pull requests (#778, #782, #791, #809) hold the seven locale files; #782 also holds the
  Mini App's API client and its route list.

## Considered Options (the alternatives it was chosen against)

### D1, the measurement

- Chosen: the baseline is SPEC-411 section 1's table, each of SPEC-081's criteria MET, PARTIAL or NOT MET at `ef4d9077` with the file:line of the test that decides it, because a criterion with no test is NOT MET and a builder re-measures a line, never a verdict.
- Chosen: the reading is 14 MET (A1 to A12, A15, A21), 0 PARTIAL, and 9 NOT MET (A13, A14, A16 to A20, A22, A23), with T1, T3 and T4 recorded undelivered beside it, because each of the three is a precondition of a wiring step.
- Chosen against: a verdict list without file:line: lost, because a builder cannot re-measure a verdict that names no line.

### D2, the scope

- Chosen: E2b-1, the delivery this record decides, is the server half in dependency order (progression admits `2x:`; notifications gains a public quiet-window function; quests gains a holdings read; coordination gains `grant_freeze_on`, three fold steps and four use cases; the daemon registers the steps; the API gains four owner routes), because each step calls only what an earlier step delivered.
- Chosen: E2b-2 is the web half (the Mini App's chest inventory, the open animation SPEC-081 A17 judges, the odds page A16 judges, the route list, the start parameter, the seven message catalogs and `economy.json`'s disclosure), citing #102 and #103, because it reads the routes E2b-1 installs.
- Chosen: E2b-3 is the bot half (the chest callbacks A20 judges, the announcement with its Open and Open-in-app buttons and the keyboard they need, the announced mark and the offers), citing #102, because its Open button calls E2b-1's open use case.
- Chosen: #104 (E2c) and #107 (E3) are never folded in, because SPEC-081 section 3c gives each its own pull request.
- Chosen: an open through the API raises no message and the API gains no router; the Mini App's reveal is its animation of the route's answer, because ADR-041 keeps one router and an animation is not a message.
- Chosen against: one build of all six surfaces: lost, because four open pull requests touch all seven locale files and the bot needs a keyboard that `crates/notifications/src/transport.rs:86`, which sends text only, does not carry.
- Chosen against: the web first: lost, because the web would read routes that do not exist.
- Chosen against: the bot first: lost, because its Open button needs the open use case.

### D3, the record

- Chosen: E2b-1 is SPEC-411, which cites SPEC-081 and edits no line of it, because SPEC-081's Status changes only when E2 is whole.
- Chosen: SPEC-411 reuses SPEC-081's exact test names for SPEC-081 A18 and A19 (SPEC-411 A2 and A13), because the delivery that makes E2 whole then moves A16 to A20 back into SPEC-081's section 3 verbatim, as its section 3c asks, and changes its Status once.
- Chosen against: amending SPEC-081 now: lost, because a partial delivery would edit its Status and section 3c before E2 is whole, and add one more amendment section for half of E2b.

### D4, integrity and containment

- Chosen: all four routes take the owner session first, so a request without one answers 401 before anything is read, because SPEC-081 R18 says each answers the owner's session only.
- Chosen: the three POSTs also take the state-change guard, so a cross-site POST answers 403 `cross_site_request` and writes nothing, because every POST here writes.
- Chosen: a chest id is a positive decimal `i64`, digits only, else 400, parsed as `crates/api/src/sensitive_decks_routes.rs:95-100` parses one, because a parse the house already pins needs no second form.
- Chosen: every route answers 503 until the database opens, as the sensitive-decks routes do, because a route with no database has nothing true to say.
- Chosen: each grant is made idempotent by its guarded transition and `xp_ledger`'s once scope in the SAME write, because a replay then finds the transition already taken and writes nothing.
- Chosen: an open pays by `mark_opened` or `mark_resolved` with its `chest:<id>` grant once; the sweep by `sweep_resolve` with its grant once; an Epic's pick by the `settle_choice` guard with `xp_tokens_one_per_chest` and the freeze in that write; an activation by a guarded update; the token bonus by settle, replaced while its day is open and only raised once it is closed, because each transition is the store's own guard.
- Chosen: the inventory never sends a sealed or vaulted chest's rarity or payout, because the open is the reveal (SPEC-081 A8) and an inventory that sent the rarity would spoil it.
- Chosen: on the current study day only a session that ended at least `SESSION_GAP_MS` before now earns a chest, because an open session's partial base XP caps a Common or Rare payout (`EarnedSession`, `crates/quests/src/chests.rs:97-113`).
- Chosen: the odds have one source, `economy.json`'s `chests` (lines 84-117), pinned to the draw by `crates/quests/tests/chests_roll.rs:88` now and to the odds page by SPEC-081 A16 in E2b-2, because two readers of one table can disagree.
- Chosen: no bot command is added, because SPEC-081 R19 says the bot gains no command.
- Chosen against: an idempotency-key table: lost, because the guards and the once index already make a replay a no-op, and a key table is a second truth that needs a migration.
- Chosen against: an API odds route: lost, because it would be a second reader of the odds.
- Chosen against: granting a chest at a session's first eligibility on the current day: lost, because an open session's partial base XP caps the payout, so the payout would depend on when the fold ran.

### D5, the formal model

- Chosen: REQUIRED, a TLA+ model, because the surface interleaves: the owner's open, pick and activate writes run against the fold's chest, sweep and token-bonus writes over the chests, `xp_tokens`, `xp_ledger` and the freeze rows.
- Chosen: `tla/ChestOpenedOnce` is extended with a constant `CreditInCall` (TRUE in its MC configuration), a new variable for the credits (its `TypeOK` gains one conjunct; no base conjunct changes and no domain widens), a property `AResolvedChestIsPaid` entering at report, at least two actors, and covers for coordination's `open_chest`, `pick_epic_choice`, `grant_freeze_on` and the sweep step's `sweep_and_pay`, because the new callers act on that entry's variables.
- Chosen: the property's witness is `witness/a-resolved-chest-left-unpaid.cfg` with `CreditInCall = FALSE`, citing #102, and it models the rejected design (a credit in a later write), because no caller exists at the base for a witness to port.
- Chosen: `tla/TokenWindow` gains covers for `activate_token` and the token-bonus step's `settle_day_token_bonuses`, and `tla/ChestRolledOnce` gains a cover for the chest step's `grant_day_chests`, because each new caller is a new actor over that entry's state.
- Chosen: each step's work lives in a uniquely named free function, because a cover anchors one item and two `evaluate` impls in one file resolve to nothing.
- Chosen: floors are only raised, and an entry's budget is overridden only when the measured need times 1.5 exceeds the default, then pinned by a test and a row, because a budget is a measured need.
- Chosen: Lean is NOT APPLICABLE, because there is no new arithmetic (`TokenBonus.lean` and `Chest.lean` are unchanged) and no balance exists: holdings are a count of rows and every amount is typed.
- Chosen against: NOT APPLICABLE: lost, because a new caller is a new actor, and the model would assume, unchecked, that each caller credits in the same write.
- Chosen against: a new entry: lost, because an entry cannot extend another entry's module, and the callers act on `tla/ChestOpenedOnce`'s variables.
- Chosen against: covers in a follow-up: lost, because a REQUIRED model lands in the same pull request as the code it covers.

### D6, CI cost and reds

- Chosen: ONE push, because every red and green is measured locally by targeted cargo tests before it.
- Chosen: cargo-mutants runs over the diff locally before the push, with the flags CI's Rust mutation job passes, because a survivor CI finds costs a second push.
- Chosen: CI's jobs are read by name after the push and none is re-run by the builder, because a red is diagnosed from its own log.
- Chosen: the suites the repository restricts to CI never run locally, because their restriction is the repository's own rule.
- Chosen against: two pushes: lost, because every red is measurable locally and a second push re-runs every job.
- Chosen against: leaving survivors to CI's Rust mutation job: lost, because a survivor found there costs a second push.

## Decision Outcome

The chosen options are D1 to D6, as each "Chosen:" bullet states. E2b-1 makes the quests core
reachable: the fold grants, sweeps and settles chests and token bonuses in each study day's one
write, and the owner opens a chest, picks an Epic's reward and activates a token through four API
routes, each grant made in the write that resolves it. The web half (E2b-2) and the bot half (E2b-3)
follow as their own deliveries. SPEC-081 keeps its Status until E2 is whole.

### Consequences

- Good, because a replayed open, pick or activation finds its transition taken and grants nothing
  (SPEC-411 A8, A9, A12, A15), and the model checks the interleaving the tests cannot enumerate.
- Good, because the API's answers are coordination's own types, so no new dependency edge exists.
- Good, because the odds still have one source, and the page that shows them reads the same file.
- Bad, because the fold's write grows: it draws and grants inside the day's write, and a draw failure
  must keep the day (SPEC-411 R4).
- Bad, because until E2b-2 and E2b-3 land, an owner reaches a chest only through the API.
- Bad, because four open pull requests share the Mini App's files, so E2b-2 waits for them.

### Confirmation

SPEC-411 A1 to A20 by their fence lines; the red-first record `docs/red-first/SPEC-411.md`; rows
S41100 to S41121 in `scripts/mutation-rows.d/S41100-S41199.json`; the formal check of
`tla/ChestOpenedOnce`, `tla/TokenWindow` and `tla/ChestRolledOnce`, with the witness caught and
`AResolvedChestIsPaid` clean in its MC configuration.

### What would make this wrong

- A replay that writes a second `xp_ledger` row for one chest, one pick or one window.
- A recompute that rolls a session on a backfilled or revisited day, or grants on a past day.
- A model run in which `AResolvedChestIsPaid` is clean with `CreditInCall = FALSE`, which would make
  the property vacuous.
- An inventory answer that carries a sealed or vaulted chest's rarity.

## More Information

#102, #103, #104, #107; SPEC-081 sections 3c and 5, R18, R19 and its amendments T1, T3 and T4;
ADR-041 (the one router); ADR-071 (each study day is settled once, in order); ADR-072 (the token
bonus is settled XP); ADR-081 (one draw per chest, stored with its pity in one write); SPEC-411.
