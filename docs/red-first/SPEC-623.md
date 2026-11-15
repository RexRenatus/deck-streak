# SPEC-623 red-first record

The criteria were written as one test module before either page changed. Six of them read red
at the commit that adds the tests alone, each an assertion failure that names what it found: the
crate folders the code map lacks, the missing `ios/` row, the units of each page that name a
Telegram surface without the word deprecated, the Telegram lines of the drawing, and the
`ios-harness` row that names only the harness. Two are `not red`, each with its planted refusal
inside the test: they guard what dev already holds, every code-map path and every fence path row
in the tree or planned.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_code_map_names_every_crate_folder_exactly_once
A2: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_code_map_has_a_row_for_the_iphone_and_ipad_app_folder
A3: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_every_code_map_path_exists_or_is_planned_in_the_context_map
A4: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_architecture_names_a_telegram_surface_only_beside_deprecated
A5: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_drawing_shows_the_two_apps_and_no_telegram_surface
A6: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_context_map_names_a_telegram_surface_only_beside_deprecated
A7: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_context_maps_ios_row_names_the_app_and_the_harness
A8: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_every_context_map_path_row_exists_or_is_planned
```

```red-first
A3: not red: guards what dev already holds; its planted row naming an absent path without planned is refused, and a planned row with its planned fence row is passed, before and after the pages change
A8: not red: guards what dev already holds; its planted path row for an absent path without planned is refused, and one that says planned is passed, before and after the pages change
```
