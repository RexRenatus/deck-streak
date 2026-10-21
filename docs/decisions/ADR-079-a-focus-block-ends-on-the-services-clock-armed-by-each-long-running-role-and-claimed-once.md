---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A focus block ends on the service's clock: each long-running role arms it, one claim by its generation credits it, and the router announces it

## Context and Problem Statement

The predecessor's focus timer lives in one process, its bot. The bot edits a countdown message every
few seconds, persists the end instant so a restart can resume or catch the block up
(`bot.py:CommandBot._rearm_focus_timer`), and guards a double credit with one flag on the timer's
row: the first caller to set it wins (`database.py:GamifyStore.mark_focus_timer_fired`). It commits
that claim before it records the block, and heals a crash between the two by clearing the row and
telling the owner the cycle was interrupted.

DeckStreak has two surfaces and two long-running roles, `api` and `bot` (SPEC-025, SPEC-026), plus
one-shot jobs (ADR-027). A block can start in the Mini App or in the bot, and it must end on time
whichever role is up. Its end is a message the owner did not ask for at that moment, so it must go
through the one router (CHARTER 2, ADR-041), and the notifications policy admits four classes of
message, of which only alerts skip quiet hours. Where does a block's end fire, how is it credited
exactly once, and what is the message that announces it?

## Decision Drivers

- A block started on either surface ends at its end instant, and a restart neither loses nor
  repeats it.
- Two roles may hold a timer for the same block; the credit must be exactly once, and a timer armed
  for a replaced block must never credit the new one.
- No new process, privilege or host unit for a timer (ADR-032).
- Every message the owner did not just ask for passes the one router, and quiet hours bind every
  class but alerts (the notifications-policy pack).
- The predecessor's rules stay proved by their goldens (ADR-012): the clamps, the cycle, the stop's
  minutes and the restart's decision.

## Considered Options (the alternatives it was chosen against)

- Persist the end instant, let each long-running role arm a timer from it, and credit the block through one conditional update keyed by its generation — chosen: a block started on either surface ends on time without a new process, a restart catches it up, and two roles racing credit it once.
- Keep the predecessor's claim, the row's fired flag alone — rejected because two roles hold timers here, and a timer armed for a replaced block would claim the new block's end.
- Commit the claim before the block's row, in the predecessor's order — rejected because a crash between the two writes leaves a claimed block uncredited, which then needs a repair and a notice.
- A job that checks the timer every minute — rejected because it starts a process each minute on a small host and ends a block up to a minute late.
- Only the bot role arms timers — rejected because a block started in the Mini App would not end until the bot role next read the row.
- A transient systemd timer for each block — rejected because creating units at run time needs privileges the service's hardened units do not hold (ADR-032).
- Announce the end as an alert — rejected because alerts are incidents, and the policy lets only alerts skip quiet hours, so the pack would refuse the exemption.
- Announce the end as a nudge — rejected because quiet hours suppress a nudge outright, so a block that ends at night would never be announced.
- Announce the end as the ladder event `focus_block` at T2 — chosen: it celebrates the owner's own block, is deduplicated per block, and is deferred, not lost, in quiet hours.
- Edit a bot countdown every few seconds for every block, as the predecessor does — rejected because the Mini App counts down on the client from the server's end, so the bot keeps a countdown only for a block it started.

## Decision Outcome

Chosen option: "persist the end instant, arm a timer in each long-running role, claim once by the
block's generation, and announce the end as the ladder event `focus_block`", because it ends every
block on time from either surface with no new process, and makes a double credit impossible rather
than repaired.

- **The row.** `focus_timer` holds one row. Every start (a one-off block, a cycle's first block, a
  cycle's next block) increments its `block` generation and writes the end instant while running.
- **The timers.** The `api` and `bot` roles each arm an in-process timer
  (`tokio::time::sleep_until`) for the running block when the role starts and whenever it writes the
  row, and stop it at shutdown. At start, a role decides from the row as the predecessor's restart
  does: idle and paused rows arm nothing, a running row past its end fires once, a running row
  before its end is armed for the rest.
- **The claim.** The timer's claim is one conditional update that succeeds only for the block's own
  generation, while the row is running, unclaimed and past its end; a stop or a replacement claims
  by the generation alone. The claim, the block's `focus_log` row and the timer's next state (idle,
  or a cycle's next block with a new generation) commit in one write through the kernel's repository
  base. So a stale timer, a paused block and an end that an added five minutes moved all claim
  nothing, and no claimed block is left uncredited: the predecessor's interrupted-cycle notice has
  no case to report.
- **The message.** A block that ends on its timer (a one-off end, a cycle's advance, a restart's
  catch-up) is raised through the router as the celebration event `focus_block` at T2, with a dedupe
  key that names the generation. Quiet hours defer it like every celebration. A stop is answered
  where it was made, by the bot's countdown message or by the Mini App's response.
- **The policy.** `notifications-policy.json` gains `"focus_block": "T2"` under `ladder.events`, and
  its `deviations` record the key `ladder.events.focus_block` with this ADR, as the
  notifications-policy pack requires of a value that differs from its baseline.

### Consequences

- Good, because a block ends on time whichever surface started it and whichever role is up, with no
  new process and no privilege.
- Good, because the credit is exactly once by construction, and the predecessor's crash repair and
  its interrupted-cycle notice are no longer needed.
- Good, because the block's end is an ordinary celebration: deduplicated, recorded in the decision
  ledger, and deferred in quiet hours.
- Bad, because a block that ends in quiet hours is announced after them, where the predecessor sent
  it at once; the Mini App shows the end on the client meanwhile.
- Bad, because each long-running role holds a timer task, and a block armed by one role alone ends
  only when that role runs: if it is down at the end, the block ends at its next start, as a
  catch-up.

### Confirmation

SPEC-079's acceptance tests: a second claim wins nothing, a stale timer claims nothing, a timer
claims nothing before its end, two roles credit one block once after a restart, and one
`focus_block` celebration per block; the goldens of `bot.py:CommandBot._rearm_focus_timer`,
`bot.py:CommandBot._cycle_advance`, `bot.py:CommandBot._focus_stop_locked` and
`bot.py:CommandBot._focus_add_locked`; the hand-proved rows on the claim's SQL (band
`S07900-S07999`); and the notifications-policy pack's deviation check over
`notifications-policy.json`.

## What would make this wrong

- DeckStreak gains a durable task scheduler that holds one-off deadlines: the timer then moves
  there, and the roles stop arming their own.
- The owner asks for a block's end inside quiet hours: that is a change to the policy, which the
  notifications-policy pack would have to admit as a new exemption.
- Telegram's limits make the bot's countdown edits fail often: the bot then shows only the end
  instant.

## More Information

SPEC-079; ADR-041 (the router and its origin rule); ADR-071 and ADR-072 (the settle of focus XP);
ADR-032 (the hardened units); the predecessor's `bot.py:CommandBot._rearm_focus_timer`,
`bot.py:CommandBot._focus_complete_locked` and `database.py:GamifyStore.mark_focus_timer_fired` at
`27ee2bc`; #98, #99.
