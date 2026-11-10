# SPEC-673: no pull request adds or grows a file past the large-file bound unless it is listed

- **Issue:** #1393, a follow-up to SPEC-436 and ADR-450, from #913. **Context(s):** `repo`
  (`scripts/tests/` and `docs/`).
- **Decided by:** ADR-696 (this SPEC's own: one census of every tracked file's blob size, with a
  listing of today's sites at their recorded sizes, at warn). It cites ADR-450 D5 and D6 and
  SPEC-436 R21 to R24, and amends none of them.
- **Status:** built. It holds `docs/red-first/SPEC-673.md`. **Mutation band:** `S67300-S67399`.

## 1. The problem, measured

Read at the development branch's commit `b9a08e60`: the tree by `git ls-tree -r -l`, and each
named file by `git show b9a08e60:<path>`.

- **No check reads a tracked file's size.** No test, census or CI job reads the size of a tracked
  file, and no SPEC or ADR states a size rule for one. ADR-359's caps (a file over 4 MiB, or past
  16 MiB for the face) bound the media a review screen sends while it runs, not a tracked file,
  and ADR-433 rejects a file's bytes as a proxy for its mutant count. So a pull request can add a
  file of any size, or grow one, and every check stays green.
- **The attributes route no path to large-file storage.** The one `.gitattributes`, at the root,
  sets `* text=auto eol=lf`, marks nine image and font extensions `binary` and the two lock files
  `linguist-generated=true`, and holds no `filter=lfs` line.
- **Five files read are past 1 MiB, each a parity golden.** Each is a golden of the second
  oracle under `tools/parity-oracle/goldens/`, which its driver writes, a Rust test reads and
  `test_goldens.py` keeps current (SPEC-498, SPEC-029): `anki.render.card.json` at 1685445 bytes,
  `anki.render.compare_answer.json` at 1452309, `anki.render.occlusion_render.json` at 1407645,
  `anki.render.latex_images.json` at 1237599 and `anki.read.collection_view.json` at 1067666. The
  next golden is 1035047 bytes, the largest file read that is not a golden is
  `scripts/tests/test_ci_workflows.py` at 681304 bytes, and no file read is past 2 MiB.
- **This reading is a lower bound.** It read the tree by path, and some tracked paths were left
  out of it by name. The census reads every tracked file: its first run in CI prints each site,
  and each site it prints outside those paths is listed with its recorded size before the change
  merges. A site among those paths prints at warn, unlisted.

## 2. Requirements

- **R1.** The census is `scripts/tests/test_large_files.py`. Its population is every entry git
  tracks at HEAD, read by `git ls-tree -r -z --full-tree` with the format
  `%(objectmode) %(objecttype) %(objectsize)%x09%(path)`, whatever the entry's name, kind or size.
  It reads sizes only and opens no tracked file. In a pull request's CI run HEAD is the merge of
  the pull request into its base.
- **R2.** The bound is `BOUND = 1048576` bytes (1 MiB). A file is past the bound when its blob is
  strictly larger.
- **R3.** Each entry is read as its mode, its type, its blob size and its path, the path whole
  after the first tab. A size that is not a whole number of bytes, the `-` git prints for a
  submodule's commit, is no blob size.
- **R4.** A site is a file past the bound with no entry in `LISTED`, printed
  `WARN large-file <path> <size> (unlisted, past the bound <bound>)`, or a listed file whose blob
  is larger than its entry's recorded size, printed
  `WARN large-file <path> <size> (listed at <recorded>, grown past it)`.
- **R5.** Not a site: a file at or under the bound with no entry; a listed file at or under its
  recorded size; a removed file with no entry, which is in no population; and a link, which git
  stores as a blob holding the name it points at, judged by that blob's size like any file.
- **R6.** A stale entry is an entry whose path holds no tracked file, printed
  `STALE large-file <path> <recorded> (no tracked file at this path)`, or an entry whose file is at
  or under the bound, printed `STALE large-file <path> <recorded> (at or under the bound)`. No arm
  follows a renamed or moved listed file: its old entry is stale and its new path is judged as an
  unlisted file.
- **R7.** An entry with no blob size is undecided, printed
  `UNDECIDED large-file <path> (<type> entry, mode <mode>: no blob size)`.
- **R8.** `LISTED` maps each of today's sites to its recorded size and its reason: the five goldens
  of section 1 at their sizes, and each further site the first CI run prints outside the paths
  section 1 names as left out, at the size it prints.
- **R9.** `LEVEL` is `warn`. At `warn` a site and an undecided entry print and pass; at `fail`
  each fails naming its line. A stale entry fails at both levels. Moving `LEVEL` to `fail` is its
  own delivery, under ADR-450 D6.
- **R10.** The census prints `examined <n> file(s)` and fails on an empty population, which is
  VOID and never a pass.
- **R11.** The planted listings sit under `scripts/tests/fixtures/large-files/`, each named with
  a `.fixture` suffix so no runner collects it, and are read as text: one entry per row as the
  format of R1 prints it, a row opening `#` a comment. No planted file is large, so no fixture is
  a site.
- **R12.** No other file changes: no `.gitattributes`, workflow, script, production file or
  existing test, and no tracked file is removed, shrunk, split or moved to large-file storage.

## 3. Acceptance criteria of SPEC-673

| id | criterion | decided by |
|---|---|---|
| A1 | R2, R4: a planted file one byte past the bound with no entry is the one site, printed as R4 spells it, and files at the bound and far under it are not | `test_large_files.py` `test_a_file_past_the_bound_with_no_entry_is_a_site` |
| A2 | R4, R5: a file at the bound, an empty file and an executable file are read whole, a path with spaces included, and none is a site | `test_large_files.py` `test_a_file_at_the_bound_is_not_a_site` |
| A3 | R5: a listed file at its recorded size and one a byte under it, each past the bound, are neither sites nor stale | `test_large_files.py` `test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site` |
| A4 | R4: a listed file one byte past its recorded size is a site, printed as grown past it | `test_large_files.py` `test_a_listed_file_grown_past_its_recorded_size_is_a_site` |
| A5 | R6: a listed file moved to a new path at its recorded size leaves its entry stale, with no tracked file at its path, and is an unlisted site at the new path | `test_large_files.py` `test_a_renamed_listed_file_leaves_a_stale_entry_and_an_unlisted_site` |
| A6 | R6: a listed file back at the bound leaves its entry stale | `test_large_files.py` `test_a_listed_file_back_at_the_bound_leaves_a_stale_entry` |
| A7 | R3, R5, R7: a submodule's commit reads with no blob size and is undecided, and a link's blob is read by its size and is not a site | `test_large_files.py` `test_an_entry_with_no_blob_size_is_undecided_and_a_link_is_judged_by_its_size` |
| A8 | R9: at `warn` a site and an undecided entry pass and a stale entry fails; at `fail` all three fail, each naming its line | `test_large_files.py` `test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does` |
| A9 | R10: an empty planted listing fails naming `examined 0 file(s)` | `test_large_files.py` `test_an_empty_population_is_void` |
| A10 | R11: the planted listings are exactly the eight this file reads, each with a `.fixture` suffix | `test_large_files.py` `test_the_planted_listings_are_the_eight_this_file_reads` |
| A11 | R8, R9: `LEVEL` is `warn`, and each listed entry records a size past the bound and a reason | `test_large_files.py` `test_the_census_is_at_warn_and_each_entry_records_a_site_and_its_reason` |
| A12 | R1, R8: over the repository, the population holds the census's own file and nothing fails at the committed level | `test_large_files.py` `test_no_tracked_file_fails_the_census_at_its_level` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_file_past_the_bound_with_no_entry_is_a_site
A2: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_file_at_the_bound_is_not_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site
A4: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_listed_file_grown_past_its_recorded_size_is_a_site
A5: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_renamed_listed_file_leaves_a_stale_entry_and_an_unlisted_site
A6: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_a_listed_file_back_at_the_bound_leaves_a_stale_entry
A7: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_an_entry_with_no_blob_size_is_undecided_and_a_link_is_judged_by_its_size
A8: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does
A9: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_an_empty_population_is_void
A10: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_the_planted_listings_are_the_eight_this_file_reads
A11: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_the_census_is_at_warn_and_each_entry_records_a_site_and_its_reason
A12: python3 -m unittest discover -s scripts/tests -p test_large_files.py -k test_no_tracked_file_fails_the_census_at_its_level
```

Every planted listing is text the test reads, and every expected entry, line and list is a literal
the test writes, never a value the census computes. Each test asserts its positive artifact (the
read entries, the printed lines, the planted names) beside its examined count. R12 is a rule of
the documents, read in review.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_large_files.py` | repo | added: the census and its tests (R1 to R11) |
| `scripts/tests/fixtures/large-files/past-the-bound.fixture` | repo | added: a planted listing (R2, R4) |
| `scripts/tests/fixtures/large-files/at-the-bound.fixture` | repo | added: a planted listing (R4, R5) |
| `scripts/tests/fixtures/large-files/listed-at-its-size.fixture` | repo | added: a planted listing (R5) |
| `scripts/tests/fixtures/large-files/listed-and-grown.fixture` | repo | added: a planted listing (R4) |
| `scripts/tests/fixtures/large-files/listed-and-renamed.fixture` | repo | added: a planted listing (R6) |
| `scripts/tests/fixtures/large-files/listed-and-shrunk.fixture` | repo | added: a planted listing (R6) |
| `scripts/tests/fixtures/large-files/no-blob-size.fixture` | repo | added: a planted listing (R3, R5, R7) |
| `scripts/tests/fixtures/large-files/empty.fixture` | repo | added: a planted listing (R10) |
| `scripts/mutation-rows.d/S67300-S67399.json` | repo | added: the fourteen rows of section 9 |
| `docs/specs/SPEC-673-no-pull-request-adds-or-grows-a-file-past-the-large-file-bound-unless-it-is-listed.md` | repo | added |
| `docs/decisions/ADR-696-a-census-reads-every-tracked-files-blob-size-and-lists-todays-large-files-at-warn.md` | repo | added |
| `docs/red-first/SPEC-673.md` | repo | added |
| `changelog.d/large-file-guard-673.md` | repo | added |

## 5. What this does NOT cover

- It changes no tracked file: no golden is removed, shrunk, split or moved to large-file storage
  (#1393).
- It changes no `.gitattributes` line and routes no path to large-file storage (#1393).
- It follows no rename or move of a listed file by its content (#1393).
- It reads no file's growth below the bound, and no total size of the tree or of a pull request
  (#1393).
- It changes no CI job, no workflow and no test-runner configuration (#1393).

## 6. Risks

- **A site the first CI run prints.** Section 1 is a lower bound; each site that run prints
  outside the paths section 1 names as left out is listed at its printed size before the change
  merges, never relisted silently afterwards.
- **A sibling pull request grows a listed golden.** A regenerated golden past its recorded size
  prints a `WARN` line at warn; its own pull request records the new size in `LISTED`.
- **A golden shrinks to the bound or under it.** Its entry is stale and fails at both levels, so
  the pull request that shrinks it removes the entry.
- **The census reads the merge, not the branch.** In a pull request's CI run HEAD is the merge
  into the base, so a file the base grew is read at its merged size, which is the size that lands.
- **A tree-scanning guard reads the new files.** `scripts/tests/test_temp_hygiene.py` reads every
  Python test for a leaked temporary path, and the census writes no file; the planted listings
  carry no object name.

## 7. What only CI proves

- The census over the repository, the last criterion of section 3, reads every tracked file, so it
  is read in the `hygiene` job of CI, which runs `scripts/tests`; the planted tests run anywhere by
  name.

## 8. Formal model

No state, actor or write path is added, and no covered span is edited: the census is a pure reader
of the listing git prints. Its arms are closed by the planted listings, whose expected lines are
spelt literally, and by section 9's rows. No model checker runs.

## 9. Mutation rows (S67300-S67399)

Band `S67300` to `S67399`, `scripts/mutation-rows.d/S67300-S67399.json`, target
`scripts/tests/test_large_files.py`. One row per bound, arm, boundary, reading and level: each find
is taken from the green commit's text and occurs exactly once in its file.

| id | find | replacement | killer |
|---|---|---|---|
| `S67300-THE-BOUND-IS-ONE-MEBIBYTE` | `BOUND = 1048576` | `BOUND = 1048577` | `test_large_files.PlantedListings.test_a_file_past_the_bound_with_no_entry_is_a_site` |
| `S67301-A-FILE-AT-THE-BOUND-IS-NOT-A-SITE` | `elif size > bound:` | `elif size >= bound:` | `test_large_files.PlantedListings.test_a_file_at_the_bound_is_not_a_site` |
| `S67302-A-LISTED-FILE-AT-ITS-SIZE-IS-NOT-A-SITE` | `if size > listed[path][0]:` | `if size >= listed[path][0]:` | `test_large_files.PlantedListings.test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site` |
| `S67303-A-LISTED-FILE-GROWN-PAST-ITS-SIZE-IS-A-SITE` | `if size > listed[path][0]:` | `if size > listed[path][0] + 1:` | `test_large_files.PlantedListings.test_a_listed_file_grown_past_its_recorded_size_is_a_site` |
| `S67304-A-LISTED-FILE-IS-JUDGED-BY-ITS-ENTRY` | `if path in listed:` | `if False:` | `test_large_files.PlantedListings.test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site` |
| `S67305-THE-PRESENT-SIZE-IS-THE-BLOBS-OWN` | `present[path] = size` | `present[path] = 0` | `test_large_files.PlantedListings.test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site` |
| `S67306-AN-ENTRY-WITH-NO-TRACKED-FILE-IS-STALE` | `gone = sorted(path for path in listed if path not in present)` | `gone = []` | `test_large_files.PlantedListings.test_a_renamed_listed_file_leaves_a_stale_entry_and_an_unlisted_site` |
| `S67307-AN-ENTRY-BACK-AT-THE-BOUND-IS-STALE` | `present[path] <= bound` | `present[path] < bound` | `test_large_files.PlantedListings.test_a_listed_file_back_at_the_bound_leaves_a_stale_entry` |
| `S67308-A-SIZE-THAT-IS-NOT-A-NUMBER-IS-NO-BLOB-SIZE` | `int(size) if size.isdigit() else None` | `int(size) if size.isdigit() else 0` | `test_large_files.PlantedListings.test_an_entry_with_no_blob_size_is_undecided_and_a_link_is_judged_by_its_size` |
| `S67309-A-STALE-ENTRY-FAILS-AT-BOTH-LEVELS` | `return (sites + undecided if level == "fail" else []) + stale` | `return sites + undecided if level == "fail" else []` | `test_large_files.PlantedListings.test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does` |
| `S67310-A-SITE-FAILS-AT-FAIL` | `return (sites + undecided if level == "fail" else []) + stale` | `return (undecided if level == "fail" else []) + stale` | `test_large_files.PlantedListings.test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does` |
| `S67311-AN-UNDECIDED-ENTRY-FAILS-AT-FAIL` | `return (sites + undecided if level == "fail" else []) + stale` | `return (sites if level == "fail" else []) + stale` | `test_large_files.PlantedListings.test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does` |
| `S67312-THE-CENSUS-STARTS-AT-WARN` | `LEVEL = "warn"` | `LEVEL = "fail"` | `test_large_files.TheListing.test_the_census_is_at_warn_and_each_entry_records_a_site_and_its_reason` |
| `S67313-AN-EMPTY-POPULATION-IS-VOID` | `entries = examined("file(s)", entries)` | `entries = list(entries)` | `test_large_files.PlantedListings.test_an_empty_population_is_void` |

## 10. References

Issue #1393; issue #913; SPEC-436 R21 to R24; ADR-450 D5 and D6; ADR-359; ADR-433; SPEC-498;
SPEC-029; ADR-696.
