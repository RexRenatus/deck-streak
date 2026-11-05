# Red-first record: SPEC-571

The SPEC, ADR-590 and the schematic were committed first (1f9f5811). The census's tests were
committed next over a stub of the census that parses its arguments, reads the settings and the
catalogs, prints the examined line and exits 0 with no finding (81ad67da). The census replaced the
stub (60750d67), and the six catalogs were filled one locale per commit (1c243e1e, 4b02fdda,
84aac0e6, f0c156bd, 8fb210ee, 483661ff). Each red below is quoted from a run of the whole module,
`python3 -m unittest -v scripts.tests.test_locale_census`, over the tree committed as the sha
named, and each green is measured at the commit named.

## The fence, line by line

Each of the 12 lines of SPEC-571 section 3's fence resolves to one test, named by its exact name.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_key_removed_from_one_locale_is_refused` | added |
| 2 | A2 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_blank_value_is_refused_in_any_locale` | added |
| 3 | A3 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_placeholder_that_differs_from_english_is_refused` | added |
| 4 | A4 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_key_english_lacks_is_refused_as_stale` | added |
| 5 | A5 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_value_out_of_nfc_is_refused` | added |
| 6 | A6 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_locale_set_other_than_the_seven_is_refused` | added |
| 7 | A7 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_an_unreadable_or_repeating_catalog_is_refused` | added |
| 8 | A8 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_value_that_is_not_text_is_refused` | added |
| 9 | A9 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_planted_complete_tree_reads_green` | added |
| 10 | A10 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_a_tree_with_no_catalog_is_void` | added |
| 11 | A11 | `scripts/tests/test_locale_census.py` `TheWebTree.test_the_web_apps_catalogs_are_whole` | added |
| 12 | A12 | `scripts/tests/test_locale_census.py` `TheWebCensus.test_an_unreadable_settings_file_is_refused` | added |

## The reds and greens

```red-first
A1: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['missing zh-Hans summary', 'examined 7 locales, 4 keys'], '')
A1: green at 60750d67
A2: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['empty fr title', 'examined 7 locales, 4 keys'], '')
A2: green at 60750d67
A3: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['placeholder fr progress', 'examined 7 locales, 4 keys'], '')
A3: green at 60750d67
A4: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['stale ko extra', 'examined 7 locales, 4 keys'], '')
A4: green at 60750d67
A5: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['not-nfc fr summary', 'examined 7 locales, 4 keys'], '')
A5: green at 60750d67
A6: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 6 locales, 4 keys'], '') != (1, ['locale-set locales ["en", "zh-Hans",[61 chars], '')
A6: green at 60750d67
A7: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 6 locales, 4 keys'], '') != (1, ['file-missing fr', 'examined 6 locales, 4 keys'], '')
A7: green at 60750d67
A8: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['not-text fr close', 'examined 7 locales, 4 keys'], '')
A8: green at 60750d67
A9: not red: the control: a complete planted tree reads green under the stub and under the census; rows S57101 to S57108, S57110 and S57111 are killed by it
A10: red at 81ad67da: AssertionError: Tuples differ: (0, ['examined 0 locales, 0 keys'], '') != (3, ['file-missing en', 'file-missing es',[135 chars], '')
A10: green at 60750d67
A11: red at 790b5fda: 48 finding lines, the first three: ['unreadable es repeats score_title', 'unreadable es repeats score_pillars', 'unreadable es repeats pillar_consistency']
A11: green at fba8974c
A12: red at 4124bdba: AssertionError: Tuples differ: (1, [], 'Traceback (most recent call last)[1184 chars])\n') != (3, ['unreadable settings not-json', 'exam[24 chars], '')
A12: green at 3d535be3
```

## How each red was read

- At 81ad67da the module ran 11 tests: A9 and A11 passed, and A1 to A8 and A10 failed by their own
  assertions in 30 subtests, every one with empty stderr, so the stub refused nothing and crashed
  on nothing. Each line above quotes the first failing subtest of its test. A3's reordered
  placeholders subtest passed under the stub, as it must under the census.
- At 60750d67 the module ran 11 tests: A1 to A10 passed, and A11 failed by its finding lines,
  `missing` 100 times in each of the six locales other than English and no other kind.
- After each locale's commit the census listed no `missing` line for that locale: A11's finding
  lines fell to 500, 400, 300, 200 and 100, and at 483661ff the module ran 11 tests, OK, with the
  census printing `examined 7 locales, 315 keys` and no finding.
- `scripts/tests/test_locale_census.py` is byte-identical from 81ad67da to the first branch's head
  (`git diff --quiet 81ad67da d9c87dd8 -- scripts/tests/test_locale_census.py`), so no test changed
  between its red and its green.
- From 81ad67da to the commit before the rows, `scripts/locale_census.py` was in the tree and not
  yet named in `scripts/mutation-python.json`, so the committed-map test
  `test_mutation_python.TheMapCensusRefuses.test_the_committed_map_passes_and_prints_its_count`
  would read that file as omitted there. The rows commit names it, and that test passes at it.

## The rows, proved by hand

Each row of `scripts/mutation-rows.d/S57100-S57199.json` was proved on `scripts/locale_census.py`
at sha256 `1d27ab3357ca264233001155e4c431319bcf8a79a02994953da1b8b6648523cc`: its find occurs
once; its replacement was installed by a script; its killer ran by its full name, with a
`PYTHONPYCACHEPREFIX` of its own, and selected exactly one test; the saved bytes were written back
and the sha256 read equal; and the file was touched. Every row read KILLED.

| row | killer | the killer's failing assertion under the mutant |
|---|---|---|
| S57101 to S57107 (each locale) | `TheWebCensus.test_a_planted_complete_tree_reads_green` | `AssertionError: Tuples differ: (1, ['locale-set locales ["en", "zh-Hans",[67 chars], '') != (0, ['examined 7 locales, 4 keys'], '')` |
| S57108 | `TheWebCensus.test_a_planted_complete_tree_reads_green` | `AssertionError: Tuples differ: (1, ['locale-set base "en"', 'examined 7 locales, 4 keys'], '') != (0, ['examined 7 locales, 4 keys'], '')` |
| S57109 | `TheWebCensus.test_a_placeholder_that_differs_from_english_is_refused` | `AssertionError: Tuples differ: (0, ['examined 7 locales, 4 keys'], '') != (1, ['placeholder fr progress', 'examined 7 locales, 4 keys'], '')` |
| S57110 | `TheWebCensus.test_a_planted_complete_tree_reads_green` | `AssertionError: Tuples differ: (1, ['not-nfc en summary', 'not-nfc es sum[203 chars], '') != (0, ['examined 7 locales, 4 keys'], '')` |
| S57111 | `TheWebCensus.test_a_planted_complete_tree_reads_green` | `AssertionError: Tuples differ: (3, ['locale-set pattern "./messages/{loca[184 chars], '') != (0, ['examined 7 locales, 4 keys'], '')` |
| S57112 | `TheWebCensus.test_a_tree_with_no_catalog_is_void` | `AssertionError: Tuples differ: (0, ['file-missing en', 'file-missing es',[135 chars], '') != (3, ['file-missing en', 'file-missing es',[135 chars], '')` |

## The filled messages, for the fluent reader's review

The review of meaning is tracked on #1053. Each of the six catalogs gained the same 100 keys,
appended in English's order after its existing members, every existing line kept but the old last
member, which gained its comma (`git diff --numstat` read `101 1` for each):

| locale | commit | keys filled |
|---|---|---|
| `es` | 1c243e1e | 100 |
| `fr` | 4b02fdda | 100 |
| `ja` | 84aac0e6 | 100 |
| `ko` | f0c156bd | 100 |
| `zh-Hans` | 8fb210ee | 100 |
| `zh-Hant` | 483661ff | 100 |

The keys, in English's order:

`study_day`, `loading`, `reopen_from_telegram`, `server_unavailable`, `about`, `about_title`,
`about_license`, `privacy_policy`, `source_code`, `back_to_today`, `score_title`, `score_none`,
`score_pillars`, `pillar_consistency`, `pillar_retention`, `pillar_workload`, `pillar_volume`,
`pillar_mastery`, `retention_absent`, `level_title`, `level_progress`, `level_progress_text`,
`level_today`, `level_today_none`, `level_settling`, `level_source_reviews`,
`level_source_reviews_law`, `level_source_studied`, `level_source_backlog_zero`,
`level_source_streak`, `level_source_score90`, `level_source_graduations`,
`level_source_consistency`, `level_source_ascendant`, `level_run_title`, `level_run_days`,
`level_multiplier`, `level_one_miss`, `level_ascendant`, `streak_title`, `streak_language`,
`streak_law`, `streak_days`, `streak_best`, `streak_freezes`, `streak_at_stake_freeze`,
`streak_at_stake_break`, `streak_calendar`, `streak_marker_skip`, `streak_marker_freeze`,
`streak_marker_break`, `governor_armed`, `governor_standby`, `governor_lapse`, `insights_title`,
`insights_none`, `insights_no_report`, `insights_failed_read`, `insights_run`, `insights_running`,
`dark_fields_title`, `dark_fields_none`, `dark_fields_cold`, `dark_fields_item`,
`dark_fields_more`, `dark_fields_unparseable`, `capture_title`, `capture_text_label`,
`capture_text_hint`, `capture_journal`, `capture_save`, `capture_saving`, `capture_saved`,
`capture_failed`, `remote_title`, `remote_intro`, `remote_review_open`, `remote_review_close`,
`remote_status`, `remote_side`, `remote_visibility`, `remote_lock`, `remote_refusal`, `remote_key`,
`remote_code`, `remote_none`, `remote_gamepads`, `remote_no_gamepad`, `remote_gamepad`,
`remote_mapping`, `remote_pressed`, `remote_axes`, `remote_log`, `remote_log_caption`,
`remote_log_time`, `remote_log_source`, `remote_log_raw`, `remote_log_action`,
`remote_log_visibility`, `remote_log_lock`

Each value keeps English's placeholders as a multiset, is in NFC, and uses the terms its catalog
already used for the same words (the streak, the review, the card that graduated, the source of
XP, the law track, the remote and the gamepad). French puts a no-break space before `:`, `;`, `?`
and `!`, as its catalog does; Japanese and both Chinese scripts take full-width colons, commas
and brackets. Five French values equal their English text because the French word is the same:
`pillar_volume` (Volume), `remote_code` (Code), `remote_axes` (Axes), `remote_log_source`
(Source) and `remote_log_action` (Action). No value adds pressure, countdown or loss wording that
English does not carry. Four of the keys are accessible names: `level_today` is the same message
as its visible heading; `level_progress` names a meter whose visible text, `level_progress_text`,
uses the same words for the next level in every locale; and `score_pillars` and `streak_calendar`
name a list and a calendar that carry no visible label of their own.

## The rebuild on current dev

This delivery was rebuilt on current dev. Its first commit (790b5fda) merges the first branch with
every one of its commits unchanged, and its tree equals the tree `git merge-tree --write-tree`
computes for dev and that branch's head. Since the first branch's base, English had gained 65
messages and lost one, `study_card_escaped`, which left every catalog in the merge. dev's six other
catalogs had gained 35 of those 65 and eight of the first fill's own messages (`score_title`,
`score_pillars`, the five `pillar_` messages and `retention_absent`). The other 30 were in no other
catalog: the 28 `card_health_` messages of the card information panel and the answer labels
`answer_hard` and `answer_easy`.

- At the merge, A11 read red with 48 finding lines, each `unreadable <locale> repeats <key>`, eight
  per catalog.
- 29533b03 dropped the first fill's copy of the eight repeated messages and kept dev's lines byte for
  byte. It also removed the spaces the first fill had typed between a CJK character and a Latin
  letter, a digit, a placeholder or a percent sign: 36 in 22 Japanese values, 28 in 18 Simplified
  Chinese values and 28 in 18 Traditional Chinese values. The stylesheet's `text-autospace` draws
  that gap. A11 then read 180 finding lines, each `missing <locale> <key>`, 30 per catalog.
- fba8974c filled those 30 messages in each of the six catalogs, appended in English's order with no
  typed space at a CJK and Latin boundary, and A11 read green, the census printing
  `examined 7 locales, 379 keys` and no finding.
- A12 was committed over the census as the first branch left it (4124bdba) and read red by its own
  assertion in both subtests, the census stopping on a traceback; 3d535be3 made the census refuse
  both settings files by name, with the test unchanged.
- Before the rebuild, A11 read red at 60750d67 with 600 finding lines and green at 483661ff; the
  A11 lines above now name the rebuild's red and green.
- A run of the whole test module at 81ad67da, over that commit's census and test module, fails A6
  first in its subtest `six locales`, and A6's red line above quotes that failure. The first branch's record quoted its subtest `another base` instead.

The greens the first branch recorded were measured on the code and catalogs its own pull request's
CI ran at d9c87dd8; each of these exits 0:

- `git diff --quiet 60750d67 d9c87dd8 -- scripts/locale_census.py scripts/tests/test_locale_census.py`
- `git diff --quiet 483661ff d9c87dd8 -- scripts/locale_census.py scripts/tests/test_locale_census.py web/app/messages web/app/project.inlang`

The run at this delivery's head reads every criterion green by name. A11's and A12's greens name
commits that run did not run on their own; each of these exits 0 at the head:

- `git diff --quiet fba8974c HEAD -- scripts web`
- `git diff --quiet 3d535be3 HEAD -- scripts/locale_census.py scripts/tests/test_locale_census.py`
- `git diff --quiet 4124bdba HEAD -- scripts/tests/test_locale_census.py`

The rebuild's filled messages, for the fluent reader's review (tracked on #1053): each of the six
catalogs gained the same 30 keys in fba8974c, in this order: `card_health_open`, `card_health_title`,
`card_health_close`, `card_health_unreadable`, `card_health_reviews`, `card_health_lapses`,
`card_health_time_total`, `card_health_time_mean`, `card_health_time_median`,
`card_health_seconds`, `card_health_no_answers`, `card_health_logged`, `card_health_recall_title`,
`card_health_estimates`, `card_health_recall_chance`, `card_health_days_to_90`,
`card_health_absent`, `card_health_fixture`, `card_health_unready`, `card_health_stale`,
`card_health_origin_title`, `card_health_origin_collection`, `card_health_history_title`,
`card_health_history_shown`, `card_health_col_date`, `card_health_col_answer`,
`card_health_col_time`, `card_health_manual`, `answer_hard` and `answer_easy`. Each value keeps
English's placeholders as a multiset, is in NFC and uses the terms its catalog already used.
