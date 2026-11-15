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

## Amendment: the card frame's click-step connection in Chromium's full build is an accepted residual, and what it was chosen against

SPEC-650 section 7 records the reading: in Chromium's full build, the card frame's click step reads
one TCP connection with no request for six cards, a preconnect the browser starts as the click
starts, toward the card link's own origin (the count in run 42542817356, `measure` job 128921292916, and the preconnect in `measure` jobs 128976741152 and 129037052776). Every
layer of the card frame is on, W1 to W4 (ADR-352 D7): W4 still refuses the follow, W2 still refuses
every fetch, and no request is read on the connection. No rule of the frame governs a connection the
browser opens ahead of a request (ADR-498 D2), and the headless shell the card suite runs opens
none, so nothing the suite measures changes.

- D5. The connection is recorded as a residual the owner accepts, and the measurement keeps printing and recording it unjudged (SPEC-650 R3). No card, frame, step, control, project, assertion, census entry or UNOBSERVABLE entry is removed, relaxed or added, and the card suite, its configuration and the `card-sandbox` job are unchanged.

Chosen against:
- The strip removing every `href` from the card's links (ADR-498 D2's R2): rejected because it narrows a gate: it rewrites the six cards' W4-alone cells, drops their single-layer "W4 off: opens" controls, and leaves W4 no channel of its own, which `web/app/src/lib/card/planted-coverage.test.ts:316` asks of each layer.
- The host turning pointer events off for the nested frame: rejected because it narrows a gate and changes the study screen: no card could be scrolled or its media played by pointer, and every variant built through the host would stop reaching the card's link, so the single-layer "opens" controls of W1 and W4 would read red.
- A style or a policy directive in the frame: rejected because card CSS can undo a style, and no directive governs the preconnect (ADR-498 D2).
- An exception or a declared cell in the card suite: rejected because the suite's Chromium, the headless shell, opens no such connection, and a declared exception narrows the gate (ADR-498 D2's R3), which SPEC-650 section 5 also keeps out.
- A judged assertion in the measurement: rejected because no cure is taken that would make it pass, so it would read red at every head, and SPEC-650 R3 judges no reading.
- A further round of feature switches: rejected because no switch measured turns the connection off (SPEC-650 section 7), and a switch is no cure: a learner's browser runs its own settings.

Consequences:
- Bad, and accepted as this decision's residual: in Chromium's full build a click on a card's link opens one connection to the origin the link names, with no request on it. ADR-478's condition that "An engine build the suite does not run opens a speculative connection toward a link" reads true for that build, and so does ADR-498's first condition, a link with no target reaching a preconnect in the shipped frame; the cure ADR-498 records stands for the engines the suite runs.
- Good, because every layer, test and control stays as it is, and the reading stays in the measurement's log and artifact, so a build that stops opening the connection shows as a changed reading.

What would make this amendment wrong:
- A later reading holds a request or the card's path on that connection: the residual is no longer the one accepted, and the frame needs a cure of its own (ADR-478).
- A later reading holds a connection at another card-frame step, or in the headless shell, WebKit or Firefox: what the suite measures moves, and the residual is reopened.
- A page-side rule is found that stops the connection without narrowing a gate: the cure is designed then.
