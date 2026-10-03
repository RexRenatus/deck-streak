---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, ruling on the design pass for #257"
---

# Command replies are a declared reply class of the policy

## Context and Problem Statement

All 26 committed golden messages are replies to a command a person typed, written in the same
envelope as a notification but with no `kind`. The notification-policy check's `message-metadata`
row judges every envelope as a notification and reads `kind None is not a kind the policy declares`
on all 26, so the box run defers that row (#257). How does the check tell a reply from a notification
without letting any notification escape judgement?

## Decision Drivers

- A reply is never scheduled, budgeted, deduplicated or deferred, so no kind describes it (ADR-026).
- The pinned probe already reads one selection rule, a top-level `replies` list in the policy file.
- The router's loader refuses a top-level key it does not know, so the same file must be typed.

## Considered Options (the alternatives it was chosen against)

- The policy file declares `"replies": ["bot-commands"]`, the loader types it, and refuses an entry that names a declared kind (chosen).
- A path convention, a golden under a bot `tests/messages/` is a reply: lost, because the pinned probe reads no path rule, so the deferral could not end, and any notification golden could be moved into that directory to escape judgement.
- A registry the router owns: lost, because the router never sees a reply (a reply leaves through the bot's send helper, never `route`), so the list would be a second record the router never reads, and the check reads the policy file, not router code.
- A reply kind, or a reply that names a declared kind: lost, because ADR-026 rejected it (a reply would look like an alert or a nudge) and the probe refuses a `replies` entry naming a declared kind.
- A field each golden carries (`"notification": false`, an `x-` key): lost, because it is a self-declared exemption per file, which the probe does not read.

## Decision Outcome

Chosen option: "the policy declares the replies' duty", because the rule then lives in the file the
check reads and the box run applies.

- `notifications-policy.json` gains, as its last key, `"replies": ["bot-commands"]`.
- The typed policy gains a required `replies` list. `check()` refuses an entry that names a declared
  kind (`Malformed`, key `replies`). Nothing in production reads the list.
- A test over the committed goldens holds the population: each is a notification with a `kind`, or a
  kind-less reply whose duty is declared, and each declared duty is carried by a kind-less golden.

### Consequences

- Good, because the `message-metadata` deferral can end and the check judges any future notification.
- Good, because a notification golden under a reply duty is still judged: it carries a `kind`.
- Bad, because at this head `message-metadata` judges 0 notification envelopes (all 26 are replies and
  are skipped), so its green measures no notification until one exists (#122, #129, #123).
- Neutral, because a command reply stays judged on its payload by the telegram-platform rows and by
  `scripts/tests/test_bot_messages.py`.

### Confirmation

`crates/notifications/tests/policy.rs` (A1, A2); rows S32300 to S32304.

## What would make this wrong

- A reply that becomes scheduled, budgeted or deduplicated, which then needs a kind.
- The pack's selection rule changing so that a declared duty no longer skips a kind-less envelope.

## More Information

SPEC-323; SPEC-026 (section 8); ADR-026 (amended); ADR-041; ADR-069; issue #257.
