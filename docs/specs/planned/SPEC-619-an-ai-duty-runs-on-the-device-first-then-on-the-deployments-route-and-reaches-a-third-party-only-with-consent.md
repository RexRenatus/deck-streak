# SPEC-619: an AI duty runs on the learner's device first, then on the deployment's own route, and reaches a third-party service only with the learner's consent

- **Campaign:** the AI layout. **Issue:** #928 (with #65, #52, #53, #780 and #840).
  **Context(s):** `deck-streak-agent`, `deck-streak-web-engine`, `deck-streak-ffi` and the study
  clients (`web/app`, `ios/`), as `docs/CONTEXT-MAP.md` names them. The build's file manifest also
  names `deck-streak-coordination` and `deck-streak-api`: the use case and the API route by which a
  study client reaches the deployment route.
- **Decided by:** ADR-641, with ADR-054, ADR-301 and ADR-392.
- **Schematic:** `docs/schematics/ai-routes-and-consent.md`.
- **Mutation band:** `S61900-S61999`.
- **Status:** proposed, waiting for the owner's signature; planned (in `docs/specs/planned/`) until the delivery that builds it moves it to `docs/specs/` with its tests and `docs/red-first/SPEC-619.md` (ADR-016).

## 1. The problem, measured

Every figure below was read at DeckStreak `dev` with the command beside it.

- **What it serves.** The PRD's goal, quoted from `docs/PRD.md`: "An AI agent for the digest's
  coaching and the second-brain duties, fail-closed." The study clients are the web app and the
  iPhone and iPad app; the Mini App and the bot are deprecated (the owner's ruling on the study
  clients). #928 asks that the study clients' AI run on the device first, fall back to the
  deployment's own AI route, and reach a third-party service only with the learner's consent.
- **One AI route, and it is on the service.** The route port `AiRoute` holds two values, `Absent`
  (the default) and `Proxy` (`git show dev:crates/agent/src/route.rs | grep -c -E '^    (Absent|Proxy),$'`
  reads 2), chosen by the service's setting `DECKSTREAK_AI_ROUTE` (ADR-054). No value names a
  route on a device or a route to a third-party service.
- **The study clients hold no AI.** No file under `web/` or `ios/` names an AI route or an output
  gate (`git grep -l -i -E 'AiRoute|ai_route|OutputGate|output_gate|ai-route' dev -- web ios`
  lists none).
- **One output gate, and it runs on the service's host.** The agent context's one `OutputGate`,
  `ProbeGate` (`git show dev:crates/agent/src/gate.rs | grep -c 'impl OutputGate for'` reads 1),
  runs the packs' blocking classes as subprocesses of the service, outside the model (SPEC-043 R11,
  ADR-043). Nothing on a device can run it as it stands.
- **Two declared AI tasks.** `ai-safety.json` declares two tasks, the daily reading for law and for
  languages (`git show dev:ai-safety.json | grep -c -E '"id": '` reads 2), with 20 gate entries
  between them (`git show dev:ai-safety.json | grep -c -E '"(ai-content-safety|persona-core|law-professors):'`
  reads 20), and each withholds an output that fails.
- **One deck gate, on the service.** `DutyEngine::decide` (`crates/agent/src/duty.rs`) asks the
  `DeckGate` port after the route check and before the input gate, `compose` and the runner
  (SPEC-381 R4, ADR-392). A route on a device would not pass it.
- **Card text may reach no other model today.** ADR-301 (f)'s first bullet sends card text to a
  model only through the deployment's one AI route, and no class, script or generator sends it to
  any other model or service. A device route and a third-party route each need that clause amended,
  which ADR-641 proposes.

## 2. Requirements

R1. **The route order** (ADR-641 D1). No route runs while AI is off. A duty the service starts runs
    only when the deployment's route is configured (ADR-054 and CHARTER 16, unchanged); a duty a
    study client starts runs only after the learner turned AI on in that client, a setting that is
    off by default. With AI on, a duty's request tries the routes its declaration names (R3), in
    this order, and takes the first that answers: (1) the device route, a model that runs on the
    learner's own device inside the study client, so the request's text never leaves the device;
    (2) the deployment route, ADR-054's route port (today `Proxy`), reached through DeckStreak's own
    service; (3) the third-party route, a third-party service reached from the study client, taken
    only under R2. A route that is absent, unavailable or fails to answer is passed over for the
    next route the learner allows; an output the gate refuses is not a failure of its route (R5).
    A duty with no device in reach, one the service starts on its own or one whose input lives only
    on the service, such as the vault, starts at the deployment route and never takes the
    third-party route.

R2. **Consent for the third-party route** (ADR-641 D2). The third-party route is taken only with the
    learner's consent, in one of two forms: per use, where the client asks before each request,
    saying what the request carries and that it goes to a third-party service, and the answer
    covers that request alone; or a standing setting the learner turns on in the client, off by
    default. Either can be withdrawn: a declined or unanswered ask sends nothing, and turning the
    setting off holds from the next request. Consent is never inferred from turning AI on, from
    another setting or from an earlier answer.

R3. **The duties and their declarations** (ADR-641 D3). Each duty this layout covers declares, in
    one file the service and both study clients read, who starts it, where its input lives, the
    routes it may take in order, and its deterministic path, as the table below states. A duty that
    declares no route or no deterministic path is refused.

| duty | who starts it | where its input lives | routes it may take, in order | its deterministic path |
|---|---|---|---|---|
| cards drafted from the learner's notes (#65) | the owner, from a study client | the vault, on the service only | the deployment route | no card is drafted: the candidates are the note's own flashcard lines (SPEC-150), and the screen says drafting was unavailable |
| an explanation of a missed card | the learner, in a study client, on a card just missed | the card, on the device | the device route, the deployment route, the third-party route | the card's own answer stays shown, with a line saying no explanation is available |
| the persona duties (SPEC-044, ADR-343) | the service on its own, such as the daily reading, or the learner in a study client | the filled persona from the private roster and its subject's memory, on the service only (CHARTER 18) | the deployment route | no persona text and no stand-in: the surface says the text is unavailable |
| practice questions (#52) | the learner, in a study client | the persona from the private roster and, for a bar set, its subject's memory, on the service only | the deployment route | no set is made: the screen says a set could not be made, and a stored set can still be completed |
| the digest's coaching (#53) | the service on its own | the day's stats, on the service only | the deployment route | the digest goes out in its deterministic form with a line saying coaching was unavailable |

Grading of typed answers, which #928 also lists, is deferred until the owner decides it: this SPEC
holds no requirement, criterion or design for it.

R4. **One gate on every route** (ADR-641 D4). The same gate, one definition of CHARTER 17's
    blocking classes (the classes `ai-safety.json` names for the duty's task), runs on the output of
    every route, the device route included, before any text is shown or delivered. The device's
    class list equals the service's: no route drops, adds or weakens a class.

R5. **An output that fails the gate is withheld** (ADR-641 D4). On every route, an output the gate
    refuses is withheld: none of its text is shown, stored or passed on, and the duty records the
    failing class (the service in `agent_runs`, a study client in its own record of AI runs, which
    holds no text). The request does not move to the next route: a refusal is not an outage, and
    moving on would carry the same text further from the device.

R6. **The mark and its provenance** (ADR-641 D6). Every AI output that is shown or delivered carries
    the AI-assisted mark of #840 and its provenance: the route that made it (the device route, the
    deployment route or the third-party route) and the duty. A card made with AI assistance carries
    #840's `ai-assisted` tag. An output with no provenance is not shown. No shown or delivered text
    names a model or a maker (#840).

R7. **The deterministic fallback, said in plain words** (ADR-641 D5). With AI on, when no route
    answers (none allowed, or every allowed route absent or failed), or the gate withholds the
    output, the duty takes the deterministic path R3 declares for it and says so in plain words, as
    the digest already does (CHARTER 16, #53). A silent skip is a defect. With AI off, a duty
    records `ai_route_absent` and shows what ADR-054 shows, unchanged.

R8. **A deck kept away from AI is read by no route** (ADR-641 D7; #780, SPEC-381, ADR-392). Before
    any card text is given to a model, the device route and the third-party route read the same
    marks the service's deck gate reads, through SPEC-381 R7's API routes, and judge each card by
    the same rule (SPEC-381 R2); when the marks cannot be read, no route runs and the duty says so.
    ADR-392's deck gate stays in front of the deployment route, unchanged.

R9. **No card text leaves the device without the learner's consent** (ADR-641 D8). The deployment
    route carries a study client's card text only after the learner turned that route on in that
    client (a setting, off by default, withdrawable), and the third-party route carries it only
    under R2. The sync's sealed bodies (ADR-602) are not AI requests and are unchanged.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | one request through a fake of each route, each set absent, failing or answering, over every combination the test enumerates (its examined count refused at zero), tries the routes R3 declares in R1's order, and the test asserts the route tried at each step and the route taken; a duty with no device in reach starts at the deployment route and never tries the third-party route (R1) | `pnpm exec vitest run web/app/src/lib/ai/routes.test.ts -t "tries the routes in order and takes the first that answers"` |
| A2 | with the setting off and no per-use answer no third-party request is made; a per-use consent covers one request; turning the setting off stops the next request; turning AI on alone grants nothing (R2) | `pnpm exec vitest run web/app/src/lib/ai/consent.test.ts -t "takes the third-party route only with a consent the learner can withdraw"` |
| A3 | a census over the duties' declarations refuses a duty that declares no route or no deterministic path, with a planted duty that lacks each (R3) | `python3 -m unittest discover -s scripts/tests -p test_ai_routes_declared.py -k test_every_duty_declares_its_routes_and_its_deterministic_path` |
| A4 | a census that every route's output reaches a screen or a delivery only through the one gate, and that the device's class list equals the service's; a planted extra or missing class reds it (R4) | `python3 -m unittest discover -s scripts/tests -p test_ai_one_gate.py -k test_every_route_reaches_a_screen_only_through_the_one_gate` |
| A5 | #928's first named test: a planted failing output on each of the three routes is withheld (nothing shown, stored or passed on), no other route is tried with it, and its failing class is recorded (R5) | `pnpm exec vitest run web/app/src/lib/ai/withheld.test.ts -t "withholds an output that fails the gate on every route"` |
| A6 | every shown or delivered output carries the mark, its route and its duty; a planted output with no provenance is refused; no shown or delivered text names a model or a maker (R6) | `pnpm exec vitest run web/app/src/lib/ai/provenance.test.ts -t "shows every output with the mark its route and its duty"` |
| A7 | for each duty, with no route answering, the duty takes its deterministic path and shows the line that says so; no duty skips silently (R7) | `cargo test -p deck-streak-agent --test fallback -- --exact every_declared_duty_takes_its_deterministic_path_and_says_so` |
| A8 | a marked deck's card text reaches none of the three routes, and unreadable marks stop every route with the duty saying so (R8) | `pnpm exec vitest run web/app/src/lib/ai/kept-away.test.ts -t "reads no card of a deck kept away from AI on any route"` |
| A9 | #928's second named test: with consent absent, a recording transport sees zero AI requests carrying card text leave the device on any route past the device; with consent, only the consented route's request does (R9) | `pnpm exec vitest run web/app/src/lib/ai/card-text-egress.test.ts -t "sends no card text past the device without consent"` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/ai/routes.test.ts -t "tries the routes in order and takes the first that answers"
A2: pnpm exec vitest run web/app/src/lib/ai/consent.test.ts -t "takes the third-party route only with a consent the learner can withdraw"
A3: python3 -m unittest discover -s scripts/tests -p test_ai_routes_declared.py -k test_every_duty_declares_its_routes_and_its_deterministic_path
A4: python3 -m unittest discover -s scripts/tests -p test_ai_one_gate.py -k test_every_route_reaches_a_screen_only_through_the_one_gate
A5: pnpm exec vitest run web/app/src/lib/ai/withheld.test.ts -t "withholds an output that fails the gate on every route"
A6: pnpm exec vitest run web/app/src/lib/ai/provenance.test.ts -t "shows every output with the mark its route and its duty"
A7: cargo test -p deck-streak-agent --test fallback -- --exact every_declared_duty_takes_its_deterministic_path_and_says_so
A8: pnpm exec vitest run web/app/src/lib/ai/kept-away.test.ts -t "reads no card of a deck kept away from AI on any route"
A9: pnpm exec vitest run web/app/src/lib/ai/card-text-egress.test.ts -t "sends no card text past the device without consent"
```

Every line names a test the build writes first, red for its criterion's reason, and records in
`docs/red-first/SPEC-619.md`. None exists in this planned SPEC's pull request, which adds no code.

## 3a. What only a device proves

| id | criterion | who proves it, and the record |
|---|---|---|
| V1 | on the owner's own device, with AI on and only the device route allowed, an explanation of a missed card makes no network request that carries the card's text | the owner, from the device's own network log during one explanation, at the release that ships the build; recorded as an evidence comment on #928 |

## 4. File manifest

The build's paths. This planned SPEC's own pull request adds only this file, its schematic, ADR-641
and its changelog fragment.

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-619-an-ai-duty-runs-on-the-device-first-then-on-the-deployments-route-and-reaches-a-third-party-only-with-consent.md` | docs | moved from the planned folder by the build, with its tests (ADR-016) |
| `docs/red-first/SPEC-619.md` | docs | added: the red-first record of A1 to A9 |
| `docs/decisions/ADR-641-ai-runs-on-the-device-first-the-deployments-one-route-next-and-a-third-party-only-with-the-learners-consent.md` | docs | changed: its status, after the owner's signature |
| `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md` | docs | changed: one insert-only pointer note under (f), naming ADR-641 |
| `docs/LEXICON.md` | docs | changed: the rows of section 8 |
| `privacy.json` | repo | changed: the AI settings, a study client's record of AI runs, and the third-party route's recipient |
| `PRIVACY.md` | repo | changed: the same, in words |
| `ai-safety.json` | repo | changed: the explanation's task, its prompt, its golden output and its gate |
| `agent/ai-routes.json` | `deck-streak-agent` | added: one declaration per duty of R3 |
| `agent/prompts/explain-missed-card.prompt.md` | `deck-streak-agent` | added: the explanation's prompt, the card fenced as data |
| `agent/golden/explain-missed-card/missed-card.output.md` | `deck-streak-agent` | added: the explanation's golden output |
| `crates/agent/src/declared_routes.rs` | `deck-streak-agent` | added: reads the declarations, and refuses at start a duty with no device in reach that names another route |
| `crates/agent/src/runs.rs` | `deck-streak-agent` | changed: a run's record carries its route |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the new module |
| `crates/agent/tests/fallback.rs` | `deck-streak-agent` | added: A7 |
| `migrations/061901_agent_runs_route.sql` | `deck-streak-agent` | added: the route of each run |
| `crates/coordination/src/explain.rs` | `deck-streak-coordination` | added: the explanation's use case on the deployment route |
| `crates/api/src/ai_routes.rs` | `deck-streak-api` | added: the API route a study client reaches the deployment route by, behind the owner's session |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: that API route is mounted |
| `crates/web-engine/src/ai.rs` | `deck-streak-web-engine` | added: the device route's port in the web client's Worker, and its gate call |
| `crates/ffi/src/ai.rs` | `deck-streak-ffi` | added: the device route's port on the native transport, and its gate call |
| `crates/ffi/src/allow_list.rs` | `deck-streak-ffi` | changed: the allow-list names the device route's calls |
| `web/app/src/lib/ai/routes.ts` | miniapp | added: the route order (R1, R5, R7) |
| `web/app/src/lib/ai/consent.ts` | miniapp | added: the AI settings and the per-use ask (R2, R9) |
| `web/app/src/lib/ai/provenance.ts` | miniapp | added: the mark and its provenance on a shown output (R6) |
| `web/app/src/routes/study/ai/+page.svelte` | miniapp | added: the AI settings screen |
| `web/app/src/lib/ai/routes.test.ts` | miniapp | added: A1 |
| `web/app/src/lib/ai/consent.test.ts` | miniapp | added: A2 |
| `web/app/src/lib/ai/withheld.test.ts` | miniapp | added: A5 |
| `web/app/src/lib/ai/provenance.test.ts` | miniapp | added: A6 |
| `web/app/src/lib/ai/kept-away.test.ts` | miniapp | added: A8 |
| `web/app/src/lib/ai/card-text-egress.test.ts` | miniapp | added: A9 |
| `ios/App/Sources/AiRoutes.swift` | ios-harness | added: the route order, the consent and the provenance line |
| `ios/App/Sources/AiSettingsView.swift` | ios-harness | added: the AI settings screen |
| `ios/AppTests/AiRoutesTests.swift` | ios-harness | added: the twins of A1, A2, A5, A6, A8 and A9 |
| `scripts/tests/test_ai_routes_declared.py` | scripts | added: A3 |
| `scripts/tests/test_ai_one_gate.py` | scripts | added: A4 |
| `scripts/mutation-rows.d/S61900-S61999.json` | scripts | added: the band's rows |
| `changelog.d/ai-routes-built-619.md` | repo | added: the build's changelog fragment |

## 5. What this does NOT do

- Grading of typed answers is deferred until the owner decides it. This SPEC holds no
  requirement, criterion or design for it (#928).
- How the gate's classes reach the device is not in this SPEC (#928); R4 states only that the
  device's classes equal the service's.
- Which third-party services a study client may reach, and how a client holds the credential for
  one under CHARTER 15, are not in this SPEC (#928).
- Which model each study client runs is not in this SPEC (#928), and no text here names one.
- The AI-assisted mark's own design, its tag and its words, belongs to #840 and its draft #842;
  this SPEC requires the mark on every route and adds the route and the duty to it (#840).
- A persona duty or a practice set does not run on the device: the filled persona and its subject's
  memory stay on the service (CHARTER 18) (#928, #52).
- The digest's coaching keeps its own planned SPEC (SPEC-115); this SPEC fixes only its route and
  its fallback (#53).
- The vault to Anki flashcard bridge is SPEC-150 to SPEC-152's, planned; this SPEC fixes only the
  routes its drafting may take (#65).
- The native deck switch is not built here; it follows this design (#780).
- No server-side route is added beside the deployment's one AI route, and CHARTER 16 and 17,
  ADR-054's default and ADR-392's gate are unchanged (#928).

## 6. Risks

- **A study client's route order drifts from the declarations.** Detected by A1 and A3, which read
  the one declaration file, and by the iPhone and iPad app's twin of A1.
- **The device's gate drifts from the service's classes.** Detected by A4, whose planted extra and
  missing class each red it.
- **A refusal is read as an outage and the text moves to the next route.** Detected by A5 on each
  of the three routes.
- **Card text leaves the device on a route the learner did not allow.** Detected by A9's recording
  transport, and on a real device by V1.
- **A consent is withdrawn while a request is in flight.** The request already sent is not
  recalled; the withdrawal holds from the next request (R2). Detected by A2's withdrawal case; the
  build's formal surface (section 9) names it.
- **A client judges a kept-away deck from marks it read earlier.** R8 reads the marks before each
  request, and unreadable marks stop every route. Detected by A8.
- **A fallback goes silent.** Detected by A7, which examines every declared duty.
- **The third-party route's recipient is missing from the privacy policy.** Detected by the
  privacy-gdpr pack's policy row on the box run, over the build's `privacy.json` and `PRIVACY.md`.
- **A shown text names a model or a maker.** Detected by A6.

## 7. Delivered by the next pull requests

| item | delivered by |
|---|---|
| the build of A1 to A9, red first, with `docs/red-first/SPEC-619.md` | the build of this SPEC, after the owner's signature (#928) |
| the routes in the agent context and in both study clients, with the iPhone and iPad app's twins of A1, A2, A5, A6, A8 and A9 | the build of this SPEC (#928) |
| the gate on the device | the build of this SPEC, once the way its classes reach the device is chosen (#928) |
| the vault to Anki flashcard bridge, planned SPEC-150 to SPEC-152, under this layout | #65, closed with an evidence comment (#928) |
| ADR-301's insert-only pointer note naming ADR-641 | the build of this SPEC, after the owner's signature (#928) |
| ADR-641's status after the signature | the build of this SPEC (#928) |
| the glossary rows of section 8 in `docs/LEXICON.md` | the build of this SPEC (#928) |
| the `privacy.json` entries for the AI settings and the consent, a study client's record of AI runs and the third-party route's recipient | the build of this SPEC (#928) |
| the native deck switch, reading the same marks through the same API routes | #780, which follows this design |
| V1's evidence comment | the release that ships the build (#928) |

## 8. Words

| term | context | meaning |
|---|---|---|
| device route | agent | a model on the learner's own device, inside the study client; its request text never leaves the device |
| deployment route | agent | the deployment's one AI route, ADR-054's route port, reached through DeckStreak's own service; the only route on the server side |
| third-party route | agent | a third-party service a study client reaches, only with the learner's consent |
| consent | agent | the learner's per-use answer or standing setting for the third-party route; never inferred, always withdrawable |
| provenance | agent | the route that made an AI output and the duty it served, carried with the mark on every shown or delivered output |

In this SPEC a route is one of these three; an HTTP route is always called an API route.

## 9. Formal

The build's formal surface is the consent's withdrawal against a request in flight (a withdrawal
holds from the next request, and a request already sent is the last one the consent covers), and
the route order as a state machine: a route tried, passed over or taken, an output withheld, and
the fallback, where a withheld output never reaches a later route. Whether a model is owed for
either is left to the build under ADR-295; this planned SPEC writes none.
