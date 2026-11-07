---
status: "accepted"
---

# ADR-650: the engine suite, its harness and its two configurations join the one type-check project by name

Decides SPEC-628.

## Context and Problem Statement

The web app's type check, `pnpm --dir web/app check`, runs svelte-check over one project,
`web/app/tsconfig.json`, and diagnoses only the files its `include` list names. The list names the
app's own sources and configurations, the card suite with its two configurations (SPEC-341) and
the study suite with its two (SPEC-350), but no file of the engine suite (SPEC-338 R9): its two
specs under `web/app/tests-engine/`, its harness page's script `web/app/engine-harness/main.ts`,
and its Vite and Playwright configurations. Playwright strips types without checking them, so a
type error in any of those five files failed no check: at the base, a type error planted in each
of the five read 0 errors in the check. How does the engine suite join the type check, and what
keeps the next suite from being left out the same way?

## Decision Drivers

- One project and one check script, as the card and study suites already have.
- Coverage that names what it covers, so a reader of the list sees each suite.
- A class closed by a test: the next top-level suite, harness or configuration left out of the
  project reads red by name.
- No behaviour change in a type-check delivery.

## Considered Options (the alternatives each was chosen against)

- Chosen: the four entries join the one project by name, under one comment line, beside the card
  and study suites' entries, because the project already reads two suites this way and one check
  then covers all three.
- A second project for the engine suite with its own run: rejected because it adds a second
  project and a check-script change, while one project already reads two suites.
- Globs such as `tests-*/**/*.ts` or `*.config.ts`: rejected because they are coverage that names
  nothing, and they would also take any later top-level scratch file into the project.
- A type check inside the `web-engine` job before Playwright: rejected because it runs only where
  that job runs, after its build, while the `web` job's check runs on every pull request.
- Only the two entries the gap was first stated by, the suite and its Playwright configuration:
  rejected because the harness page and the Vite configuration the suite runs on would stay
  unread, unlike the card and study suites' configurations.
- Runtime fixes for a diagnostic the widening finds: rejected because a behaviour change does not
  belong in a type-check delivery.

## Decision Outcome

Chosen: the engine suite joins the one project by name, and a census holds the class.

- **D1.** The four entries `tests-engine/**/*.ts`, `engine-harness/**/*.ts`,
  `vite.engine.config.ts` and `playwright.engine.config.ts` join the `include` list of
  `web/app/tsconfig.json` after the card suite's `policy-header.ts`, under one comment line that
  cites SPEC-628, beside the card and study suites' entries.
- **D2.** A census test, `web/app/src/lib/type-check-project.test.ts`, reads the `include` list by
  TypeScript's own configuration parser and the top level of `web/app`, and names, sorted, every
  suite, harness and configuration the list omits (SPEC-628 R3); it prints the count it examined
  and refuses zero.
- **D3.** A diagnostic the widening finds is cured by a type-level change alone, each file's
  JavaScript unchanged. At the base the widening found none, so no file of the engine suite
  changes.

### Consequences

- Good: a type error in the engine suite, its harness page or its configurations fails the `web`
  job's check before any browser test runs.
- Good: a new top-level suite, harness or configuration reads A1 red by name until it joins the
  project.
- Bad: an open change to a file of the engine suite must now type-check when it lands.
- Neutral: the check reads five more files.

### Confirmation

SPEC-628's A1 and A2, run in the `web` job, and the job's check over the widened project reading
0 errors and 0 warnings.

## What would make this wrong

- A suite placed below the top level of `web/app`, or named outside R3's forms: the census does
  not select it, and the census would need a new form, by its own SPEC.
- A top-level script that is no browser suite, harness or configuration but matches R3's file
  form: the census asks for it, and it joins the project or the form narrows, by its own SPEC.

## More Information

- SPEC-338 (the engine suite and its harness page), SPEC-341 (the card suite's entries in the
  project) and SPEC-350 (the study suite's entries).
- The schematic `docs/schematics/web-type-check-project.md`.
