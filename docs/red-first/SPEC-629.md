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

## The reds and greens

```red-first
A1: red at 59dac4d: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['spaced ja title 1', 'examined 7 locales, 4 keys'], '')
A1: green at 3480dfe
A2: red at 59dac4d: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['spaced ja progress 1', 'examined 7 locales, 4 keys'], '')
A2: green at 3480dfe
A3: not red: it guards against over-refusal, and the census before the refusal refuses no spacing
A4: not red: the control: a complete planted tree reads green before and after the refusal
A5: red at 59dac4d: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['spaced ja title 1', 'examined 7 locales, 4 keys'], '')
A5: green at 48c8a97
```

## How each red was read

- At 59dac4d the whole module, `python3 -m unittest -v scripts.tests.test_locale_census`, ran 16
  tests and failed exactly A1, A2 and A5, in 18 failures counting subtests, each an
  `AssertionError` on the missing `spaced` finding; A3, A4 and every SPEC-571 test passed. The
  pull request's run at that commit, ci run 42157984235, read the same 18 failures by name in its
  `hygiene` job; its `ci` aggregate read red with it, and every other job read success or skipped.
- At 3480dfe, the census's refusal, A1, A2 and A3 read green, and exactly two tests read red by the
  catalogs as they stood, A5 and SPEC-571's `TheWebTree.test_the_web_apps_catalogs_are_whole`,
  each by `AssertionError: Lists differ: ['spaced ja wallet_loss_cap 1', 'spaced ja[4062 chars]y 2'] != []`.
  A5 printed `typed spaces at a CJK and Latin boundary: 203 in ja, zh-Hans, zh-Hant; planted control counted 1`.
- At 48c8a97, the stripped catalogs, the module ran 16 tests, OK, and A5 printed
  `typed spaces at a CJK and Latin boundary: 0 in ja, zh-Hans, zh-Hant; planted control counted 1`.
- The four planted values of R7 lost their typed spaces with their assertions unchanged: the
  planted `progress` values `{total}枚中{done}枚`, `{total}张中的{done}张` and `{total}張中的{done}張`,
  and the "added" placeholder case `關閉{name}`.
- No test changed after 59dac4d, and each green commit is byte-identical to this record's commit
  on what it exercises: `git diff --quiet 59dac4d HEAD -- scripts/tests`,
  `git diff --quiet 3480dfe HEAD -- scripts/locale_census.py scripts/tests` and
  `git diff --quiet 48c8a97 HEAD -- scripts web` each exit 0 here.

## The rows, proved by hand

Each row of `scripts/mutation-rows.d/S62900-S62999.json`, S62901 to S62911, was proved on the
tree committed at 3480dfe: its find occurs once in a copy of that tree, its replace applied once
makes its named killer fail, and the same killers pass on an unmutated copy.
