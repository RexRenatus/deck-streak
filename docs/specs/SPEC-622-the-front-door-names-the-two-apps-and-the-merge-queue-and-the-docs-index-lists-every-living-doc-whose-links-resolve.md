# SPEC-622: the front door names the two apps and the merge queue, and the docs index lists every living doc whose links resolve

- **Issue:** none. **Context(s):** the repository's front door (`README.md`, `AGENTS.md`,
  `CONTRIBUTING.md`), the docs index (`docs/README.md`), every living doc, `scripts/tests` and
  `docs/`.
- **Decided by:** ADR-644 (the docs index and the link check are one standard-library test module
  in the gate, chosen over a link-checker action); ADR-589 (the web app and the iPhone and iPad app
  are DeckStreak's surfaces, and the Telegram Mini App and bot are deprecated); ADR-625 (dev merges
  through the Mergify merge queue).
- **Schematic:** `docs/schematics/docs-front-door.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-622.md`. **Mutation band:** S62200-S62299.

## 1. The problem, measured

Read at dev `8620664`, by `git show <dev>:<path>`, `git ls-files` and `grep -n`:

- **The front door names one app, and the deprecated one.** `README.md` line 27 calls DeckStreak "a
  Rust workspace (the backend) and a SvelteKit Mini App (the front end)"; lines 36 and 52 name the
  Mini App's dependencies and tests; lines 58 to 60 say a deployment needs "a Telegram bot".
  `AGENTS.md` line 10 says "the Mini App is SvelteKit in `web/app`" and line 25 names the Mini App's
  unit tests. `CONTRIBUTING.md` line 43 names "the Mini App's checks and tests". ADR-589 made the web
  app and the iPhone and iPad app the surfaces, yet `grep -c -i 'ios/'` reads 0 on each of the four
  pages.
- **The merge queue is missing from the front door.** `.mergify.yml` holds one queue rule, `dev`,
  with `batch_size: 6`, `merge_method: merge` and the queue conditions `base = dev`,
  `label = queue`, `-draft`, `#review-threads-unresolved = 0` and `check-success =` each of `ci`,
  `fragment` and `dependency-review`. `grep -c -i -E 'mergify|merge queue'` reads 0 on each of the
  four pages, and `CONTRIBUTING.md` lines 8 to 9 still call `main` "the branch visitors see",
  though `dev` is the default branch.
- **The docs index misses about half of the living docs.** By R3's rule `git ls-files` yields 30
  living docs, `docs/README.md` among them. `docs/README.md` links 15 of the other 29 and the folders
  `decisions/`, `specs/` and `schematics/`; it misses 14 living docs (among them `CHANGELOG.md`,
  `PRIVACY.md`, `deploy/README.md` and the two runbooks under `docs/runbooks/`) and 4 record folders
  (`docs/specs/planned/`, `docs/red-first/`, `docs/rulings/` and `changelog.d/`).
- **Nothing checks a link.** No module under `scripts/tests` resolves the relative links of a
  living doc; `scripts/check.sh` runs `python3 -m unittest discover -s "$suite" -p 'test_*.py'`
  over `scripts/tests`, so a new module there joins the gate with no workflow change.

## 2. Requirements

- **R1.** The front door names DeckStreak's two surfaces. The `## Install` section of `README.md`
  names the SvelteKit web app in `web/app` and the SwiftUI iPhone and iPad app in `ios/` beside the
  Rust workspace, and the `## Overview` section of `AGENTS.md` names both with their folders. A
  build fact about `ios/` is read from the tree, with no version figure.
- **R2.** In the text this delivery owns (`CONTRIBUTING.md` and `docs/README.md` whole; `README.md`
  from its `## Install` heading up to its `## Source code` heading; `AGENTS.md` whole except the
  first sentence of its Overview paragraph), every block that matches
  `mini[\s-]?app|telegram|\bbots?\b|initdata` (case-insensitive) says `deprecated` or cites
  `ADR-589`. A block is a blank-line-separated paragraph, each list item with its continuation
  lines, and each line inside a fenced code block. The Overview of `AGENTS.md` gains one paragraph
  that states ADR-589's outcome for an agent: the two apps are the surfaces; the Telegram Mini App
  and bot are deprecated, gain no feature, and keep their code until one removal delivery takes it
  out; the service answers one owner at launch; and Rex is the only scheduler, as CHARTER item 22
  states, pointed to and not restated.
- **R3.** `docs/README.md` links every living doc and each record folder, under the four Diataxis
  sections it keeps. A living doc is a tracked `*.md` file outside `docs/specs/`, `docs/decisions/`,
  `docs/schematics/`, `docs/red-first/`, `docs/rulings/`, `changelog.d/` and `agent/` and outside
  any `tests/` or `fixtures/` folder, `docs/README.md` itself excepted. The record folders are
  `docs/decisions/`, `docs/specs/`, `docs/specs/planned/`, `docs/schematics/`, `docs/red-first/`,
  `docs/rulings/` and `changelog.d/`. A link lists a target when its destination, resolved from
  `docs/`, names that path (a folder with or without its trailing slash). The test lists tracked
  files by `git ls-files` from the repository root, names only, and opens only living docs.
- **R4.** Every relative destination in a living doc resolves. The check reads inline links and
  images, reference definitions and HTML `href` and `src` values, outside fenced code and inline
  code spans. A destination's path resolves from the doc's folder to a tracked file or folder; a
  fragment resolves to a heading anchor of the target Markdown file by GitHub's anchor rule, and a
  bare `#fragment` to one of the same file; a destination with a scheme (`https:`, `mailto:`) is
  neither fetched nor checked. The check runs in the gate as a module of `scripts/tests`, with no
  workflow change.
- **R5.** The `## Branch model` section of `CONTRIBUTING.md` states that `dev` is the default branch
  and the base of every pull request; that `main` changes only through a release pull request from
  `dev` merged with a merge commit; and how a pull request into `dev` lands: through the Mergify
  merge queue, which it enters when it carries the label `.mergify.yml` names, is not a draft, has
  no unresolved review thread and passes each check `.mergify.yml` requires, after which the queue
  tests up to its batch size together (written as a word) and merges each by a merge commit. The
  test reads the label, the checks, the batch size and the merge method from `.mergify.yml` with the
  standard library, so a change there reds the page.
- **R6.** The `## Contributing` section of `README.md` and the `## Pull requests` section of
  `AGENTS.md` each name the Mergify merge queue and its label, read from `.mergify.yml`.
- **R7.** The identity text and the pinned clauses stay: `README.md` lines 1 to 26 and its closing
  lines from `## Source code` on, and `AGENTS.md` lines 1 to 9, stay byte-equal; the `## ` headings
  of `README.md`, and so its contents list, are unchanged; `test_fork_rule.py`,
  `test_brand_files.py` and `test_privacy_policy.py` pass unchanged.
- **R8.** Nothing outside section 4 changes: `CLAUDE.md`, `CHARTER.md`, `ARCHITECTURE.md`,
  `RELEASING.md`, `PRIVACY.md`, `SECURITY.md`, the `docs/` guides, ADR-589, `.mergify.yml`,
  `scripts/check.sh`, `.github` and the three modules R7 names stay as they are.

## 3. Acceptance criteria of SPEC-622

| id | criterion | decided by |
|---|---|---|
| A1 | the docs index links every living doc and record folder, and a planted index missing one living doc is refused (R3) | `test_front_door_docs.py` |
| A2 | every relative link in a living doc resolves; planted docs with a missing file, a missing folder and a missing anchor are each refused, and a broken link inside a fence or a code span is ignored (R4) | `test_front_door_docs.py` |
| A3 | the branch model states the default branch, the release path and the queue facts read from `.mergify.yml`, and a planted queue file with another label refuses the real page (R5) | `test_front_door_docs.py` |
| A4 | the README and AGENTS name the Mergify merge queue and its label (R6) | `test_front_door_docs.py` |
| A5 | the front door names the web app in `web/app` and the iPhone and iPad app in `ios/` (R1) | `test_front_door_docs.py` |
| A6 | every Mini App, bot or Telegram block of the owned text says deprecated or cites ADR-589; a planted block naming the Mini App as current is refused and one saying deprecated passes (R2) | `test_front_door_docs.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_docs_index_links_every_living_doc_and_record_folder
A2: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_every_relative_link_in_a_living_doc_resolves
A3: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_contributing_states_the_branch_model_and_the_queue_file_facts
A4: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_readme_and_agents_name_the_merge_queue_and_its_label
A5: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_front_door_names_the_web_app_and_the_iphone_and_ipad_app
A6: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_front_door_names_the_mini_app_and_bot_only_as_deprecated
```

R7 is held by the three existing modules it names, unchanged, and by the hand-back's byte checks;
R8 by the pull request's file list against section 4.

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-622-the-front-door-names-the-two-apps-and-the-merge-queue-and-the-docs-index-lists-every-living-doc-whose-links-resolve.md` | new (C1) |
| `docs/schematics/docs-front-door.md` | new (C1) |
| `docs/decisions/ADR-644-the-docs-index-and-the-link-check-are-one-standard-library-test-module-in-the-gate-chosen-over-a-link-checker-action.md` | new (C1) |
| `changelog.d/docs-front-door-622.md` | new (C1) |
| `scripts/tests/test_front_door_docs.py` | new (C2): A1 to A6 |
| `docs/red-first/SPEC-622.md` | new (C2), lines inserted at C4 |
| `README.md` | amended (C3): only cut lines 27 to 91 change; cut lines 1 to 26 and 92 to 106 stay byte-equal; no `## ` heading added, removed or renamed |
| `AGENTS.md` | amended (C3): cut lines 1 to 9 stay byte-equal; the fork-rule clauses kept |
| `CONTRIBUTING.md` | amended (C3): the `## Branch model` heading and the fork-rule clauses kept |
| `docs/README.md` | amended (C3): the four Diataxis sections kept |
| `scripts/mutation-rows.d/S62200-S62299.json` | new (C3) |
| `scripts/tests/test_ci_workflows.py` | ONLY IF CI's python.log at push 1 shows a census there refusing the new module by name: insert-only, the census listing alone, in the census's own `**allowed(<reason>, (module, qual, text, count))` form, at a line no open pull request's hunk holds (measured by hunk header); otherwise untouched |

## 5. What this does NOT cover

- The README header and opening paragraph (lines 1 to 26) and the first sentence of the AGENTS
  overview, which the identity text pull request rewrites (#1067).
- The ADR-589 docs check over the README, ARCHITECTURE, AGENTS and the PRD, and its status, which a
  sibling delivery carries; the removal of the deprecated code is tracked apart (#1046).
- Living docs added after this delivery: a delivery that adds a living doc adds its index line to
  `docs/README.md` in the same pull request (#1132, #1146).

## 6. Risks

- **A new living doc reds A1.** A pull request that adds a living doc reads red on A1 until it adds
  its index line; the failure names the missing path, so the cure is one line.
- **The queue file moves.** The merge queue text follows `.mergify.yml` through A3: a changed
  label, check, batch size or merge method reds the branch model until the page says the same.
- **A heading renamed under a link.** A2 refuses a fragment whose heading changed; the failure names
  the doc, the destination and the target, so the link or the heading is fixed in the same change.
- **An anchor rule that differs from GitHub's.** The check follows GitHub's documented rule
  (lower case, spaces to hyphens, punctuation and markup dropped, a numbered suffix for a repeat); a
  heading shape outside it could read red where GitHub resolves it, and the failure names it.
