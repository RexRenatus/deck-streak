---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-589: DeckStreak is the web app and the iPhone and iPad app; the Telegram Mini App and bot are deprecated, and Rex is the scheduler

## Context and Problem Statement

DeckStreak began as a Telegram Mini App and bot (CHARTER 2 as first written). The owner's design rounds then chose a
web client and a universal iPhone and iPad client over the same engine (SPEC-334, ADR-335, ADR-336) and native push
through the one router (ADR-341), and ruled that the Mini App and the bot remain until those carry the router, and that
the v9 cutover runs on them (`docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`). Rex, DeckStreak's own memory
model, entered as its only scheduler (`docs/rulings/OWNER-RULING-2026-10-18-rex-enters-deckstreak.md`, the amendment
of ADR-339), but the charter does not name it, and the public documents still call DeckStreak a Telegram Mini App. The
owner has now ruled (`docs/rulings/OWNER-RULING-2026-10-29-web-and-ios-and-rex.md`): the Mini App and the bot are
deprecated, "everything is being folded into the webapp and ios app", the v9 cutover moves to the web app, DeckStreak
stays owner-only at launch, and the public text names Rex and its method. What do DeckStreak's surfaces, its owner
gate and its scheduler become, and what happens to the hundreds of records that say otherwise?

## Decision Drivers

- One router, one policy (CHARTER 2): more surfaces never mean a second policy.
- Owner-only is a safety property (CHARTER 14), on every surface, at launch.
- The repository is public and the learner private (CHARTER 11): the method is public; the licensed data, the raw
  research and every per-learner row are not.
- Records stay records: a landed SPEC, an ADR, a ruling or a changelog fragment is history and is not rewritten.
- No surface may lose its owner gate while its code still runs.

## Considered Options (the alternatives it was chosen against)

- Deprecate both Telegram surfaces now, name the two apps as DeckStreak's surfaces, amend the charter by one dated
  note that quotes each replacement, and remove the Telegram code in its own delivery — chosen because the owner chose
  it.
- Restate the charter in place — rejected because the owner chose to add notes, as every earlier amendment did: the
  charter keeps its earlier text, and the note below it binds.
- Remove the Mini App and the bot in this change — rejected because the bot still carries the router's only built
  transport and the Telegram owner gates; removing them before native push carries the router leaves no way to reach
  the owner outside the apps.
- Keep the bot as a notification channel beside the apps — rejected because the owner deprecated the bot.
- Keep Rex unnamed in public text — rejected because the owner chose "Name Rex and its method".
- Rewrite every record that names the Mini App — rejected because records are history; this ADR supersedes their
  statements instead.

## Decision Outcome

Chosen option: deprecate, amend by note, and remove later.

- **The surfaces.** DeckStreak's surfaces are the web app (`web/app`, SvelteKit) and the universal iPhone and iPad app
  (`ios/`, SwiftUI over the engine through FFI), with APNs and web push as the router's transports outside a session
  (ADR-341). Every celebration and nudge passes through the one router.
- **Deprecated.** The Telegram Mini App and the Telegram bot are deprecated: neither gains a feature, and no current
  text presents either as a current surface (where CHARTER keeps its earlier text, its dated note supersedes it). As
  signed on 2026-10-04, they remain until native push carries the router; the v9 cutover, which that ruling ran on
  them, runs on the web app (CHARTER 2, as its dated note words it). Their code (`crates/bot/`, `identity`'s
  `initData` port, the Mini App's Telegram mapping in `web/app`) stays until one removal delivery takes it out with its
  own ADR, after native push carries the router (ADR-341); since the cutover runs on the web app, the removal no
  longer waits for it.
- **The owner gate.** The service answers one owner. Sign-in on the web app and on the iPhone and iPad app pins to the
  one owner account by the methods ADR-131 and ADR-132 specify. The Telegram gates stay while their code runs, and
  leave with it.
- **The scheduler.** Rex is DeckStreak's only scheduler, as CHARTER 22 states with its method: a recurrent model with a
  memory per card and per study day, a curve head over whole days aimed at the home preset's desired retention,
  same-day learning and relearning steps, two grades (Again and Good), and live inputs that include reading and
  exposure events and content embeddings. A frozen version runs in a Rust core on the device, fed weights through its
  one format, `.rexw`; the server relays and never computes Rex state. It is trained centrally on research data only,
  fitted against FSRS-6 and FSRS-7 on development learners and a sealed set, judged by a scheduling simulator, and
  promoted only when scheduling-ready with its proofs and release gates met. With no promoted version, no Rex due is
  written. Every other model stays reorder-only (ADR-339).
- **Superseded statements.** Wherever a landed SPEC, an ADR, a schematic, a ruling or a changelog fragment calls
  DeckStreak a Telegram Mini App, names the Mini App or the bot as a current surface, or names Telegram `initData` as
  the owner gate of a current surface, this ADR supersedes that statement; the record's text is kept. A planned SPEC
  that targets a Telegram surface is re-targeted to the two apps by the delivery that builds it, which adds its dated
  note.

### Consequences

- Good, because the charter's dated note, the README and the architecture now describe what DeckStreak is and how it
  schedules.
- Good, because no record is rewritten: history stays readable, and one ADR names what changed.
- Bad, because the charter reads in layers: its earlier text stays as it was, and the dated note below it binds.
- Bad, because the tree keeps deprecated code and setup steps until the removal delivery, each marked deprecated.
- Bad, because identifiers keep their earlier names (`miniapp` for the web app's front end and its mutation records)
  until a rename delivery.

### Confirmation

- The docs check of the delivery that lands this ADR: README, ARCHITECTURE, AGENTS and the PRD call DeckStreak a
  Telegram Mini App nowhere and name the Mini App or the bot as a current surface nowhere; CHARTER keeps its earlier
  text, names Rex in item 22, and its dated note quotes each replacement; it reads red on the earlier text.
- `scripts/tests/test_declared_write_classes.py` stays green: CHARTER item 4 and its dated note are kept byte for byte.
- The signed ruling `docs/rulings/OWNER-RULING-2026-10-29-web-and-ios-and-rex.md` is on `dev` in the same commit.

## What would make this wrong

- The v9 cutover, which runs on the web app, cannot complete there; the owner then rules where it runs, the removal
  delivery waits for that ruling, and this ADR's "deprecated" still holds.
- A platform rule makes native push unable to carry the router; the bot's transport would then outlive its
  deprecation, which needs a ruling on CHARTER 2.
- Rex's weights reach a build someone other than the owner uses before written permission for its training data is on
  file; CHARTER 22 forbids it.

## More Information

- `docs/rulings/OWNER-RULING-2026-10-29-web-and-ios-and-rex.md` (CHARTER mission, 2, 14, 19, 22).
- `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`, `docs/rulings/OWNER-RULING-2026-10-18-rex-enters-deckstreak.md`.
- SPEC-001, SPEC-334; ADR-131, ADR-132, ADR-335, ADR-336, ADR-339 and its amendment, ADR-341.
