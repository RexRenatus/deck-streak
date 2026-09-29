---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Images are drawn through a port with no provider wired, by a sending job, capped, cached and gated, and the fold only enqueues

## Context and Problem Statement

#125 (share cards) and #126 (keepsake art) ask for AI images at a milestone and a chapter
ceremony. The predecessor drew them inline, through one hosted image API with a configurable model,
cached by event key, two a day, falling back to text on any failure (`art.py:generate`,
`art.py:ART_DAILY_CAP`). The image provider is the owner's decision (#169), and the AI route is
optional with no-AI mode the default (ADR-054). Where does a draw run, what stops it from blocking
a ceremony, and what is wired before #169 is answered?

## Decision Drivers

- A provider's failure, timeout or absence never blocks a streak, a ceremony or a grant.
- The fold is deterministic and replayable: it makes no network call.
- Every message goes through the one router (ADR-041).
- A generated image passes the output gate before anyone sees it.
- No provider credential in the repository or the environment (SPEC-066).

## Considered Options (the alternatives it was chosen against)

- A port with no provider wired, drawn by the sending job `image_art`: chosen, because the fold
  only enqueues rows and stays free of calls, and a draw's failure touches only its own row.
- Draw inline in the fold, as the predecessor did: rejected because a 90-second call would sit
  inside the fold, and a replay would call the provider again.
- Draw from a request handler when the owner opens the gallery: rejected because a milestone's
  card would then wait for the owner to look, and a page load would wait on a provider.
- Wire a provider now: rejected because the provider is the owner's decision (#169), and no-AI mode
  is the default (ADR-054).
- A second send path for photos beside the router: rejected because every message goes through the
  one router (ADR-041), whose ledger makes a send happen once.

## Decision Outcome

Chosen option: "a port with no provider wired, drawn by a sending job", because it keeps the fold
pure, keeps every send on the router, and makes the product with no provider the tested default.

- **The port.** `agent::images::ImageProvider`, one call answering an image or a closed failure;
  the daemon wires `NoImageProvider`, which is off.
- **The draw.** One outcome per row, in order: no provider, cached, capped (two draws a study day,
  shared by both kinds, counted before the call), then the call bounded at 90 seconds.
- **The gate.** The agent's output gate judges each image's prompt and caption; a failing class
  withholds it, and the image is discarded.
- **The job.** `image_art` runs hourly on the sending template (ADR-124, planned) and raises each
  ready image through `Router::route_photo`.
- **The credential.** `image-provider-key`, optional through the kernel's loader, named by no unit
  until #169's delivery binds it.

The options for #169, recorded for the owner's decision:

- A hosted image API reached through an aggregator, one key for several models, as the predecessor
  drew: the fewest moving parts, and the model a configuration value.
- One vendor's image API directly: one contract fewer, at the cost of a new adapter if the vendor
  changes.
- No provider: the default stands, and the product is the text ceremony and no share card.

### Consequences

- Good, because the product with no provider is what every test and every deploy runs until #169.
- Good, because a draw is counted before its call, so a crash mid-call cannot exceed the cap.
- Bad, because an image reaches the owner up to an hour after its milestone.

### Confirmation

SPEC-135 §3 (the draw's outcomes, the no-provider criterion and the job) and its rows in
`S13500-S13599`; SPEC-136 §3 for the share card's path.

## What would make this wrong

- The owner wants the image in the same message as the ceremony: the job would run right after the
  fold's run, and the ceremony would still never wait for it.
- A provider whose answer takes longer than 90 seconds: the bound would move, and the job's
  schedule with it.

## More Information

SPEC-132, SPEC-135, SPEC-136, SPEC-043, SPEC-066, SPEC-074, SPEC-100, ADR-041, ADR-054, ADR-124
(planned), and the W7 schematic `docs/schematics/w7-image-pipeline-and-its-no-provider-path.md`.
