# Red-first record: SPEC-060

The SPEC was promoted, ADR-060 accepted and the tools' schematic drawn alone (1a75948). The tests of
A1 to A8, and two beyond the criteria, were committed next (3bd0f96), beside stubs of the three
tools that held the designs ADR-060 rejected:
- the inventory ran its reads and the health checks as they came, with no allow list and no `nice`,
  and measured nothing of the disk;
- the plan listed every file under every root, as a checklist would, with no rule, reason or digest;
- the apply deleted every item the list held, with nothing tying the owner's approval to what went.

Each criterion was run there with the SPEC's own fenced command. Each selected exactly one test and
failed by assertion for its own reason, not by an error or an empty selection, and each red was
observed again on an export of 3bd0f96. A8 is a census, honestly red because its positive artifact,
the example rules, was not there yet. A2's failure compared the root's unmeasured space, 0, with the
size of the scratch file system, a figure of the machine that is not quoted here.

A6's refused approvals shared one file name at 3bd0f96, so each case read the last one written;
3ed706a gave each its own file, and A6 stayed red on the stubs. The implementation followed
(ceaa1b9): the three tools, the example rules and the runbook, and every criterion was green there.
After the green, the cases the mutants below needed were added, each green on arrival: an approval
without its approver, its date or ids the list holds, an edited list, a snapshot at the inventory's
instant or without an offset, rules the inventory did not read, and a package end to end (57652b4);
then two links to one file listed together, a note inside a listed worktree, and a proxy the
environment names (de71b3c). SPEC-060's amendments and ADR-060's delivery decisions followed
(8523555), and the fixture's in-use unit was given a plain name (e67aec8). Last, a pattern given as
a protected path was refused, test first: its case in A7 was red at 82bbaa9, where the apply deleted
the approved item the pattern was meant to protect, and green at 6d513bb.

```red-first
A1: red at 3bd0f96: AssertionError: Lists differ: [] != [['dpkg-query', '-W'], ['ps', '-eo', ...]] (no command ran under nice and ionice -c3)
A1: green at ceaa1b9
A2: red at 3bd0f96: AssertionError: 0 != the scratch file system's size (the root's space was never measured)
A2: green at ceaa1b9
A3: red at 3bd0f96: AssertionError: Items in the first set but not the second (the plan listed files no rule selects)
A3: green at ceaa1b9
A4: red at 3bd0f96: AssertionError: {...} != {...} (with no approval, the apply deleted every listed file)
A4: green at ceaa1b9
A5: red at 3bd0f96: AssertionError: {...} != {...} (an approved item changed after the list was made, and the apply deleted it)
A5: green at ceaa1b9
A6: red at 3bd0f96: AssertionError: {...} != {...} (an approval that names no snapshot, and the apply deleted the item)
A6: green at ceaa1b9
A7: red at 3bd0f96: AssertionError: {...} != {...} (an item under a protected path was deleted)
A7: green at ceaa1b9
A8: red at 3bd0f96: AssertionError: Items in the second set but not the first: 'rules.example.json' (the example rules were not there)
A8: green at ceaa1b9
```

Two tests beyond the criteria were written with them, red at 3bd0f96 and green at ceaa1b9, and carry
no line above: `test_a_health_check_red_after_an_apply_stops_the_scrub` pins R9's stop (red:
`AssertionError: {} != {'example-service': True, 'example-ready': True}`, no health read before the
inventory), and `test_no_tool_writes_its_output_inside_the_repository` pins R8's refusal (red:
`AssertionError: 1 != 2`, the stub tried to write inside the repository).

## Mutants of the changed code

No mutation runner generates mutants of this repository's Python (#218), so the changed code's
mutants were proved by hand at 6d513bb, on the committed tree. Each mutant replaced an anchor that
occurs exactly once, and its killer, one test selected alone, was green on the unmutated file and
red on the mutant. The file was restored byte for byte after each, checked by its sha256. All 63
were killed:

| requirement | mutant | killed by |
|---|---|---|
| R1 | the record of the commands run dropped | A1 |
| R1 | a hard-linked file counted per link; every entry's size counted, not only files'; the worktree kinds swapped; the virtual environment's marker misnamed; an environment walked twice rather than counted once; a sizes-only root listing its entries; the space's total read from free blocks; the mount holding a root dropped | A2 |
| R2 | the allow list skipped; the health commands not checked before the first runs; `nice` and `ionice` dropped; `ionice` dropped; the wrapper bypassed; `systemctl` admitted with any verb | A1 |
| R3 | a day retention's cutoff inverted; the copies ordered oldest first | A2 |
| R3 | a unit that is not loaded read as naming a path; the working directory not read; the name match inverted | A3 |
| R4 | the mode, the modification time, the size or the content left out of a digest; a directory's lines unsorted; a file whose other links survive counted as freed; two listed links to one file counted twice; the list's digest taken over nothing; rules the inventory did not read accepted | A3 |
| R6 | a list whose own digest no longer matches its content; no approval; an approval without the list's digest, the approver, the date, or ids the list holds | A4 |
| R6 | a digest not computed again; the dry run skipped; the item checks skipped; a package's digest not computed again; a package's dry removal ignored; a directory unlinked rather than removed | A5 |
| R6 | a snapshot with no name; a snapshot at the inventory's instant; a snapshot before it; an instant with no offset | A6 |
| R7 | an item under a protected path listed | A3 |
| R7 | an item holding a protected path; a pattern accepted as a protected path; the apply's protected check skipped; a link above an item not checked; the link check stopping short of the item's own directory | A7 |
| R8 | the repository check answering no; the plan's refusal skipped; the apply's refusal skipped | `test_no_tool_writes_its_output_inside_the_repository` |
| R9 | a command check's verdict inverted; a GET's verdict inverted; no health read before the inventory; a GET sent through the environment's proxy; the turned checks left out; a red check exiting 0 | `test_a_health_check_red_after_an_apply_stops_the_scrub` |
| R10 | the service's own rotation ignored | A2 |
| R10 | a candidate inside a listed item listed again; a package purged rather than removed | A3, A5 |

Not mutated: the rules file's shape checks (`load_rules`, `check_rule`, `check_health`), the
parsers' tolerance of a malformed line, the runner's answers to a missing command and a timeout, and
each tool's own `os.nice`, which no test observes. The mutants cover every check that stands before
a deletion and every figure a criterion reads.
