---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# The weekly report names this week's stored instrument reports and splices none

## Context and Problem Statement

The predecessor's weekly report appends, after its own blocks, one text block per weekly research
instrument, each computed while the report is built and each omitted when it has nothing to say,
then sends a chart image (`pipeline_layers/digests.py:DigestsLayer.run_weekly_report`, predecessor
`27ee2bc`). #130 keeps the report as the owner's Sunday message, and its Mini App note calls it too
long for chat: a scrollable report with a section per instrument, and a short push that links to it.

W4 already moved the instruments out of the report. They run after a sync once in seven study days,
or on demand, and coordination stores each one's latest report (ADR-094, SPEC-094 R7 and R9). The
insights screen renders each stored report as a section, a failed read included (SPEC-094 R13). W4
ported no instrument's chat text: the report's JSON and the screen are its only renderings. ADR-094
says the weekly report reads the stored reports, and leaves open how it shows them. How does the
weekly report carry the week's research?

## Decision Drivers

- One router, one message a week for the report, and one class of message for it (SPEC-041).
- A report with a failed read never reads as an all-clear (SPEC-094 R13, learning-science).
- Every text the report carries equals a golden of the predecessor's own function, or is new and
  says so (ADR-012).
- Notifications reads no insights type: the crate graph gives notifications the kernel alone.
- Charts are drawn on the client, and the host renders no image (ADR-085).

## Considered Options (the alternatives it was chosen against)

- One research line naming this week's stored reports by id, marking a failed read, with a button to the insights screen — chosen, because the report stays one short message, each report is rendered once on the screen that already renders it, and a failed read is still visible in chat.
- Splice each instrument's text block into the report, as the predecessor does — rejected because no instrument's chat text is ported, so nine texts and their goldens would join this wave, and each block would render the same report a second time beside the screen's section.
- One message per instrument — rejected because a week would bring up to nine research messages beside the report, each needing its own kind and budget, against one message for the report.
- Omit the research from the report — rejected because the owner would learn of a new report only by opening the screen, and #130 asks the report to carry the week's research.
- Have insights render each report as text and pass the strings through coordination — rejected because it still splices up to nine blocks into one chat message, and still owes nine ported texts.

## Decision Outcome

Chosen option: "one research line naming this week's stored reports, with a button to the insights
screen".

- The line names each weekly instrument whose stored report's study day falls in the report's week,
  by its registry id and in the registry's order, and marks a report with a failed read `
  (incomplete)`. It is empty when no report falls in the week.
- The report's row carries `📈 Road to C2` (the startapp token `progress`) and, when the line shows,
  `🔬 Research` (the token `insights`). The Road to C2 chart becomes the button, and the progress
  screen draws the chart (ADR-085).
- The report is the new kind `weekly`, class `digest`, one a study day, behind its own switch
  `weekly_enabled`, recorded in the policy's deviations with this ADR. It is not the kind `digest`,
  so the owner can keep the daily digest and turn off the weekly report, or the reverse.
- The weekly report runs no instrument. An instrument's failure is isolated where it runs (SPEC-094
  R7), and each of the report's own blocks is isolated where it is read.

### Consequences

- Good, because the Sunday message stays short, and the research reads in full on a screen built for
  it.
- Good, because a failed read is marked in chat and rendered as a failure on the screen, never as a
  clean result.
- Good, because no instrument's rendering exists twice.
- Bad, because the research is one tap further away than in the predecessor's message.
- Bad, because the report is no longer the predecessor's text past the holdout's readout: its golden
  covers the message with every instrument block empty, and the line and the buttons are new.

### Confirmation

SPEC-101's A14, A15 and A20.

## What would make this wrong

- The owner wants the research in the chat message itself: the texts would then be ported with their
  goldens, one per instrument, and the line would give way to them.
- The instruments stop storing their reports: the report would then have nothing to name.

## More Information

Cites ADR-094 (the stored reports), ADR-085 (charts on the client), ADR-041 (the router core),
ADR-012 (the parity oracle), SPEC-094 R7, R9 and R13, and SPEC-041. SPEC-101 builds it.
