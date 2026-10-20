# SPEC-493 red-first record

A1 to A5 are red at the red commit, before the code, and green after it. The test modules run in
CI, so every red and green line is quoted from the pull request's checks, from the job that runs
`scripts/tests`, by the round that reads them, and joins the `red-first` fence below.

The red commit is `cd1827d3`, which holds the test modules alone; the code lands at `d3f0951a`. CI
run 39394179900 ran at the red commit, and each red line below was read there by test name: 92
failure and error lines, all from the two new modules. Each green line names the greening commit
and is confirmed at the push-2 CI read. The reds are the new module's absence:
`scripts/mutation_carry.py`, its one `scripts/mutation-verdict.py carry` hook, and `ci.yml`'s carry
steps do not exist at `cd1827d3`, so the tests that import or read them fail.

Three edits sit between the red tests and the code, each a fix to a red test or to the new module
and none a weakening of a shipped behaviour:

- `test_no_base_step_can_fail_the_plan` (A3, `Workflow`) compared a download step's first line, its
  `- ` stripped, against pinned lines that keep the step, so no workflow could pass; it now compares
  the pinned action text after `uses: `.
- `test_the_manifest_is_kept_when_a_leg_fails` (A5, `Manifest`) passed `shard` twice to one
  `report(...)` call; it now spreads the call and sets `shard` once,
  `{**report(1, 2, {...}), "shard": "0/2"}`.
- `scripts/mutation_carry.py`'s read-set builder (A2, `read_set`) read `scripts/a.py`'s read set as
  the whole tree: its `.parents` subscript gate tested a dotted name and so missed
  `Path(__file__).resolve().parents[1]`, whose chain roots in a call, leaving the `sys.path.insert`
  argument unevaluable; the gate now tests the subscript's value for a `.parents` attribute, so the
  argument evaluates and the read set is bounded to the file and its import.

After the code, one test-only commit lets two house censuses that read every test module read
`test_mutation_carry.py`, and changes no assertion: the module imports `BaseHTTPRequestHandler` and
`ThreadingHTTPServer` from `http.server` by name, `Workflow` reads `ci.yml` in `setUp` instead of a
class-level hook, and the fake forge's silenced request log no longer shares its body with a helper
of the command finder. Run 39394179900 read those two censuses red on this module, in five failure
lines outside A1 to A5.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k UnionRule
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_carry_census.py -k ReadSet
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_carry_census.py -k PlantedControls
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k BaseSelection
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Workflow
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Key
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k FailSafe
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k JudgedAtHead
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Manifest
```

```red-first
A1: red at cd1827d3: read by name on CI run 39394179900, 7 failure lines (scripts/mutation_carry.py absent at the red commit, so UnionRule cannot import it)
A1: green at d3f0951a
A2: red at cd1827d3: read by name on CI run 39394179900, 38 failure lines (scripts/mutation_carry.py absent, so ReadSet and PlantedControls cannot import it)
A2: green at d3f0951a
A3: red at cd1827d3: read by name on CI run 39394179900, 24 failure lines (scripts/mutation_carry.py and ci.yml's carry steps absent, so BaseSelection and Workflow fail)
A3: green at d3f0951a
A4: red at cd1827d3: read by name on CI run 39394179900, 18 failure lines (scripts/mutation_carry.py absent, so Key and FailSafe cannot import it)
A4: green at d3f0951a
A5: red at cd1827d3: read by name on CI run 39394179900, 5 failure lines (scripts/mutation_carry.py and the mutation-verdict.py carry hook absent, so JudgedAtHead and Manifest fail)
A5: green at d3f0951a
```
