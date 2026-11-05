---
status: "proposed"
decision-makers: "the DeckStreak architect, for the signature of @RexRenatus (owner)"
---

# ADR-641: AI runs on the learner's device first, the deployment's one route next, and a third-party service only with the learner's consent

Decides SPEC-619 (#928): the order in which an AI duty's request tries its routes, and the consent
rule that keeps card text on the learner's device unless the learner chose otherwise. It amends
ADR-301 (f). It is the DeckStreak architect's proposal and takes effect only when the owner signs
it; until then ADR-301 and every accepted decision read as they do today.

## Context and Problem Statement

DeckStreak's AI runs on the service today. The route port `AiRoute` holds `Absent`, the default,
and `Proxy`, the deployment's one AI route (ADR-054); the output gate runs the packs' blocking
classes as subprocesses on the service's host (ADR-043); and the deck gate stands before the runner
(ADR-392). ADR-301 (f) sends card text to a model only through that one route, and ADR-015 rejected
a server-side path to a third-party model. The study clients, the web app and the iPhone and iPad
app, hold no AI of their own.

#928 asks for the study clients' AI to run on the learner's device first, fall back to the
deployment's own AI route, and reach a third-party service only with the learner's consent, with
one gate on every route and the AI-assisted mark of #840 on every output. Which route does a
request take, in what order, and what consent must stand before card text leaves the device?

## Decision Drivers

- #928's order: the device first, the deployment's route next, a third party only with consent.
- The learner is private (CHARTER 11): a card's text leaves the device only when the learner chose
  that.
- No AI is the default and a first-class path (CHARTER 16, ADR-054): no route runs before AI is on.
- Every AI output passes the packs' blocking classes before delivery (CHARTER 17), on every route.
- A deck the learner keeps away from AI stays away on every route (ADR-392, #780).
- No secret on disk or in a repository (CHARTER 15): the deployment holds no credential for a
  third-party service.
- No shown text names a model or a maker (#840).

## Considered Options (the alternatives it was chosen against)

- Chosen: the device route first, the deployment's one route next, and a third-party service only with the learner's consent, because a card's text stays on the device whenever the device can answer, and every step past it is the learner's own choice under one gate.
- The server route only, as today, with ADR-301 (f) unchanged: rejected because #928 asks for the device first and a consented third party, and with the deployment's route the only one, every AI request sends card text off the device.
- A third-party service by default: rejected because card text would leave the device to a recipient the learner never chose, which CHARTER 11 and the no-AI default of ADR-054 both refuse.
- On-device only: rejected because a duty whose input lives on the service, such as the vault, the private roster or the digest's stats, has no device in reach, and a device that runs no model would lose AI altogether.
- The deployment's route first and the device as its fallback: rejected because card text would leave the device whenever the service answers, even when the device could have answered.
- The third-party service reached through DeckStreak's service: rejected because it would add a second server-side route beside the one ADR-301 (f) keeps, and the deployment would hold a credential for a third party (CHARTER 15).
- Consent inferred from turning AI on: rejected because turning AI on says nothing about a third-party recipient, and an inferred consent cannot be withdrawn as an act of its own.

## Decision Outcome

Chosen option: the device route first, the deployment's one route next, and a third-party service
only with the learner's consent, because it keeps card text on the device whenever the device can
answer, adds no server-side route, and makes each step past the device the learner's own choice.

### D1. The route order

No route runs while AI is off. A duty the service starts runs only when the deployment's route is
configured, as ADR-054 has it; a duty a study client starts runs only after the learner turned AI
on in that client, a setting that is off by default. With AI on, a duty's request tries the routes
in this order and takes the first that answers:

1. the device route, a model that runs on the learner's own device inside the study client, so the
   request's text never leaves the device;
2. the deployment route, the deployment's own one AI route (ADR-054's route port, today `Proxy`),
   reached through DeckStreak's own service;
3. the third-party route, a third-party service reached from the study client, taken only under D2.

A route that is absent, unavailable or fails is passed over for the next route the learner allows.
A duty with no device in reach, one the service starts on its own or one whose input lives only on
the service, such as the vault, starts at the deployment route and never takes the third-party
route.

### D2. Consent for the third-party route

The third-party route is taken only with the learner's consent, in one of two forms: per use, where
the client asks before each request, saying what the request carries and that it goes to a
third-party service, and the consent covers that request alone; or a standing setting the learner
turns on in the client, off by default. Either can be withdrawn: a declined or unanswered ask sends
nothing, and turning the setting off holds from the next request. Consent is never inferred from
turning AI on, from another setting or from an earlier answer.

### D8. Card text and the device

No card text leaves the device for an AI request without the learner's consent. The deployment route
carries a study client's card text only after the learner turned AI on for that route in that
client, a setting that is off by default and can be withdrawn; the third-party route carries it only
under D2. The sync's sealed bodies (ADR-602) are not AI requests and are unchanged.

### The rules SPEC-619 states

These follow from D1, D2 and D8, and SPEC-619 states them as requirements; this ADR does not decide
them again.

- D3, the duties: cards drafted from the learner's notes (#65), an explanation of a missed card, the
  persona duties (SPEC-044, ADR-343), practice questions (#52) and the digest's coaching (#53), each
  declaring who starts it, where its input lives, its routes in order and its deterministic path.
  Grading of typed answers is deferred until the owner decides it.
- D4, one output gate: the same gate, one definition of CHARTER 17's blocking classes, on the output
  of every route, the device included; a refused output is withheld and does not move to the next
  route.
- D5, the deterministic fallback: when no route answers, or the gate withholds the output, the duty
  takes its deterministic path and says so in plain words; a silent skip is a defect.
- D6, the mark and its provenance: every shown or delivered output carries #840's mark, the route
  that made it and the duty, and names no model or maker.
- D7, a deck kept away from AI: no route reads it; the device route and the third-party route read
  the same marks the service's deck gate reads, and ADR-392's deck gate stays in front of the
  deployment route.

### The amendment of ADR-301 (f)

ADR-301's own file is not edited by this ADR's pull request.

What changes. The first bullet of (f) reads today: "Card text reaches a model only through the
deployment's one AI route (CHARTER 16, ADR-054), the owner's subscription proxy. No class, script or
generator sends card text to any other model or service." With this amendment it reads: card text
reaches a model only on the learner's own device, where it does not leave the device, through the
deployment's one AI route, or, from a study client, through a third-party service under this ADR's
consent rule (D2 and D8).

What stays.

- The deployment's one AI route stays the only server-side route: no server-side class, script or
  generator sends card text to any other model or service.
- The other three bullets of (f): derived data under data rights, card text that leaves for another
  store needing its own ADR, and the persona review under CHARTER 18.
- ADR-301's parts (a) to (e) and (g).
- CHARTER 16 and 17 as written, ADR-054's no-AI default and ADR-392's deck gate.

When. The amendment takes effect only when the owner signs this ADR. ADR-301's insert-only pointer
note naming this ADR, and this ADR's status change, follow the signature, in the build of SPEC-619.

### Consequences

- Good, because a card's text stays on the device whenever the device can answer.
- Good, because one gate holds every route, so a third-party output meets the same classes as the
  deployment's.
- Good, because the default stays no AI, and each step past the device is a setting or an answer
  the learner gives and can take back.
- Good, because the server side keeps its one route, so ADR-015 and ADR-301 (f)'s server-side rule
  stand.
- Bad, because the gate must run on the device, and how its classes reach the device is not yet
  chosen (SPEC-619 section 5).
- Bad, because both study clients carry the route order, held to one declaration file by tests.
- Bad, because a third-party service is a new recipient of card text, which the privacy policy must
  name before the route can be taken.
- Bad, because the device's answer is taken when it passes the gate, even where the deployment's
  route might answer better.

### Confirmation

SPEC-619's criteria A1 to A9, each red first in its build, and its device-only criterion V1; among
them, a test that an output which fails the gate is withheld on every route, and a test that no card
text leaves the device without the learner's consent. Nothing is built before the owner signs.

## What would make this wrong

- No study device can run a model the owner accepts: the device route stays absent, and the layout
  is the deployment's route and the consented third party.
- The gate's classes cannot run on a device: the device route and the third-party route cannot meet
  D4, and this ADR is amended before either is taken.
- The owner wants no third-party route: D2 is withdrawn by an amendment, and D1 keeps two routes.
- The owner decides how typed answers are graded: SPEC-619 gains that duty by an amendment, under
  this order.

## More Information

#928, #65, #52, #53, #780, #840 and #842; CHARTER 11, 15, 16 and 17; ADR-015, ADR-043, ADR-054,
ADR-068, ADR-301, ADR-392 and ADR-602; SPEC-043, SPEC-044, SPEC-381 and SPEC-619;
`docs/schematics/ai-routes-and-consent.md`.
