# SPEC-037: vendoring the pack probes refuses excluded and deny-listed files by construction

- **Wave:** W0. **Issue:** #206 (epic #1). **Context(s):** `repo` (`scripts/`, `.packs/`).
- **Decided by:** ADR-039 (this SPEC's own: vendoring is a committed script that excludes at copy
  time and scrubs before it writes), ADR-004 (the probes vendored and pinned), ADR-033 (the public
  scrub reads every blob).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-037.md` (ADR-016). The
  delivery made R1 to R6 exact where the code decided them, and added the run's schematic to the
  manifest (§7).

## 1. The problem, measured

- **Vendoring had no committed tool.** The probes in `.packs/` were copied from a packs
  checkout by a maintainer's script outside the repository. That script copied every file of every
  listed pack and applied the exclusions that `.packs/VENDORED.json` records only afterwards, by
  hand.
- **The near-miss.** At the train-84 re-pin (#205), re-running that script wrote the two excluded
  subscription-proxy files into the tree as untracked files: the reference client and the
  scanner, which name a private secret. They were noticed and moved out before any commit, and the
  re-pin was redone file by file. A `git add -A` at that moment would have published them, and only
  the gate's history scrub (SPEC-033) would then have refused the push.
- **The exclusions are prose.** Of `VENDORED.json`'s three `excluded` notes, one is a glob, and two
  name paths in a sentence ("`skills/packs/subscription-proxy/** and scripts/proxy-client-scan.py`").
  No tool can apply them.

## 2. Requirements

R1. `scripts/vendor-packs.py --source DIR [--root ROOT] [--deny-list FILE]` re-vendors from a
    packs checkout. It takes every file `.packs/VENDORED.json` lists (by its `from`), plus any
    new file under a pack directory already vendored, and nothing else. A new pack stays a person's
    decision, because it needs a wiring state.
R2. Every `excluded` entry of `VENDORED.json` carries `globs`: a list of patterns over source paths
    (`fnmatch`, `**` crossing directories). A candidate matching any glob is dropped before it is
    read. The script never writes an excluded path into the tree or into any staging area, not even
    untracked. The three current notes become:
    - `skills/packs/*/examples/**`;
    - `skills/packs/subscription-proxy/SKILL.md`;
    - `skills/packs/subscription-proxy/**` and `scripts/proxy-client-scan.py`.
R3. Before it writes anything, the script reads every remaining candidate from the source and scans
    it with the public scrub's own rules (`scripts/public-scrub.py`): persona-core's and
    privacy-gdpr's shapes, plus the maintainer's private list from `--deny-list` or
    `$PERSONA_CORE_DENY_LIST`.
    - A binary is refused by the scrub's `binary` rule.
    - The rule files the scrub skips (`deny-list.json`) are skipped the same way.
    - Any finding refuses the run with exit 1, and one line per finding names the source path and
      the rule, never the value.
R4. All or nothing: the tree changes only when every candidate passed. Then the script:
    - writes the files;
    - updates `VENDORED.json` (each digest, and `vendored_from` = the source's `HEAD`);
    - updates `methodology.json`'s `vendored_from`.
    On any refusal the tree is byte-identical to before the run.
R5. A file `VENDORED.json` lists that the source lacks refuses the run with exit 2, naming it. The
    script deletes nothing.
R6. The run ends with one line naming the files examined, changed, new and excluded, and the commit
    they came from. It exits 3 when it examined nothing.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| ~~A1~~ | an excluded upstream file is never written into the tree | `test_vendor_packs.py` |
| ~~A2~~ | a planted address shape upstream refuses the run by path and rule, and the tree is unchanged | `test_vendor_packs.py` |
| ~~A3~~ | a private literal upstream refuses the run by its index, never its value | `test_vendor_packs.py` |
| ~~A4~~ | a binary upstream file refuses the run | `test_vendor_packs.py` |
| ~~A5~~ | a listed file the source lacks refuses the run with exit 2, and nothing is deleted | `test_vendor_packs.py` |
| ~~A6~~ | a clean upstream re-vendors with its digests and its commit | `test_vendor_packs.py` |
| ~~A7~~ | every exclusion the manifest records is machine-applicable | `test_vendored_packs.py` |

```acceptance
```
```retired
A1: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k an_excluded_upstream_file_is_never_written_into_the_tree
A2: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k a_planted_address_upstream_refuses_the_run_and_the_tree_is_unchanged
A3: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k a_private_literal_upstream_refuses_the_run_by_index
A4: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k a_binary_upstream_file_refuses_the_run
A5: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k a_listed_file_the_source_lacks_refuses_the_run
A6: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k a_clean_upstream_revendors_with_its_digests_and_commit
A7: python3 -m unittest discover -s scripts/tests -p test_vendored_packs.py -k every_exclusion_is_machine_applicable
```

A1 to A6 run the script against a fixture upstream, built at run time in a temporary directory as
a git repository with one commit. The fixture's planted address and literal are assembled at run
time, so the test file itself holds neither.

The red stub, committed with the tests, is the old behaviour: it copies every listed and new file
and applies no exclusion and no scan. A1 is then red because the excluded file lands in the tree,
and A2 to A5 are red because the run exits 0.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/vendor-packs.py` | `repo` | added: R1 to R6 |
| `scripts/tests/test_vendor_packs.py` | `repo` | added: A1 to A6 |
| `scripts/tests/test_vendored_packs.py` | `repo` | changed: A7 |
| `.packs/VENDORED.json` | `repo` | changed: `excluded` entries carry `globs` |
| `docs/decisions/ADR-039-vendoring-is-a-committed-script-that-scrubs-before-it-writes.md` | `repo` | added |
| `docs/specs/SPEC-037-vendoring-refuses-excluded-and-deny-listed-files.md` | `repo` | added (moved from `docs/specs/planned/`) |
| `docs/red-first/SPEC-037.md` | `repo` | added |
| `docs/schematics/pack-vendoring.md` | `repo` | added: the run's data flow (an amendment: the order of work asks a schematic before a new data flow) |
| `changelog.d/` fragment | `repo` | added |

## 5. What this does NOT do

- It vendors no new pack. Adding one is a wiring decision with its own issue, and the script
  reports it and leaves it out (#60).
- It runs no binary-built pack and builds no binary: the box run stays `scripts/box-packs.sh` (#23).
- It deletes nothing, upstream or local. A vanished upstream file stops the run for a person (#60).
- It does not replace the history scrub. The gate still reads every blob (SPEC-033, #206).

## 6. Risks

- **An upstream file carries a shape the scrub reads as private, though it is a rule or an
  example.** The run refuses, naming the file. A person then excludes it in `VENDORED.json` with a
  reason, never by weakening the scan.
- **The exclusion globs drift from what upstream ships**, for example a renamed client directory.
  A1's fixture holds the current names, and R3's scan catches the secret-bearing file even under a
  new name.

## 7. Amendments at delivery

- **R1: the files come from the source's commit, never its working tree.** The run lists the
  commit `HEAD` names (`git ls-tree`) and reads each candidate's bytes from it (`git cat-file`), so
  `vendored_from` names exactly what was read, and an untracked file in the checkout is never a
  candidate. A pack is a `skills/packs/<pack>/` directory. The run names every upstream pack that
  is not vendored and leaves it out, and names apart a pack whose every file an exclusion matches
  ("excluded whole"), so an excluded pack is never offered as a new one.
- **R2: `fnmatch` treats `/` as an ordinary character,** so `*` crosses directories as `**` does,
  and a glob can exclude more than its directory reading suggests, never less. A listed file an
  exclusion matches is never read, and a run that passes drops its entry, so the manifest records
  only what it vendored; the run deletes no file, and `test_vendored_packs.py` names a copy the
  tree still holds. An `excluded` entry without `globs` or `why` stops the run with exit 2 before
  anything is read: a prose exclusion fails closed.
- **R3: the rules are composed as the scrub's `main()` composes them,** by importing
  `scripts/public-scrub.py` and using its `Scan`, binary rule, size limit (`oversize`), skipped
  names and blob reader; `public-scrub.py` itself is unchanged. A symlink in the commit is refused
  by the rule `symlink`, because it would be vendored as a text file holding its target.
- **R4: a file the tree already holds byte for byte, with the same executable bit, is not
  touched,** and `VENDORED.json` and `methodology.json` are written only when their content
  changes, so a run with nothing to change leaves the tree byte-identical. The executable bit
  follows upstream. `methodology.json` keeps its own layout: only its pin's value is rewritten.
  The script sets `sys.dont_write_bytecode` before it loads the scrub, so a run writes no
  `__pycache__` into either tree.
- **R5 and R6: exit 2 also names a usage error** (an unreadable manifest, pin or private list, a
  source that is not a git checkout, a destination claimed twice or reached through a symlink),
  always before any write. A refused run ends with the same summary line, where `changed` and
  `new` are 0 because nothing was written.
- **The manifest gains `docs/schematics/pack-vendoring.md`,** the run's data flow, because the
  order of work asks a schematic before a new data flow.

## 8. Amendment, 2026-09-28: criteria whose tests SPEC-056 removed

Made by SPEC-056 (ADR-069), insert-only under ruling (i) of SPEC-038 section 8: every earlier byte
is kept in order. It inserts:

- section 3: `~~` around A1 to A7 in the criteria table, so the table no longer states them;
- section 3: the fence lines that close the acceptance fence before its first line, which leaves it
  empty, and set A1 to A7 apart in a `` ```retired `` fence after it;
- this section.

The retired criteria, why their subject is gone, and what judges it now:

- A1 to A7 (the vendoring script's refusals, its digests and its exclusions): SPEC-056 removed the
  vendoring script, the vendored tree and their tests. Nothing is vendored any more, and SPEC-056 A1
  proves that neither is in the tree.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).
