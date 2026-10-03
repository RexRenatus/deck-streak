# SPEC-323: the notification-policy check judges notifications only

- **Wave:** W4. **Issue:** #257. **Context(s):** `deck-streak-notifications` (the policy file and its
  typed loader).
- **Decided by:** ADR-323 (command replies are a declared reply class of the policy). Amends
  SPEC-026 (section 8) and ADR-026, each append-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-323.md`. **Mutation band:** `S32300-S32399`.

## 1. The problem, measured

Measured at dev `02758d40b`.

- Every committed golden message is a command reply: 26 files under `crates/bot/tests/messages/`
  (`*.msg.json`: badges-failed, erase-done-log-held, erase-done, erase-expired, erase-failed,
  erase-prompt, export-failed, export, help, long-sample.1, long-sample.2, privacy, records-failed,
  score-failed, score-no-retention, score-none, score, start, sync-failed, sync-not-run,
  sync-refused, sync-reused, sync-scores-refused, sync-still-running, sync-synced, sync-unchanged).
  Each carries exactly `duty`, `schema`, `send` and `x-message`; `duty` is `bot-commands`; none
  carries a `kind`. No other `*.msg.json` exists in the tree. The bot's fake API writes them
  (`crates/bot/tests/support/fake_bot_api.rs:365-371`).
- The pinned notifications-policy probe, run in scratch over an extract of dev: `message-metadata`
  exits 1 with 26 findings `kind None is not a kind the policy declares`, examined 52; the 13 policy
  rows and `one-router` exit 0.
- The box run therefore defers that row to #257 in the maintainer's private wiring (ADR-069). The
  tree only states the deferral: SPEC-026 R12 and its manifest row, ADR-026, SPEC-041 section 3a.
- The pack already has the selection rule: a top-level `replies` list in the policy file. An envelope
  is skipped as a reply only when it carries no `kind` key, its `duty` is a string in `replies`, and
  every policy-reading class is clean; a skipped reply is still counted. The router's loader refuses
  any top-level key outside `TOP_KEYS` (`crates/notifications/src/policy.rs`), so today a policy file
  carrying `replies` stops the router at start.

## 2. Requirements

R1. `notifications-policy.json` ends with the top-level key `"replies": ["bot-commands"]`, the
    duty of the 26 command replies.
R2. The typed policy reads `replies` as a required key (`Missing` when absent) and writes it back,
    so the written-back policy still equals the file (SPEC-041 A1).
R3. The policy refuses at start a `replies` entry that names a declared kind
    (`Malformed`, key `replies`): a notification is judged, never skipped as a reply.
R4. Every committed golden message is either a notification carrying a `kind`, or a command reply
    with no `kind` whose duty the policy names in `replies`; and every `replies` entry is the duty
    of at least one kind-less golden. No golden moves on disk.
R5. Nothing in production reads `replies`: the router routes notifications only, and the check is
    the reader.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every committed golden message is a notification carrying a `kind`, or a command reply with no `kind` whose duty the policy names in `replies`, and every `replies` entry is the duty of a kind-less golden; a planted kind-less golden of duty `daily-digest` and a planted `replies` entry no golden carries are each refused by name | `cargo test -p deck-streak-notifications --test policy -- --exact every_golden_is_a_declared_notification_or_a_declared_reply` |
| A2 | the policy refuses at start a `replies` entry that names a declared kind (`Malformed`, key `replies`), and a file without `replies` (`Missing`, key `replies`) | `cargo test -p deck-streak-notifications --test policy -- --exact a_reply_that_names_a_declared_kind_is_refused_at_start` |

```acceptance
A1: cargo test -p deck-streak-notifications --test policy -- --exact every_golden_is_a_declared_notification_or_a_declared_reply
A2: cargo test -p deck-streak-notifications --test policy -- --exact a_reply_that_names_a_declared_kind_is_refused_at_start
```

Two box-run criteria carry no fence line, because no public test can run a pack's row (SPEC-041
section 3a's shape): B1, every row of notifications-policy passes over the tree with
`message-metadata` no longer deferred, reading `message-metadata: replies skipped 26: bot-commands=26`
with the 13 policy rows and `one-router` at their counts; B2, telegram-platform's three payload rows
still examine the 26 goldens. B1 is measured by the pinned probe over a scratch copy of the wiring
without the deferral (the maintainer removes the real entry when this lands).

## 4. File manifest

| file | context | change |
|---|---|---|
| `notifications-policy.json` | `deck-streak-notifications` | `replies` key added |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | `TOP_KEYS`, the `replies` field, `parse`, `check` |
| `crates/notifications/tests/policy.rs` | `deck-streak-notifications` | the two tests and a selection helper |
| `docs/specs/SPEC-323-the-notification-policy-check-judges-notifications-only.md` | docs | added |
| `docs/decisions/ADR-323-command-replies-are-a-declared-reply-class-of-the-policy.md` | docs | added |
| `docs/specs/SPEC-026-bot-transport-and-owner-gate.md` | docs | section 8 appended |
| `docs/decisions/ADR-026-bot-updates-by-long-polling.md` | docs | an amendment appended |
| `docs/schematics/notification-policy-check-population.md` | docs | added |
| `docs/red-first/SPEC-323.md` | docs | added |
| `scripts/mutation-rows.d/S32300-S32399.json` | scripts | added |
| `changelog.d/policy-replies-257.md` | changelog | added |

## 5. What this does NOT do

At this head the notification-policy check's message-metadata row judges 0 notification envelopes:
all 26 committed goldens are command replies and are skipped as replies, so its green measures no
notification. A1 judges every committed golden and the planted cases.

- No notification golden is added: the first ones belong to the deliveries that send the
  notifications (#122, #129, #123).
- The stale sentences that say the message metadata "stays deferred (#257)" are left to their own
  deliveries. Measured at the base: `docs/specs/SPEC-041-notification-router-core.md:150`,
  `docs/specs/SPEC-102-landmarks-and-milestones-are-celebrated-once-and-the-widget-is-one-pinned-message-a-study-day.md:264`,
  `docs/specs/planned/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md:433`,
  `docs/specs/planned/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md:282`,
  and `docs/specs/planned/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md:12`
  and `:27`. SPEC-041's is superseded by the next SPEC-041 amendment (#297); the others are owned by
  #127, #122, #129 and #60.
- The private wiring's deferral entry is not edited here: it is the maintainer's (ADR-069, #257).
- The bot's goldens and `scripts/tests/test_bot_messages.py` are unchanged: a command reply stays
  judged on its payload by the telegram-platform rows and by that test (#257).

## 6. Risks

- A reply that becomes scheduled, budgeted or deduplicated would need a kind; the selection test
  refuses a kind-less golden whose duty is not declared, so a new duty is caught at its first golden.
- A notification golden moved under a reply duty: it carries a `kind`, so it is judged as always.
- A stale `replies` entry: refused by A1's second rule.
- A policy loader and a probe that disagree on the key: the loader types it and the box run reads
  the same file.

## 7. The mutation rows

Band `S32300-S32399`, in `scripts/mutation-rows.d/S32300-S32399.json`: S32300 to S32304, each proved
with `python3 scripts/mutation_rows.py prove --row <id>` on a committed tree.
