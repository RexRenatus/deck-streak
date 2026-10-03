---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, ruling on the design pass for #571"
---

# The recompute holds each celebration for the bot's senders through a router with no transport

## Context and Problem Statement

No production recompute cycle holds a notification router (#571), so every celebration the awards
phase offers, the level-up line and the due relights are never raised. Both production cycles are
built by one seam, `RecomputeSetup::cycle`, in the job process, and the job process holds no bot
credential (ADR-066). A router with no bot transport withholds a bot occasion `no_notifier`, and
the award's port marks an award on any answer (ADR-303), so attaching such a router as it is would
lose every award by name. Which router does the recompute hold, and who sends what it raises?

## Decision Drivers

- The job keeps holding no bot credential (ADR-066), and no deploy file or owner act is needed.
- An award offered at a daytime owner's sync must reach the owner, not be marked and lost.
- The cycle's type is covered by three formal entries (AwardOnce, FoldSettlesOnce, RelightOrder);
  `sync_cycle.rs` is not edited.
- #402 items 8 and 11, whose recommended answer binds until the owner answers: a delivery landing
  before the cutover checklist seeds its own switch off, with an insert that ignores an existing row.

## Considered Options (the alternatives it was chosen against)

- A holding router, `Router::new(..).holding()`, attached by `RecomputeSetup::cycle` through `CycleParts::with_flush` (chosen): a bot celebration it cannot send is deferred `send` and held on the queue, as the breaker-open arm holds one; the bot's flush after the owner's answer and the `held_flush` job send it.
- The sync job holds the bot's credentials and sends: lost, because ADR-066 rejected the job joining the bot's router (it widens the job unit's credentials), and it needs a deploy change and an owner act.
- A plain router with no transport: lost, because every award an owner's daytime `/sync` evaluates is withheld `no_notifier` and marked, so it is lost by name.
- A new `CycleParts::with_router`: lost, because it is a second name for the one `router` field that `with_flush` already sets, and it would edit `sync_cycle.rs`, which three formal entries cover.
- The recompute in the bot process: lost, because SPEC-059 moved the owner's cycle into the job, and the bot does not read the collection.
- An outbox table of celebrations for the bot to raise: lost, because ADR-303 already rejected an outbox; the queue already holds a deferred celebration with its claim.
- The senders offer the owed awards themselves: lost, because the level-up is decided in memory during the cycle (`sync_cycle.rs:321-327`), and #127 part b needs the collection, which only the job reads.
- A new hold value `sender`: lost, because the queue's `CHECK (hold IN ('quiet','send'))` needs a migration that rebuilds the table; the recap's wording is recorded as a known deviation instead (SPEC-319 section 5).
- The switch seeded by a migration: lost, because a migration seeds every database the kernel opens, and 20 files in five crates build a notification router and name a celebration, so each celebration they expect sent would turn withheld; the seed runs where the recompute starts instead.
- No seed: lost, because #402 item 11's hold is in force until the owner answers, and this delivery is the first to let a celebration leave the box.

## Decision Outcome

Chosen option: "a holding router attached by `RecomputeSetup::cycle`", because it raises every owed
celebration without giving the job a credential, sends through the two processes that already hold
the bot's credentials, and changes no covered item of the cycle.

- `Router::holding()` sets a flag that `Router::new` leaves off. In `after_claim`, a bot
  celebration with no transport on a holding router is `Defer(Hold::Send)`; any other bot occasion
  with no transport is still withheld `no_notifier` and its key released. A router without the flag
  is unchanged.
- `RecomputeSetup::load` compiles the policy once and seeds `celebrations_enabled` to `"0"` with
  `INSERT ... ON CONFLICT (key) DO NOTHING` on `notification_settings`, through notifications'
  `seed_celebrations_off`. `RecomputeSetup::cycle` attaches
  `Router::new(policy, db, SystemClock, rule).holding()` through `with_flush`.
- The cycle's own flush after its sync answers `Flushed::NoNotifier`. The senders are the bot's
  flush after it observes the owner's request answered by a sync that ran and succeeded
  (`sync_request.rs::answer`), and the `held_flush` job at 07:36.

### Consequences

- Good, because every owed award, the level-up and the due relights reach the router, and each
  one the router answers is marked once (ADR-303).
- Good, because the job's units, credentials and deploy files are unchanged.
- Good, because the router's once-ever key still keeps each celebration to one send when two
  cycles overlap (AwardOnce), and the flush lease and row claims keep each held row to one push
  (HeldFlush).
- Bad, because a celebration held at a daytime owner's sync whose answer the bot does not observe
  in its bound waits for the next 07:36 flush, which abandons it by name when it is past 720
  minutes.
- Bad, because a held row is stored `send`, so a recap reads "(send failures)" though no send
  failed (SPEC-319 section 5).
- Neutral, because until the cutover checklist turns the switch on, every celebration is withheld
  `nudges_disabled` and marked, which is #402's hold.

### Confirmation

`crates/daemon/tests/recompute_router.rs` (A1 to A4) drives the production seam;
`crates/notifications/tests/router.rs` (A5) holds the arm; rows S31900 to S31905;
`formal/tla/HeldFlush` with `MCHeldSendHold.cfg` and the witness
`a-hold-outside-the-window-with-no-later-flush.cfg`.

## More Information

SPEC-319; SPEC-041 section 10; ADR-066 and ADR-109 (amended); ADR-303; issues #571, #402, #572,
#127.
