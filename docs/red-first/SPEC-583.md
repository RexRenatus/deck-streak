# SPEC-583 red-first record

A1, A3, A5, A8 and A10 are red at the red commit, which adds the test module alone, and green at the commit that
installs the records each one reads. A2, A4, A6, A9 and A11 plant their refusal over a fixture the test spells (a
planted table, census or text, or a temporary tree), beside a control that reads green, so each is `not red`; A11's
reds are its mutation rows. Every line below was measured locally by running the test module by its path
(`python3 -B scripts/tests/test_sealed_sync_records.py -v`) on a tree byte-identical to the commit it names.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_key_table_names_four_keys_and_the_server_holds_none_in_the_clear
A3: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_clear_head_holds_exactly_the_judges_fields
A5: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_daily_record_holds_exactly_four_fields
A8: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_census_covers_every_ingest_context_and_every_table
A10: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_staged_privacy_text_names_what_is_sealed_and_what_is_kept
```

```red-first
A1: red at e50f8e4e9ad8057f460d0db1244d8633a826e98d: AssertionError: Lists differ: ['no-key-table: SPEC-583 has no key table'] != []
A1: green at aa06456d7ba4604c2f2a951b96b8b8ffccdbf953
A2: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A3: red at e50f8e4e9ad8057f460d0db1244d8633a826e98d: AssertionError: Lists differ: ['no-head-table: SPEC-583 has no head table'] != []
A3: green at aa06456d7ba4604c2f2a951b96b8b8ffccdbf953
A4: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A5: red at e50f8e4e9ad8057f460d0db1244d8633a826e98d: AssertionError: Lists differ: ['no-daily-table: SPEC-583 has no daily-facts table'] != []
A5: green at aa06456d7ba4604c2f2a951b96b8b8ffccdbf953
A6: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A8: red at e50f8e4e9ad8057f460d0db1244d8633a826e98d: AssertionError: Lists differ: ['no-census: docs/sealed-sync/device-census.md is absent'] != []
A8: green at aa06456d7ba4604c2f2a951b96b8b8ffccdbf953
A9: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A10: red at e50f8e4e9ad8057f460d0db1244d8633a826e98d: AssertionError: Lists differ: ['no-staged-text: docs/sealed-sync/privacy-at-the-cut-over.md is absent'] != []
A10: green at aa06456d7ba4604c2f2a951b96b8b8ffccdbf953
A11: not red: the privacy page claims no sealed sync at dev; its reds are rows S58313 and S58314
```
