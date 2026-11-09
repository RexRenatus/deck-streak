# SPEC-651 red-first record

A1, A2 and A3 are red, each on an assertion, at the commit that adds `scripts/tests/test_product_docs.py` with the four
documents unchanged, and green at the commit that adds the notes and marks, with the test unchanged. Each red and green
line below was measured locally, on a clean tree at the commit it names, by its criterion's command.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_every_section_naming_a_telegram_surface_says_deprecated
A2: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_each_product_doc_holds_one_dated_adr_589_note
A3: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_each_campaign_spec_keeps_its_text_and_gains_one_dated_note
```
