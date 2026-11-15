# SPEC-676: the counts that living documents state are read, and each bound count is derived again

Path:
`docs/specs/SPEC-676-the-counts-living-documents-state-are-read-and-each-bound-count-is-derived-again.md`.

- **Issue:** #1395 (its epic #913). **Context(s):** the python stage's test modules
  (`scripts/tests/`), their planted fixtures, the rows band and the documents. No document,
  workflow, script, production source or existing test changes.
- **Decided by:** ADR-699 (the counts living documents state are read by line forms in one census,
  bound to a derivation by a listing, and start at warn). It cites SPEC-436 and ADR-450 (D6's
  promotion rule), and amends neither.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-676.md`. Read at dev `b9a08e602fc10c9f3ecd88a1544f3f595d55cbda`.

## 1. The problem, measured

A document line that states a count of something the tree holds (a number of jobs, stages, rows,
crates or files) has no check that derives the count again, so a count can go stale with nothing
to say so. `git grep -n -i 'derived count'` at dev names three lines, none of them a check on a
document: a code pin in `crates/coordination/tests/job_table.rs:200`, a line of ADR-305 and a line
of `docs/red-first/SPEC-362.md`. No test under `scripts/tests` derives a figure a document states:
the tests that name a document read its words or its sets (the device census's contexts in
`scripts/tests/test_sealed_sync_records.py`), never its stated counts.

A tracked Markdown file is a document. It is a dated record, whose counts are history, only by a
rule at dev that makes it append-only; every other document is living text (default-deny):

| class | rule that makes it a record, at dev |
|---|---|
| delivered SPEC (`docs/specs/SPEC-*.md`) | a SPEC there is delivered and never reads planned (`scripts/tests/test_planned_specs.py:60-74`); its figures name the dev they were read at |
| ADR (`docs/decisions/ADR-*.md`) | a decision is replaced by a later ADR, never rewritten (`docs/decisions/adr-template.md:3`) |
| red-first record (`docs/red-first/*.md`) | each line names the commit it was measured at (`docs/BUILDER-BRIEF.md:36-40`) |
| changelog fragment (`changelog.d/*.md`, its README apart) | each pull request adds one, compiled at release (`changelog.d/README.md:3-5`) |
| released changelog section (`CHANGELOG.md` from its first `## [x.y.z]` heading) | a release adds a section, which pull requests never edit (`CHANGELOG.md:8-9`) |
| signed ruling (`docs/rulings/*.md` stating that the commit adding it is signed) | its authority is that signed commit |

A dry run of the census's matching (not the census, which reads the whole tree only in CI) over
338 of the living documents at dev read 333 count claims on prose lines, and left unread 115 in
code blocks, 202 in table rows, 1 in a block quote and 3 on lines naming their run or job id. Two
claims can be derived today and are bound: `docs/TESTING.md:67` states five jobs, and five jobs
of `.github/workflows/ci.yml` run `bash scripts/check.sh`; the message-catalogs schematic's line 1
states seven message catalogs, and seven files match `web/app/messages/*.json`. The other 331 are
unbound sites. These figures are a lower bound: the census's own run in the first push's
`hygiene` job measures the whole population, and the second push lists each site it prints in a document this measurement read; a document it did not read stays in the population, its sites printed at warn, unlisted.

## 2. Requirements

R1. The census's population is every tracked `.md` file that no rule of R2 leaves out, each read
    whole. It prints `examined N living document(s)` and refuses an empty population.
R2. A document leaves the population only by the rule of section 1 that makes it a dated record:
    a delivered SPEC (`docs/specs/SPEC-*.md`, never one under `docs/specs/planned/`), an ADR, a
    red-first record, a changelog fragment (never `changelog.d/README.md`), a ruling under
    `docs/rulings/` that states its commit is signed, and the released sections of
    `CHANGELOG.md` (its preamble stays in).
R3. A count claim is a number in digits that no identifier, version, date, path or line citation
    holds, or a number word from two up, then at most three words that are not function words,
    then a noun naming something the tree holds (the census's `NOUNS`: jobs, stages, steps, rows,
    crates, files, tests, locales, catalogs and the like).
R4. A claim is read only on a prose line. A fenced code block (three or more backticks or tildes,
    closed by a run of the same character at least as long; an unclosed block runs to the end),
    an indented code block (four or more spaces after a blank line, outside a list item), front
    matter (a first line `---`, closed by `---` or `...`), a block quote and an HTML comment are
    not read.
R5. A table row (a line holding `|` above a delimiter row, and the rows under it up to a blank
    line) is not read. Inline code spans, link targets and HTML tags are blanked before a line is
    read.
R6. A line that names its run, job or attempt id (six or more digits) states a figure measured
    once and is not read.
R7. A claim bound by a `BOUND` entry (its document, its words, a derivation and the derivation's
    arguments) has its count derived again from the tree: `jobs-running` counts the jobs of a
    workflow holding a line that runs a command, and `tracked` counts the tracked paths a pattern
    matches.
R8. A bound claim whose derived count differs from its stated count is a site, `mismatch`.
R9. Every other claim is a site, `unbound`, unless `LISTED` names its key (document, count, noun,
    words and ordinal among equal words, never a line number) with its reason.
R10. At `warn` each unlisted site prints `WARN doc-count <form> <key>` and the test passes; at
    `fail` each unlisted site fails it. A `LISTED` or `BOUND` entry that names no current claim
    prints `STALE doc-count` and fails at both levels.
R11. The census lands at `warn`, with `LISTED` filled from the first push's `hygiene` job.
    Moving `LEVEL` to `fail` is its own delivery, under ADR-450 D6's promotion rule.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the repository's living documents are examined, the two bound claims are derived and match, and no listing or binding entry is stale (R1, R2, R7, R9, R10, R11) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_the_repository_holds_todays_listing` |
| A2 | a planted bound claim whose derived count matches is clean, and each derivation counts what it names (R3, R7) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_matches_is_clean` |
| A3 | a planted bound claim whose derived count differs is a `mismatch` site (R8) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_differs_is_a_site` |
| A4 | a planted unbound claim is a site that warns at `warn`, fails at `fail` and passes when listed (R9, R10) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_unbound_claim_is_a_site_at_warn_and_fails_at_fail` |
| A5 | a count in fenced or indented code, a quote, front matter, a comment, a table, an inline span or a line naming its run id is not read (R4, R5, R6) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_count_in_code_a_quote_front_matter_a_table_or_a_measured_run_is_not_read` |
| A6 | each dated record leaves the population by its rule, and a planned SPEC, the fragment guide, an unsigned ruling and the changelog's preamble stay in (R2) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_dated_record_leaves_the_population_by_its_rule` |
| A7 | an empty population is refused (R1) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_an_empty_population_is_void` |
| A8 | a listing entry or a binding that names no current claim fails at both levels (R10) | `python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_listing_entry_that_names_no_site_fails_at_both_levels` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_the_repository_holds_todays_listing
A2: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_matches_is_clean
A3: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_differs_is_a_site
A4: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_unbound_claim_is_a_site_at_warn_and_fails_at_fail
A5: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_count_in_code_a_quote_front_matter_a_table_or_a_measured_run_is_not_read
A6: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_dated_record_leaves_the_population_by_its_rule
A7: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_an_empty_population_is_void
A8: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_listing_entry_that_names_no_site_fails_at_both_levels
```

The planted tests read planted text only, from `scripts/tests/fixtures/doc-counts/`, each file
named `.fixture` and read as text. `test_the_repository_holds_todays_listing` reads the whole tree, so it runs in
the `hygiene` job's python stage, which collects every `scripts/tests` module by name.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_doc_counts.py` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/bound-match.md.fixture` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/bound-differs.md.fixture` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/unbound.md.fixture` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/code-block.md.fixture` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/dated-record.md.fixture` | the python stage | added |
| `scripts/tests/fixtures/doc-counts/planted-ci.yml.fixture` | the python stage | added |
| `scripts/mutation-rows.d/S67600-S67699.json` | the rows band | added |
| `docs/specs/SPEC-676-the-counts-living-documents-state-are-read-and-each-bound-count-is-derived-again.md` | the documents | added |
| `docs/decisions/ADR-699-the-counts-living-documents-state-are-read-by-line-forms-bound-by-a-listing-and-start-at-warn.md` | the documents | added |
| `docs/red-first/SPEC-676.md` | the documents | added |
| `changelog.d/doc-counts.md` | the documents | added |

The new module loads no code through `importlib`, `runpy` or `exec` and imports no module of the
workflow loader's population, so `scripts/tests/test_ci_workflows.py` is not changed.

## 5. What this does NOT do

- It changes no document, so no stated count is reworded here; a site's wording is its own pull
  request, which updates `LISTED` beside it (#1395).
- It does not read table rows, and binds no claim but the two of section 1; a new derivation is
  its own delivery (#1395).
- It does not move `LEVEL` to `fail`; that is its own delivery under ADR-450 D6 (#1395).

## 6. Risks

- A false site (a number that counts no tree thing) is listed with its reason, never silenced by
  a narrower form; a form change that drops a listed site stales its entry, which fails (R10).
- A document whose words change moves its key: the stale entry fails at both levels, so the
  listing is updated in the same pull request (R10).
