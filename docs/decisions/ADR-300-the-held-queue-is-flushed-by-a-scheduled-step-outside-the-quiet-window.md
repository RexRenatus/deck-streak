---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The held queue is flushed by a scheduled step outside the quiet window

## Context and Problem Statement

SPEC-041 R7 flushes the held queue after every successful sync. The scheduled sync runs inside the
default quiet window (23:00 to 07:30), so on that path the flush finds the window closed and delivers
nothing: a celebration held overnight waits for some sync that lands after 07:30. Issue #291 asks for
the held queue to be flushed when the window ends. Two further facts shaped the answer. Only the
bot's process held a router, so the scheduled job's process had none to flush with. And the
flush's first transaction was committed before its sends, so two flushers over one queue could each
read the same held row and send it twice, and a lease that lapses while its holder is still sending
does not stop that. What triggers a flush outside the window, and how does one held item reach the
owner at most once when flushers overlap, lapse or die?

## Decision Drivers

- A held notification reaches the owner at most once and is never lost unnamed: it ends delivered, or
  abandoned by name with its reason (expired, send failed, or may have been sent), on every path that
  can flush; a flush delivers only while the window is open when it starts.
- The step's calendar is derived from the window the router reads, never a second copy of it.
- A missed fire is replayed late, and a hold that the replay finds past its age limit (720 minutes) is
  abandoned by name, never dropped silently.
- A claim on the row needs a state and a token on the queue, so the schema changes: a `state` change
  to the queue is a table rebuild, and the rebuild keeps every index and trigger the old table had.
- The two flushers are an interleaving, so a model states the properties before the code (the
  entry `formal/tla/HeldFlush/`).

## Considered Options (the alternatives it was chosen against)

The options were measured against one generated population of 96 flush cases (3 flushers x 8 clock
positions x 4 hold states) and against the model, whose witness configurations switch each rejected
shape on.

- A scheduled job of its own, `held_flush` at 07:36 UTC with catch-up — chosen because over the 96
  cases it delivers or abandons each hold exactly as expected, and the model holds both properties
  clean at 558 distinct states with its lease on.
- Flushing inside the sync job only — rejected because it is the shape that failed: the model's
  witness with no scheduled flusher violates `HeldReachesOrAbandons`, and at dev 12 of the 96 cases
  (the scheduled-step arm with the window open) delivered nothing. A sync that is refused, skipped
  or late never flushes.
- A readings job that does not exist yet — rejected because the readings jobs (#39) are not built,
  so the flush would wait on them, and a readings step runs on a readings job's failure and retry
  rules, which a flush must not inherit.
- A delay loop in the daemon that sleeps until the window ends — rejected because it adds a second
  clock beside the timer, is lost at every restart where a `Persistent=` timer replays the miss, and
  lives in the long-running process rather than the job a ledger records.

## Decision Outcome

Chosen option: "A scheduled job of its own", `held_flush`, a daily job at 07:36 UTC with catch-up,
with its own timer, drop-in and job-table entry, because it is the only shape that flushes on a
path with no sync, and its calendar is checked against the policy's window by a test over the job
table and the deploy templates.

- **The calendar.** 07:36 is six minutes after the default window ends. With `Persistent=true` a
  missed fire is replayed up to 360 minutes late (13:36), still outside the window, and the oldest
  hold raised at the window's start (23:00) is 516 minutes old at the first fire and 876 at the latest
  replay, against the 720 limit. A fire replayed after 11:00 therefore abandons that hold by name in
  the recap instead of delivering it: a missed fire can cost a hold its delivery and never its name.
  The catch-up bound is not shortened: a replay inside it still delivers every hold younger than the
  limit, and one past it is named rather than sent late.
- **The catch-up decision.** `catch_up: true`, because a flush is idempotent: a replayed fire finds
  an empty queue or delivers what the first fire would have, and the lease keeps two fires from
  overlapping.
- **The claim: a row is claimed, sent and settled at most once.** In its first transaction a flush
  moves each held row it will send from `held` to `sending`, writing its lease token (the instant its
  lease lapses) in the row's new `claim` column, by one update that matches only `held` rows, so a row
  another flush claimed is never read for sending. A push is made only for a row the flush claimed.
  The settle that removes the row, the relatch after a failed send and the abandonment each match that
  token, so a flush that lost its claim changes nothing. A flush that finds a `sending` row whose claim
  has lapsed abandons it by name, "may have been sent", and never pushes it: its claimant either died
  after the push reached the owner or is still sending past its lease, and in neither case is a resend
  safe. An abandonment is logged with the item, its claimant and the reason, and the recap names it.
  The schema change is migration 004102, a STRICT rebuild of `notification_queue` that adds the state
  `sending` and the `claim` column and re-creates every index and trigger the old table had (it had
  none; a test compares the sets before and after), and the data-rights export and erasure carry the
  column.
- **The lease stays as the queue's coarse exclusion.** A flush takes a lease before its first send,
  held as the setting `flush_lease` whose value is the instant it lapses (ten minutes on), and
  releases it, by its token, when it ends. A flush that finds an unlapsed lease answers `Busy` and
  sends nothing. The lease is not what makes a double send impossible, because it lapses while its
  holder may still be sending; the row claim is. The lease uses the non-macro query path.
- **The job's router.** The job process builds its own router from the policy, the owner's chat and
  the bot's credentials, loaded by a service drop-in with the same two credentials the bot loads.

- **What the at-most-once rule was chosen against.**
  - At-least-once, resending a row that is still unsettled: rejected because a push that reached the
    owner and was never settled is then doubled, which the round-1 design did in 24 of the review's
    population members for a lapsed lease and 16 for a flusher that died after its push.
  - Renewing the lease before each send as the only guard: rejected because a paused or dead holder
    still resends after its last renewal lapses, so the death class stays.
  - Marking the row before the send with no `sending` state (delete or flag it, then push): rejected
    because a failed send, or a death before the push, loses the item with no name.
- **How this relates to ADR-124.** ADR-124 chose a separate job template, `deck-streak-job-send@`,
  that loads the bot's two credentials for the jobs that send, and rejected a drop-in in each sending
  instance's `.service.d` directory because the unit guards admit one instance directory per template.
  This decision's `held_flush` job runs as an instance of the shared job template with its own
  drop-in, the shape ADR-124 rejected, under a ruling on PR #512 that relaxes the guard to an
  allowlist (below); it does not use the separate template. The row claim and its at-most-once rule
  do not depend on which template loads the credentials, so ADR-124's decision on the template stands
  for the jobs it names, and the held flush's different choice is the ruling's.

### Consequences

- Good, because a hold raised in the window reaches the owner at the first scheduled flush after it
  ends, on a day when no sync has landed.
- Good, because a held item reaches the owner at most once: the model's witness of the first design
  (a lease and no row claim, `witness/LeaseLapses.cfg`) violates `NoDoubleDelivery` once the lease can
  lapse and a flusher can crash, the same configuration with the claim holds it, and the repo's tests
  show a flush that outlives its lease and one that dies after its push reached each leaving the item
  sent once.
- Bad, because an item whose flusher died after its push reached is named "may have been sent" and not
  resent, so the owner may see it once and also read it named; and one whose flusher died before its
  push is named too, and never delivered. That is the price of never sending twice.
- Bad, because a flush that starts just before the window opens checks the window once, at its start,
  and does not re-check it at each send; a long flush started at 22:59 can send after 23:00. Held
  items are few (the queue is bounded at 20), so the exposure is seconds.
- Bad, because the lease is a row in the settings table, so the data-rights export lists
  `flush_lease` while a flush holds it.
- Bad, because the table rebuild touches a ledger the deployed database already holds; it is one
  migration, checked by a test that compares the index and trigger sets before and after, and the
  token match of the lease's own release is killed by a mutation row.
- Bad, because the deploy tests' rule that a job template ships exactly one instance drop-in
  directory is relaxed (ruling on PR #512): the held flush needs its own, beside the sync job's, to
  load the bot's two credentials. It is relaxed to default-deny, never removed: an instance
  directory of a shipped template is admitted only for an instance named on one allowlist
  (`sync`, `held_flush`), so any other instance directory is refused. The reader merges every
  instance's drop-in into the template, so the template's expected credentials are now four, and a
  test pins which instance loads which (the bot's two by the held flush alone, the sync login by
  the sync job alone), each with a mutation row whose mutant crosses the pairs.
  - A separate job template for the flush is rejected: it duplicates the template's hardening and
    its unit guards, doubles the deploy surface, and loses the per-instance exactness the shared
    template gives (one credential set judged per instance).
  - Refusing the flush and routing it through the bot service is rejected: the bot is a long-running
    process, so the flush would lose the job ledger, the timer's catch-up and its own failure
    record, which is the shape ADR-300 was chosen against.
  - Loading the bot's credentials in the template is rejected: it would give every job the bot's
    token.
  - Why the allowlist wins over a separate template: a smaller deploy surface, and the exactness
    of each instance's credentials is judged where the instance is named.

## Amendment 2026-10-01: a flush whose work fails after a push

When a flush ended it gave every row still claimed under its token back to `held`, on success and on
error alike. So when its work answered an error after a push had reached the owner (the settle,
the in-app record, the decision row or the commit failed), the row went back to `held` and the
next flush pushed it again. The full render, the recap line and the held reactions each had that
shape. The model shows it: with a step for a flush whose work fails, a push pending or not, the main
configuration violates `NoDoubleDelivery` along `Take`, `Push`, `Fail`, `Take`, `Push`.

Decision: a flush keeps, in memory, the id of each row whose push answered delivered: a held
reaction made, a full render on the bot, and every row a delivered recap line rolled up. When the
flush ends, its release transaction first names each of those rows still `sending` under the flush's
token, `abandoned` with its claim kept, which the next recap reads as "may have been sent"
(`ledger.rs::abandon_pushed`), and only then gives every other row it still claims back to `held`
(`ledger.rs::release_claims`). So a row whose push was attempted is never given back by any path: it
reached the owner once, and if its settle failed it is named. A row the flush never pushed goes back
to `held` and a later flush delivers it.

- A push the transport answered as failed is not in the set: it reached nobody, and it keeps the
  retry rule it had (SPEC-041 R8).
- A Mini App row is not in the set either: its delivery is the in-app write inside the settle's own
  transaction, so an error leaves it undelivered and it goes back to `held`.
- If the release transaction fails too, every row stays `sending` under the token, and the lapse
  path names each one when the claim lapses, as it does for a flush that died.
- The release runs the naming on success as well. A settled row has left the queue and matches
  nothing, so the step needs no branch on how the flush ended.

What the amendment was chosen against:

- Name the pushed rows in the release and give back only the rest: chosen because it holds at most
  once for every attempted push and still delivers each row the flush never pushed. The model is
  clean with it, and its witness `witness/ReleaseOnFail.cfg` keeps the earlier design caught.
- Release nothing on an error and let the lapse path name every row of the lease: rejected because a
  row the flush never pushed is then named "may have been sent" and never delivered, though nothing
  reached the owner, so an error before the first push costs the whole flush.
- A durable mark on each row before its push: rejected because it costs a write and a commit before
  every push and a schema change, while the flush's own set already holds every row it pushed. The
  one case the set cannot cover, a release that fails as well, is the lapse path's.
- Leave the pushed rows `sending` for the lapse path and give back the rest: rejected because the
  name then waits for the claim to lapse, ten minutes, and the give-back needs a query that skips a
  list of ids. Naming them in the release puts them in the next recap.
- Re-run the settle in the release and record the row as sent: rejected because it repeats in a
  second place the writes that just failed (the settle, the in-app record, the decision row at its
  rendered tier), two copies that can drift, and it still needs the name for when that write fails.
- Keep giving back every claimed row on success and error alike: rejected because a row whose push
  reached is then pushed again, which `crates/notifications/tests/flush_fails_after_push.rs` shows
  for the full render, the recap line and a reaction.

Consequences of the amendment:

- Good, because no row is pushed twice when the work after its push fails, and a row the failed
  flush never reached still reaches the owner at a later flush.
- Bad, because a row whose push reached and whose settle failed is named "may have been sent" in a
  later recap, so the owner may see it once and also read it named. That is the price of never
  sending twice, the same as for a flush that died.
- Bad, because a recap whose own settle fails leaves the abandoned rows it named in the queue, so
  the next recap names them again: a repeated name, never a repeated celebration.
- Bad, because the release writes one more conditional update for each row the flush pushed, even
  when every settle succeeded and the update matches nothing.
