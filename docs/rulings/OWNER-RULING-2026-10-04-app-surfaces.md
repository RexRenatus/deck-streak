The owner amends CHARTER items 2, 4 and 14, the PRD's third non-goal and DESIGN_SYSTEM's first two principles, in the owner's own words (owner, 2026-10-04): "Retire both" (SCP-03), "option 1 with experimental capability for fsrs7 and rwkv-instant" (ANK-01), "Show them, scope the rule (Recommended)" (STAT-01), "Vivid in session, calm out (Recommended)" (GAM-03), "Dark-first, colour-rich game (Recommended)" (UX-02), and "send the commits to me on telegram for signature", "i gove you peision for the rulings" and "try again for ruling": DeckStreak's surfaces become a web client, a universal iPhone and iPad client and native push, still one router and one owner; its study client writes the owner's own answers; the private study client shows the scheduler's output; and the design system becomes dark-first, vivid in session and calm outside it (`CHARTER.md:28-29`, `:33-34`, `:72-76`; `docs/PRD.md:29`; `docs/DESIGN_SYSTEM.md:8-10`).

# OWNER RULING 2026-10-04: the app's surfaces

## What was held

At dev `df2a4cdb`:

| where | the text held | the owner's answer it meets |
|---|---|---|
| `CHARTER.md:28-29`, item 2 | "Two surfaces, one policy. The Mini App and the bot are two views of one service." | SCP-03 "Retire both": the Mini App and the bot retire once the web client, the iPhone and iPad client and native push (APNs and web push, NOTIF-01) carry the router |
| `CHARTER.md:33-34`, item 4 | "Pull, then read ... The skip day is the ONLY write back to Anki." (since widened by ADR-301's write classes, owner ruling 2026-10-01) | ANK-01: a real Anki study client, the owner's primary one (OQ1), and answering a card is a write back |
| `CHARTER.md:72-76`, item 14 | the bot gates on the owner's identity; the Mini App validates Telegram `initData` | SCP-03: once Telegram retires neither mechanism exists, and "The service answers one owner" must survive |
| `docs/PRD.md:29`, non-goal 3 | "Any countdown, exam date or forward-looking timeline." | STAT-01: the private study client shows the scheduler's own output, as Anki does |
| `docs/DESIGN_SYSTEM.md:8-9`, principle 1 | "Native to Telegram: the Mini App takes Telegram's theme ..." | UX-02 and SCP-03: an identity of its own once Telegram retires |
| `docs/DESIGN_SYSTEM.md:10`, principle 2 | "Calm by default: celebrations are earned and rare ..." | GAM-03: vivid in session, calm out of it |

## What replaces it

- **Charter 2: one router, one policy.** DeckStreak's surfaces are a web client, a universal iPhone and iPad client
  and native push (APNs and web push). Every celebration and nudge still passes through ONE router, so an event can
  never celebrate twice. The Mini App and the bot remain until the new surfaces carry the router, and the v9 cutover
  runs on today's surfaces. Native nudges are opt-in, at most one a day (NOTIF-01).
- **Charter 4: pull, then read, and the owner's own study.** The private copy stays read-only for the game. The study
  client writes the owner's own answers, and the owner's own gestures under
  `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`. Every other write goes through a declared write class
  (ADR-301). An experimental model only reorders cards the stock scheduler already made due (OQ4), except FSRS-7 on
  the one preset the owner switches (OQ1).
- **Charter 14: one owner, on every surface.** "The service answers one owner" is unchanged. Sign-in on the web and on
  the iPhone and iPad pins to the one owner account, which is configuration held in a secret, by the methods ADR-131
  and ADR-132 specify (AUTH-01). The Telegram `initData` gate stays while the Mini App and the bot exist. Commands with
  stakes or destructive effects carry the same gate on every surface. No surface opens a session before its gate is
  built.
- **PRD non-goal 3** reads: "Any countdown, exam date or forward-looking timeline on DeckStreak's game, public or AI
  surfaces." The study client shows the stock scheduler's output as Anki does: the next interval on the answer
  buttons, a card's due date, the browser's Due column and the future-due forecast. Charter 12 is unchanged.
- **Design principle 1: an identity of its own.** A dark-first, colour-rich palette with its own reference tier and
  role mappings, WCAG 2.2 AA in both modes. The Mini App keeps its Telegram mapping until it retires.
- **Design principle 2: vivid in session, calm out of session.** In a session: haptics, motion, sound, instant XP and
  ladder celebrations. Outside one: nothing pressures, shames or fabricates, and at most one nudge a day. Every
  charter 10 anti-goal is kept.
- **Order.** Each document keeps its text until the delivery that builds against it adds a dated note naming this
  ruling, as the 2026-10-01 ruling did for ADR-301.

## Why it is admitted

Each change is the owner's own answer in the design rounds (2026-10-04). Charter 10, charter 12 and the never-list's
binding on every write class are untouched.

The rejected alternatives:
- **SCP-03, keep both** (main's recommendation). The owner chose to retire both.
- **SCP-03, retire the Mini App and keep the bot.** The owner chose native push over the bot.
- **STAT-01, hide the scheduler's output everywhere.** It leaves the study client short of Anki, which the owner chose
  to match.
- **GAM-03, a full engagement loop.** It breaks charter 10's anti-goals outside a session.
