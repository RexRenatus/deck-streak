# SPEC-134: every box-run pack is enforced, each red that #60 owns turns green in the tree or stays a named exception on its own issue

- **Wave:** W7. **Issue:** #60 (every pack enforced) (epic #8). **Context(s):** `repo`
  (`docs/schematics/`, `scripts/`, `scripts/tests/`), `deck-streak-kernel` (one test), the Mini App
  (`web/app`, the page template and its server hook), and the box run's expectations, which are
  private (SPEC-056 R7) and change only through the JSON diff this delivery hands back.
- **Decided by:** ADR-134 (this SPEC's: which of #60's reds turn green in the tree, and which stay
  named exceptions bound to an issue each), ADR-069 (the public tree carries no vendored hub files),
  ADR-056 (the packs stay box-only), ADR-059 (public text describes DeckStreak only), ADR-004 and
  ADR-030 (the box run and its verbs).
- **Prerequisites:** SPEC-030 (the box scan's expectations), SPEC-056 (every pack judged on the
  box), and the deliveries of #29, #32, #42, #49, #52, #53, #57, #58, #59, #102, #257 and #360,
  each of which enforces a pack, a row or a scan this SPEC counts on (R2). #42 has closed, and the
  other eleven are open. **Mutation band:** `S13400-S13499`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-134.md` (ADR-016).

## 1. The problem, measured

Measured at dev `f10a483`, against the public contract of SPEC-056 (R7 to R11) and SPEC-030.

- **Packs that wait.** The box run judges every pack the maintainer's private file names. Eight packs
  are not yet enforced, each waiting on the issue that builds its subject: #29 (two packs), #32
  (two), #49, #52 and #53 (one each), and #61 (one, the v9 import, W8). The subscription-proxy
  client scan and the apiKeyHelper scan read pending on #29, because no settings document exists
  for them to examine.
- **Rows that wait.** Five rows of enforced packs are deferred, one each on #42, #57, #102, #257
  and #360. Two rows of one enforced pack are excluded rather than deferred, because they judge a
  deploy workflow that DeckStreak does not have (ADR-010, ADR-017); an exclusion carries its reason
  and waits on no issue, so this SPEC keeps both.
- **Reds that #60 owns.** Six box-run packs show reds the box run expects on the tree, and these
  name #60 as the issue that settles them:
  - the cyber-pipeline pack finds no threat model, because none is written under
    `docs/schematics/`;
  - the cyber-pipeline and web-security packs judge the served script policy strict, and the
    Caddy block's policy carries no script source by design: the page's own meta policy carries
    them with its build's hashes (SPEC-028 R14), which the pack does not read;
  - the greenfield pack reads the notice markers in the AGPL text's own appendix (`LICENSE`,
    `LICENSES/`), which is verbatim and may not change;
  - the ledger-sqlite pack's five blocking rows run tests that exist only in the pack's own
    repository, which a consuming repository cannot run;
  - one box-run pack examines nothing, because every row it carries is catalog-only;
  - the web-launch pack reads the Mini App's page template for its language, and
    `web/app/src/app.html` carries Paraglide's placeholder (`%paraglide.lang%`) until the page
    renders.
- **DeckStreak's own counterparts.** `crates/kernel/tests/schema.rs` holds that every table has
  `created_at` and is `STRICT`; `crates/kernel/tests/db.rs` holds that two writers serialise with
  no lost update; `crates/ingest/tests/lock.rs` holds that a second sync waits for the collection
  lock. No test holds that an applied migration, once edited, is refused, although
  `crates/kernel/src/db.rs` documents it. No migration creates a trigger, so no DeckStreak table is
  append-only by the schema.

## 2. Requirements

The goal

R1. On the delivery's pushed head, the box run reads `BOX PACKS OK` with every pack enforced except
    the one #61 enforces (W8, ADR-134), no deferred row, no pending scan, and no expected red or
    pending entry that names #60.
R2. The delivery turns enforced each pack, row and scan whose issue has closed when it builds. One
    whose issue is still open at its build stays as it is, the pull request names it with its
    issue, and #60 does not close until it is enforced.

Green in the tree

R3. `docs/schematics/threat-model.md` is a STRIDE threat model of DeckStreak, declared by the front
    matter the cyber-pipeline pack reads, with a table whose header has a `control` column. Each
    row names its threat by a STRIDE id (`S1`, `T1`, `R1`, `I1`, `D1`, `E1` and on) and cites one
    or more backticked `path:line:quote` or `path:first-last:quote` spans. The model covers at
    least: a forged Telegram handshake and a non-owner session (spoofing and elevation: identity's
    `init_data::validate` and `OwnerSession`); a cross-site write (tampering: SPEC-024's CSRF
    bound); an edited applied migration (tampering: R6); a secret in a log line (disclosure:
    the kernel's redaction); a private field on the public page (disclosure: SPEC-137's allow-list,
    when it has landed); a request flood (denial of service: the API's global concurrency limit);
    overlapping syncs (denial of service: ingest's lock); and an action with no record
    (repudiation: the ledgers that record each grant, send and sync). Linked sign-in's refusals
    join when SPEC-131 has landed. Every STRIDE category has at least one row. The table states controls only: each row names the
    control that answers its threat and describes no weakness of a live host.
R4. `scripts/threat_model.py` reads the model as the pack describes it and answers each finding: a
    citation whose path names no file or more than one, one whose cited lines hold no line
    containing its quote, one with no quote, a row with no citation, and a STRIDE category with no
    row. `scripts/tests/test_threat_model.py` runs it over the real model and over planted models,
    so the model's citations stay true between box runs.
R5. `web/app/src/app.html` opens `<html lang="en" dir="ltr">`, the base locale's language and
    direction, and `web/app/src/hooks.server.ts` replaces those two attributes with the rendered
    locale's, as it replaces the placeholders today. `hooks.client.ts` is unchanged.
R6. `crates/kernel/tests/db.rs` gains a test that opens a database whose applied migration's file
    has since changed, over the fixture migrations, and asserts `KernelError::Migrate` naming that
    migration's version, with nothing written.

Named exceptions (ADR-134)

R7. Four reds stay as named exceptions, each bound to one DeckStreak issue filed when the build
    lands (the box run's expectation names it, SPEC-056 R8), each with the pack change that would
    turn it green recorded as feedback to the pack's authors:
    - the threat model's trace, until the cyber-pipeline pack reads a neutral declaration: the
      declaration it reads today is an identifier of the pack's own catalog, which no public file
      carries (R10);
    - the strict script policy, judged by the cyber-pipeline and web-security packs, until they read
      the page's meta policy;
    - the license placeholders, until the greenfield pack skips the AGPL text's own appendix;
    - the ledger-sqlite pack's five blocking rows, until they have forms a consuming repository can
      run. DeckStreak holds four of their properties with its own tests (§1 and R6); the
      append-only census has no subject here (§1).
R8. The pack that examines nothing stays pending on its own issue, filed at build, until it carries
    a row that examines this tree.
R9. The pack #61 enforces stays pending on #61 (W8).

Public text

R10. This SPEC, ADR-134, the pull request and every issue it files name packs, and never a row id, a
     probe path or the private wiring (ADR-059, SPEC-056 R13). A pack that is not yet enforced is
     named by the issue that enforces it.
R11. CHARTER 10's eleven anti-goals bind this SPEC as one block; the one it touches is no dishonest
     copy: an exception is stated as an exception with its issue, never counted as a green.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every control of the model cites a span whose path names one file and whose lines hold its quote | `test_every_control_cites_a_line_that_holds` |
| A2 | every STRIDE category has at least one row | `test_every_stride_category_has_a_control` |
| A3 | a planted citation whose quote moved off its line is refused | `test_a_citation_whose_quote_moved_is_refused` |
| A4 | a planted citation naming no file, or two, is refused | `test_a_citation_naming_no_file_or_two_is_refused` |
| A5 | a planted citation with no quote is refused | `test_a_citation_with_no_quote_is_refused` |
| A6 | a planted model missing one category is refused naming it | `test_a_model_missing_a_category_is_refused` |
| A7 | the page template names the base locale's language and direction | `the page template names the base locale and its direction` |
| A8 | the server hook writes the rendered locale's language and direction | `the server hook writes the rendered locale's lang and dir` |
| A9 | an edited applied migration is refused by its version, and nothing is written | `a_changed_applied_migration_is_refused` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_control_cites_a_line_that_holds
A2: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_stride_category_has_a_control
A3: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_whose_quote_moved_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_naming_no_file_or_two_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_with_no_quote_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_model_missing_a_category_is_refused
A7: pnpm exec vitest run web/app/src/lib/html-lang.test.ts -t "the page template names the base locale and its direction"
A8: pnpm exec vitest run web/app/src/lib/html-lang.test.ts -t "the server hook writes the rendered locale's lang and dir"
A9: cargo test -p deck-streak-kernel --test db -- --exact a_changed_applied_migration_is_refused
```

## 3a. What the box run judges

The private-wiring change that enforces B1 to B3 turns enforced each pack, row and scan whose issue
has closed (R2), removes every expectation that names #60, and binds each named exception to its
issue (R7, R8); the delivery hands it back as a JSON diff and commits none of it.

| id | criterion | decided by |
|---|---|---|
| B1 | over `docs/schematics/`: a declared threat model whose every control is traced, with a row for each STRIDE category | the cyber-pipeline pack |
| B2 | over `web/app/src/app.html`: the page names its language | the web-launch pack |
| B3 | over the whole tree: every pack enforced but the one #61 enforces, no deferred row, and each remaining red bound to an open issue other than #60 | every box-run pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/schematics/threat-model.md` | docs | added: the STRIDE model (R3) |
| `scripts/threat_model.py` | repo | added: the model's reader (R4) |
| `scripts/tests/test_threat_model.py` | repo | added: A1 to A6 |
| `scripts/tests/fixtures/threat-model/` | repo | added: the planted models |
| `web/app/src/app.html` | miniapp | changed: the base locale's `lang` and `dir` |
| `web/app/src/hooks.server.ts` | miniapp | changed: replaces the two attributes |
| `web/app/src/lib/html-lang.test.ts` | miniapp | added: A7, A8 |
| `crates/kernel/tests/db.rs` | `deck-streak-kernel` | changed: A9 |
| `crates/kernel/tests/fixtures/migrations/edited/` | `deck-streak-kernel` | added: the migration applied, then edited |
| `docs/specs/SPEC-134-every-box-run-pack-is-enforced-each-red-that-60-owns-turns-green-in-the-tree-or-stays-a-named-exception-on-its-own-issue.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-134.md` | docs | added |
| `scripts/mutation-rows.d/S13400-S13499.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It changes no pack; each named exception's pack change is the pack authors', and its issue waits
  on it (#60).
- It builds no subject a waiting pack examines; each pack's issue does (#29, #32, #49, #52,
  #53).
- It builds none of the landing page the public-site packs judge (#59), and none of linked sign-in
  (#58).
- It leaves the pack #61 enforces pending until the v9 import (#61).
- It posts no commit status; the box run's `--post-status` stays off by default (#60).

## 6. Risks

- **An exception mistaken for a green.** R7 to R9 bind each to an issue, and B3 fails on one that
  names #60 or a closed issue (SPEC-056 R8's stale expectation).
- **A threat model that drifts from the code.** R4's reader runs in the tree's own tests; detected
  by A1 and A2 on every change.
- **A template attribute the hook no longer finds.** R5; detected by A8.
- **A pack issue that never closes.** R2 keeps #60 open; the pull request names what it waits on.

## 7. Parity goldens

None: nothing here ports the predecessor's behaviour.

## 8. Tables and the v9 import

None.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13401-QUOTE-ON-LINE` | `scripts/threat_model.py` | a citation holds only when a cited line contains its quote | `test_threat_model.ACitationMustHoldInTheTree.test_a_citation_whose_quote_moved_is_refused` |
| `S13402-ONE-FILE` | `scripts/threat_model.py` | a citation's path names exactly one file | `test_threat_model.ACitationMustHoldInTheTree.test_a_citation_naming_no_file_or_two_is_refused` |
| `S13403-QUOTE-REQUIRED` | `scripts/threat_model.py` | a citation with no quote is a finding | `test_threat_model.ACitationMustHoldInTheTree.test_a_citation_with_no_quote_is_refused` |
| `S13404-EVERY-CATEGORY` | `scripts/threat_model.py` | every STRIDE category needs a row | `test_threat_model.TheModelCoversStride.test_a_model_missing_a_category_is_refused` |
