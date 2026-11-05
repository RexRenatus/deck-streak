The owner, 2026-10-31, in session: "Sign as drafted."
The owner supersedes the ANK-01 wording of the two signed rulings of the app's surfaces and of the owner's own taps, where they call DeckStreak's study client "a real Anki study client" and "a real Anki client", the owner's primary one: DeckStreak is a spaced-repetition study game for the web, iPhone and iPad, whose own memory model, Rex, becomes its only scheduler before 1.0, with the stock scheduler running until then; it is built on Anki's engine, collection format and sync protocol, and AnkiMobile studies the same collection until the switch (`docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md:12`, `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md:21`; `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:27-28`, `:40`).

# OWNER RULING 2026-10-31: DeckStreak's identity

## What was held

At dev `e25c0c45`:

| where | the text held | the owner's answer it meets |
|---|---|---|
| `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md:12` | "ANK-01: a real Anki study client, the owner's primary one (OQ1), and answering a card is a write back" | DeckStreak is its own study game, built on Anki's engine; answering a card stays a write back |
| `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md:21` | "The study client is a real Anki client (ANK-01) and the owner's primary one (OQ1), and the owner chose full AnkiMobile parity for their own taps (ANK-03)." | the study client is DeckStreak's own, the owner's primary one, on Anki's collection format; ANK-03's parity for the owner's own taps is unchanged |
| `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:27-28` | "the owner chose a real Anki study client on the web and on a universal iPhone and iPad app, over the same engine, as the owner's primary Anki client" | the two apps are DeckStreak's own study clients over Anki's engine |
| `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:40` | "R2. (ANK-01, ANK-05, ANK-06) The study client is a real Anki client: Anki's engine, pinned to one upstream tag, opens the collection, presents cards and records answers." | the engine half stands; the "real Anki client" framing reads as DeckStreak's own study client |

## What replaces it

- **ANK-01's identity wording.** DeckStreak's study client, on the web and on the universal iPhone and iPad app, is
  DeckStreak's own client in a spaced-repetition study game, the owner's primary one. It is built on Anki's Rust
  engine, which opens the owner's Anki collection, presents cards and records answers over Anki's sync protocol,
  against the owner's own sync server; answering a card stays a write back. Rex, DeckStreak's own memory model,
  becomes its only scheduler before 1.0 (CHARTER item 22); until then the stock scheduler schedules every review, and
  AnkiMobile studies the same collection through the same server.
- **What stands.** ANK-01's experimental-capability half and every decision built on it (ADR-338, ADR-339), ANK-03's
  parity for the owner's own taps, the owner-taps exemption and its containment, ANK-05, ANK-06 and ANK-08, CHARTER
  item 22 and every write-class rule are unchanged by this ruling.
- **Order.** The two rulings of the app's surfaces and of the owner's own taps keep their text; they are records.
  SPEC-334 keeps its text: the commit that lands this signed ruling on dev appends one insert-only section to SPEC-334
  naming this ruling and stating the replacement above (`SPEC-334-amendment.final.md`, private until then). The README,
  NOTICE, AGENTS, PRD and owner setup texts of SPEC-580 already state only what holds under either wording, and do not
  wait for this ruling.

## Why it is admitted

It changes how DeckStreak names itself, not what any client does: the engine still opens the collection and records
each answer, every write still passes its write class or the owner's own tap, and no scheduler, surface or setting
changes. It brings the rulings' wording in line with the repository's description and with CHARTER item 22.

The rejected alternatives:
- **Keep "a real Anki study client" as the wording.** It contradicts the identity the repository now states, and a
  reader of the rulings would take DeckStreak for an Anki client.
- **Rewrite the two rulings and SPEC-334 R2 in place.** Signed rulings and the requirements they decide are records;
  their text is kept, and a later ruling supersedes.
- **Supersede all of ANK-01, its experimental half included.** That half decides ADR-338 and ADR-339, which this
  identity change has no reason to touch.
