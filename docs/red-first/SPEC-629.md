# Red-first record: SPEC-629

The SPEC, ADR-651, the schematic section and the changelog fragment were committed first. The
census's new tests were committed next, over the census as dev holds it, which judges no spacing:
the tests of A1, A2, A3 and A5, the four planted values of R7 written with no typed space, and the
module docstring's first line. Nothing else in the test module changed. So each red below is an
assertion on the missing `spaced` finding, never an import error or a crash. The census's refusal
and the band's rows follow, then the stripped catalogs; the test module is unchanged after the
commit that carries the reds.

Each red is quoted from a run of a fence line, `python3 -m unittest <name>`, over the tree
committed as the sha named, and each green is measured at the commit named.

## The fence, line by line

Each of the 5 lines of SPEC-629 section 3's fence resolves to one test, named by its exact name.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_latin_letter_digit_or_percent_is_refused` | added |
| 2 | A2 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_placeholder_is_refused` | added |
| 3 | A3 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_space_outside_a_cjk_and_latin_boundary_is_not_refused` | added |
| 4 | A4 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_planted_complete_tree_reads_green` | named |
| 5 | A5 | `scripts/tests/test_locale_census.py` `TheWebTree.test_the_japanese_and_chinese_catalogs_hold_no_typed_space_at_a_boundary` | added |

## How each red is measured

- A1 and A2 plant one spaced value into the complete planted tree and assert the census's exact
  lines and exit code; over the census that judges no spacing they read exit 0 and no finding.
- A5 first counts a planted control; over the census that judges no spacing the control reads
  exit 0 and no finding. Once the refusal lands, its tree half reads the catalogs' typed spaces
  until they are stripped.
- A3 guards against over-refusal and A4 is the control: neither can be red first.
