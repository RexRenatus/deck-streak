---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The readings in the bot: `/prestudy`, replies that are not notifications, deep links that carry the reading, and a morning line of its own kind

## Context and Problem Statement

The owner's on-demand ruling is tap to pick, never a typed topic, and the command must not be
confusable with the minutes habit's `/read`. The bot has two kinds of output: replies to the owner's
own commands, and notifications the router decides. The notifications-policy pack holds every
delivery call to the router module, and its message rows judge every `*.msg.json` as a routed kind.
How are the command, its buttons, its golden and the morning line shaped?

## Decision Drivers

- One router for celebrations and nudges (constraint 2), and the `one-router` row over the tree.
- A button must reach the reading itself, not only the app's front page.
- Golden messages must be judged by the packs that own Telegram's rules, without claiming a policy
  kind a reply does not have.
- During side by side the predecessor sends its own morning brief.

## Considered Options (the alternatives it was chosen against)

- `/prestudy` answered through the bot's reply path, with URL buttons to `t.me/<bot>/<app>?startapp=r_<reading id>` and `rg:` callbacks; the reply's golden kept outside the `*.msg.json` naming and checked by telegram-platform's own functions; the morning line routed as the `reading_ready` kind at the readings' own morning slot — chosen: replies stay replies, buttons land on the reading, and every message is judged by the pack that owns its rules.
- Route command replies through the router too — rejected because a reply answers the owner's own message now; budgets, quiet hours and dedupe would withhold an answer the owner just asked for.
- `web_app` inline buttons — rejected because they open the Mini App's URL without a `startapp` value, so the token map could not route to the reading; a direct link carries it.
- Commit the reply's golden as a `*.msg.json` envelope — rejected because the notifications-policy pack's `message-metadata` requires a declared kind on every envelope, and a reply has none.
- Put the readings line into the morning brief — rejected because during side by side the brief is the predecessor's, and DeckStreak sends only kinds the predecessor does not (ADR-011).
- Call the command `/reading` — rejected because it sits one character from `/read`, the confusion the owner's record already warned of.

## Decision Outcome

Chosen option. The bot crate names its handler `prestudy` (the lexicon's lock on that word covers the
readings context only). The listing and its pick tokens come from the same today view the Mini
App's Today reads (SPEC-051), so both surfaces run one code path; the bot builds the deep links
from its configured username and the Mini App's short name. The morning line is a nudge of the kind `reading_ready`, which
quiet hours and a lapse withhold like every nudge.

### Consequences

- Good, because a tap in Telegram opens the exact reading in the Mini App.
- Good, because the router's one call site stays the only one.
- Bad, because the reply's golden is judged by a test that calls telegram-platform's functions rather
  than by the pack's tree rows; the test runs in the gate.

### Confirmation

SPEC-052's tests; the notifications-policy `one-router` and `message-metadata` rows; telegram-platform's
payload checks.

## What would make this wrong

- The notifications-policy pack learns a reply envelope that carries no kind (then the reply's golden
  becomes a `*.msg.json`).
- Telegram adds a `startapp` value to `web_app` buttons (then the buttons can become `web_app` buttons).

## More Information

SPEC-052; SPEC-041 (the `reading_ready` kind and its deviation); SPEC-048 (pick tokens); the
telegram-platform pack's `tg-deep-link` and `payload-keyboard` rows.
