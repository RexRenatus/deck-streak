# SPEC-614 red-first record

A1 to A9 are red at the red commit, which adds the tests beside an inert `scripts/formal_ci.py`
that defines every function and verb the tests import, finds nothing and reads no file, and no
`.github/workflows/formal.yml`. Each red is an assertion on the missing behaviour: an examined
count the inert checker never prints, a finding it never reports, an audit file it never writes,
or the workflow file that does not exist yet. Each is green at the commit that adds the checker,
the stale list and the workflow. A4 reads the live tree's covers through the checker, so its green
is read in CI's run of the green push, on a tree whose `scripts`, `config` and `.github` equal the
green commit's.

```text
A1: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_a_moved_span_reads_stale_by_entry_path_and_anchor
A2: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_an_anchor_absent_or_defined_twice_is_unresolved
A3: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_each_reader_reproduces_a_digest_written_by_the_rule
A4: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_every_cover_at_head_is_fresh_or_listed_exactly
A5: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_a_listed_row_fresh_undeclared_or_moved_again_is_a_finding
A6: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_an_axiom_outside_the_allowed_set_or_sorry_is_refused
A7: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_audit_source_names_every_declared_theorem_and_witness_once
A8: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_formal_workflow_runs_its_checks_in_order_on_every_pull_request_and_push
A9: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_lean_install_is_the_toolchain_pin_checksum_verified
```
