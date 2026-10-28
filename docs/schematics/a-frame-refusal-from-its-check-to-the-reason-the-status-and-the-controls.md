# A frame refusal, from its check to the reason, the status and the controls

SPEC-505, ADR-523. How a card the frame refuses reaches the learner as a reason, on the web and on iOS.

## Web: data flow

```mermaid
flowchart TD
  face["the review's face: html, css, side, ordinal<br/>review.ts"] --> card["frameCard(html, css, title, classes)<br/>frame-host.ts"]
  card --> doc["frameDocument<br/>frame-document.ts"]
  doc --> c1{"class list admitted?"}
  c1 -- no --> r1["class-list"]
  c1 -- yes --> parse["compose, then re-parse as the frame will"]
  parse --> c2{"head is the policy and the style?"}
  c2 -- no --> r2["style-closed"]
  c2 -- yes --> c3{"no link, meta, base or template back?"}
  c3 -- no --> r3["reparse-element"]
  c3 -- yes --> c4{"no srcset back?"}
  c4 -- no --> r4["reparse-srcset"]
  c4 -- yes --> c5{"no target back?"}
  c5 -- no --> r5["reparse-target"]
  c5 -- yes --> c6{"body attributes are the class written?"}
  c6 -- no --> r6["body-attributes"]
  c6 -- yes --> host["frameHost: compose the host, re-parse"]
  host --> c7{"head, frame document and title read back?"}
  c7 -- no --> r7["host-readback"]
  c7 -- yes --> shown["srcdoc: the card shows"]
  r1 & r2 & r3 & r4 & r5 & r6 & r7 --> reason["RefusalReason (REFUSAL_REASONS)"]
  reason --> mark["CardFrame.svelte: empty frame, data-card-refused = reason"]
  reason --> status["review.status = reason<br/>refusal.ts: study_card_refused_reason"]
  status --> words["role=status: first sentence, reason, next step, controls kept<br/>messages/7 locales"]
  reason --> controls["controls: answer, bury, flag stay"]
```

The first failing check names the reason; every later check is not read. No branch carries any part of the card past
`frameCard`: the mark, the status and the words are each a fixed value chosen by the reason.

## iOS: data flow

```mermaid
flowchart TD
  doc["CardFaceView: the face's document"] --> make["CardWebViewFactory.makeCardWebView"]
  make -- view --> show["the card shows; the refusal line is empty"]
  make -- throws --> words["CardRefusalWords.text(of: error)"]
  words --> c{"a CardViewRefusal?"}
  c -- ruleListDidNotCompile --> s1["its DeckStreak sentence"]
  c -- any other error --> s2["the general sentence"]
  s1 & s2 --> line["Text: the refusal line"]
  s1 & s2 --> announce["CardRefusalWords.announce: one .announcement"]
```

## Components

| component | file | holds |
|---|---|---|
| the checks | `web/app/src/lib/card/frame-document.ts` | `REFUSAL_REASONS`, `RefusalReason`, six checks in order |
| the host | `web/app/src/lib/card/frame-host.ts` | the seventh check, `frameCard` |
| the frame | `web/app/src/lib/card/CardFrame.svelte` | the mark |
| the status | `web/app/src/lib/study/review.ts`, `refusal.ts` | the reason as the status, its key |
| the words | `web/app/messages/*.json` | seven messages per locale |
| iOS words | `ios/App/Sources/CardRefusalWords.swift` | each case's sentence, the announcement |
| iOS face | `ios/App/Sources/CardFaceView.swift` | shows and announces the words |
