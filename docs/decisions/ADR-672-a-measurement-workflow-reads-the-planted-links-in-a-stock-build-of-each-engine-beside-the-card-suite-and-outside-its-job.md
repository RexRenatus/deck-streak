---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-672: A measurement workflow reads the planted links in a stock build of each engine, beside the card suite and outside its job

## Context and Problem Statement

The card suite runs Chromium as its headless shell, and WebKit and Firefox as the builds
Playwright installs, so a link predictor that only a stock build runs is not exercised there
(#941; SPEC-650 section 1). SPEC-650 measures the planted links a predictor could act on in a
stock build of each engine.

## Decisions, and the alternatives each was chosen against

- D1. The measurement is its own configuration and file, so the card suite, its three projects and every census over them stay as they are.
- D2. Chromium's stock build is its full build through the `chromium` channel, run beside the headless shell in the same run as the control; WebKit and Firefox are the builds Playwright installs.
- D3. The observable is the TCP listener's accepted connections, as the suite reads a followed link (ADR-478 D1), read after load, after a hover and after a click, in the reference frame and the card frame.
- D4. It runs as a workflow of its own on a pull request that changes what it reads, as the engine measurement does, so no job of `ci` changes and every pull request pays nothing for it.

Chosen against:
- A fourth project in the card configuration: rejected because the coverage test and the workflow census hold its projects at exactly three.
- A step or a job in `ci.yml`: rejected because it changes the card job's steps or the aggregate's exact needs, and costs every pull request a measurement.
- A dispatch-only workflow: rejected because it runs only from the default branch after it lands, so the delivery's own run would not measure.

## Confirmation

- `card-speculative`'s `measure` job on the delivery's pull request, read by name: its population
  and its controls hold in every project, and each project's measurement is in its log and in the
  `card-speculative-results` artifact (SPEC-650 A1 and A2).

## More Information

SPEC-650; ADR-412 (the card suite's three browsers, with its amendment for #941); ADR-478 (the
TCP listener's count of a followed link).
