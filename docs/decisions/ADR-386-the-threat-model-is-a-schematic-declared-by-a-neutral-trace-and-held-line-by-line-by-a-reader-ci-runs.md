---
status: "accepted"
---

# ADR-386: the threat model is a schematic declared by a neutral trace and held line by line by a reader CI runs

Decides SPEC-375.

## Context and Problem Statement

#653 asks for the app campaign's threat model: a schematic that declares the threat-model trace
and holds a STRIDE table for each surface, which the security checks read. At dev the campaign's
five surfaces each have controls for all six STRIDE classes and tests that pin them, but no
schematic declares anything and no reader exists. SPEC-134 (planned, #60) names a reader's path
and a model found by its file name, and ADR-134 refused carrying a foreign check's row identifier
in public text.

The decisions below settle which surfaces the model names, how it is declared and shaped, what
public text it may hold, how it is read and tested, how the delivery is shaped and where its tests
run, and why no formal model is owed.

## Decision Drivers

- The model is public text: threats by class and asset, and the controls that answer them, never
  an unmitigated weakness.
- A citation must fail loudly when the line it names moves, in the delivery that moved it.
- No foreign identifier enters public text (ADR-134).
- The repository's text checks are Python modules under `scripts/tests`, which CI's `hygiene` job
  already runs.
- The smallest change: no new CI job, and no crate, app or workflow change.

## Considered Options (the alternatives each decision was chosen against)

### D1. The surfaces

- Chosen: five surfaces, each named from its entry at dev, because each is where input the campaign does not control reaches what it runs: the iPhone and iPad client; the web client (its page, card frame, Worker and the route that releases the sealing key); the sync service; the engine and its boundaries; and the build and release lanes.
- Rejected: one table for the whole campaign, because a class answered on one surface would hide its absence on another.
- Rejected: a surface per crate or build target, because the threat boundaries are the client, the service, the engine's boundary and the lanes, not the build units.
- Rejected: modelling the surfaces not yet wired or built (#640, #627) now, because their rows would cite nothing that runs.

### D2. The location and the table

- Chosen: a new schematic whose tables are `id | threat | asset | control | pinned by`, each control and pin a backticked `path:line:quote` or `path:first-last:quote` citation, because a citation that names its line fails in the delivery that moves the line.
- Rejected: a section in an existing schematic, because a reader would then have to tell model text from design text.
- Rejected: quote-only citations, because a quote anywhere in a file can be satisfied by a comment far from the control.
- Rejected: a test name alone as the pin, because a renamed or deleted test would not be seen.

### D3. The reader

- Chosen: `scripts/threat_model.py`, at SPEC-134's planned path, run by `scripts/tests/test_threat_model.py` in CI's existing python stage, because that stage already runs every module under `scripts/tests`.
- Rejected: a new CI job or workflow step, because the python stage already runs every module, and a workflow edit widens the change to the workflows and their own tests.
- Rejected: committed fixture trees (SPEC-134's `scripts/tests/fixtures/threat-model/`), because a committed planted model is a second declared model that any tree walker reads, and a literal plant beside its assertion is read with it.
- Rejected: a vitest or cargo reader, because the model is repository text and the repository's text checks are Python.

### D4. The public-text line

- Chosen: threat classes, assets and controls only, and no threat without a control in any public file, because a published threat with no control is an unmitigated weakness made public.
- Rejected: an "open" row for a threat with no control, because it publishes an unmitigated weakness.
- Rejected: leaving a class out of a surface silently, because SPEC-375 R3 requires all six and the reader refuses the gap.

### D5. The tests, red first

- Chosen: one unittest module, red first against a stub `judge` that returns an empty `Report` and a stub `main` that prints `examined 0 model(s), 0 surface(s), 0 row(s), 0 citation(s)` and returns 3, because every red then fails by an assertion: a missing presence (A1 to A3) or a missing refusal line (A4 to A10).
- Rejected: no stub, because an import error is not a red for the criterion's reason.
- Rejected: an accept-all real-tree assertion first, because an `examined` refusal ahead of the behaviour is red for the wrong reason.

### D6. The delivery's shape

- Chosen: one delivery with exactly two pushes, the reds read in CI on the first and the green, the record and the rows on the second, because the reader's real-tree run reads every schematic in the tree, which this delivery runs only where the gate runs it.
- Rejected: one push carrying reds read on the builder's machine, because the real-tree run would then run outside the gate, and the red-first record quotes the gate's own failures.
- Rejected: two deliveries, the model first and the reader second, because #653 asks for a model the checks read, and a model with no reader is unenforced the day it lands.

### D7. Mutation

- Chosen: CI's generator over every constant, comparison and return of the new script, plus four hand rows the generator cannot write, because a semantic mutant such as a dropped class letter is no operator swap.
- Rejected: hand rows only, because the generator already mutates every operator and constant.
- Rejected: the generator only, because it never writes a semantic mutant such as a dropped class letter.

### D8. Formal

- Chosen: no formal model, because the reader is a single process over committed text, with no second actor, no shared state and no check-then-act on state another actor changes, and the repository's CI has no formal job.
- Rejected: a Lean proof of the completeness rule, because no recorded failure motivates it and the rule is a set comparison a unit test pins exactly.

### D9. The declaration's value

- Chosen: front matter `trace: threat-model`, a neutral value, because the reader CI runs is this repository's own and no foreign identifier enters public text.
- Rejected: an external check's own row identifier as the value, because ADR-134 (lines 27-35) refused a foreign identifier in public text.
- Rejected: no front matter, the model found by its path alone (SPEC-134 R3's `threat-model.md`), because a rename silently undeclares it, #653 asks for a declaration, and a check that reads declarations never finds it.

### D10. Where the tests run

- Chosen: `scripts/tests/test_threat_model.py` and the reader's mutants run in CI only, because the real-tree run reads every schematic in the tree, which this delivery reads only where the gate runs: the `hygiene` job's python stage runs the module, and the `mutation-plan`, `mutation-python`, `mutation-rows` and `mutation-verdict` jobs decide the mutants.
- Rejected: a local run of the module and a local mutation run of the reader, with the reds in one push, because those runs would read every schematic outside the gate.

## Decision Outcome

Chosen: the campaign's threat model is a new schematic declared by `trace: threat-model`, with a
STRIDE table for each of five surfaces whose every control and pin cites a line that holds, read by
`scripts/threat_model.py` in CI's python stage, because it is the smallest change that makes the
model a declaration a check reads, fails a moved citation in the delivery that moved it, and keeps
every foreign identifier and every uncontrolled threat out of public text.

- **D1.** Five surfaces, each from its entry at dev. **Chosen against:** one campaign-wide table, because a per-surface gap would hide behind another surface's row; a surface per crate or target, because the boundaries are not the build units; modelling unwired surfaces now (#640, #627), because a row would cite nothing that runs.
- **D2.** A new schematic, tables `id | threat | asset | control | pinned by`, backticked `path:line:quote` citations. **Chosen against:** a section in an existing schematic, because model text would mix with design text; quote-only citations, because a far-away comment satisfies them; a test name alone as the pin, because a rename goes unseen.
- **D3.** `scripts/threat_model.py`, run by `scripts/tests/test_threat_model.py` in the python stage. **Chosen against:** a new CI job, because the stage already runs every module; committed fixture trees, because a committed plant is a second declared model; a vitest or cargo reader, because the text checks are Python.
- **D4.** Classes, assets and controls only, no threat without a control. **Chosen against:** an "open" row, because it publishes an unmitigated weakness; a silent class gap, because R3 requires all six.
- **D5.** Red first against a stub `judge` and a stub `main` returning 3. **Chosen against:** no stub, because an import error is not a red; an accept-all real-tree assertion first, because an `examined` refusal is red for the wrong reason.
- **D6.** One delivery, exactly two pushes, the reds read in CI. **Chosen against:** one push with reds read on the builder's machine, because the real-tree run would run outside the gate; two deliveries, because a model with no reader is unenforced the day it lands.
- **D7.** The generator plus four hand rows: S37501 (the class alphabet loses R; A7 kills it), S37502 (the quote is sought in the whole file; A4), S37503 (`target` leaves the skipped names; A5) and S37504 (completeness over the model's union, not per surface; A7). **Chosen against:** hand rows only, because the generator already mutates every operator; the generator only, because it writes no semantic mutant.
- **D8.** No formal model. **Chosen against:** a Lean proof of the completeness rule, because no recorded failure motivates it and a unit test pins the set comparison exactly.
- **D9.** `trace: threat-model`. **Chosen against:** an external check's own row identifier, because ADR-134 refused a foreign identifier in public text; no front matter and a path alone, because a rename silently undeclares the model.
- **D10.** The module and the mutants run in CI only. **Chosen against:** a local run and a local mutation run with the reds in one push, because those runs would read every schematic outside the gate.

### Consequences

- Good: an edit that moves a cited line reddens the delivery that made it, with a finding that
  names the row and the citation.
- Good: every surface answers every class, and a gap is a finding rather than a silence.
- Good: the planned whole-product model (SPEC-134, #60) can use the same declaration and reader
  unchanged.
- Bad: a delivery that edits a cited line owes the model a moved citation in the same change.
- Bad: a pin shows that a test exists, not that it asserts the control; review and the pinned
  tests' own mutation rows carry that.
- Bad: a security check outside this repository that looks for its own declaration value does not
  read this model until it accepts the neutral one.
- Neutral: the reds and the green are separate pushes, so the delivery's first CI run is red by
  design.

### Confirmation

The acceptance criteria of SPEC-375: A1 to A3 run the reader over the real tree (the one declared
model, every citation holding over the model's 34 rows, every surface covering all six classes);
A4 to A9 each plant a model and assert its whole finding line, with the corrected plant reading
clean; A10 reads the command's exits. CI's `hygiene` job runs them, and the `mutation-python` and
`mutation-rows` jobs prove that the generator's mutants and rows S37501 to S37504 are killed.

## What would make this wrong

- A security check that must read this model and accepts only its own identifier as the
  declaration: then a declared alias is owed, never a foreign identifier in public text.
- Line drift so frequent that citations cost more than they catch: then stricter definition-line
  citations, or a citation anchored to a symbol, are owed.
- A trust boundary that runs at dev but is absent from both this model and SPEC-375 §5's owned
  exclusions: then a surface section is owed.
- A threat found with no control at dev: it is fixed or owned out of public view before the model
  can name it, since D4 keeps it out of public text.

## More Information

- #653 (the need) and SPEC-341 §5, which leaves it here.
- SPEC-134 and ADR-134: the planned whole-product model (#60), its reader's path, and the refusal
  of a foreign identifier in public text.
- `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md`:
  the model itself.
- SPEC-375 §5: every surface outside this model and the issue that owns it.
