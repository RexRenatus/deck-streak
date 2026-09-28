# Red-first record: SPEC-060

The SPEC was promoted, ADR-060 accepted and the tools' schematic drawn alone (1a75948). The tests of
A1 to A8, and two beyond the criteria, were committed next (3bd0f96), beside stubs of the three
tools that held the designs ADR-060 rejected:
- the inventory ran its reads and the health checks as they came, with no allow list and no `nice`,
  and measured nothing of the disk;
- the plan listed every file under every root, as a checklist would, with no rule, reason or digest;
- the apply deleted every item the list held, with nothing tying the owner's approval to what went.

Each criterion was run there with the SPEC's own fenced command. Each selected exactly one test and
failed by assertion, not by an error or an empty selection, each for its own reason but A6's
(below), and each red was observed again on an export of 3bd0f96. A8 is a census, honestly red
because its positive artifact, the example rules, was not there yet. A2's failure compared the
root's unmeasured space, 0, with the size of the scratch file system, a figure of the machine that
is not quoted here.

A6's refused approvals shared one file name at 3bd0f96, so each case read the last one written, and
that test could not pass against any implementation: its failure there is not a red for A6's
reason. 3ed706a gave each its own file, and A6 was red there by assertion on the stubs, so the fence
records A6's red at 3ed706a. The implementation followed (ceaa1b9): the three tools, the example
rules and the runbook, and every criterion was green there.
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
A6: red at 3ed706a: AssertionError: {...} != {...} (an approval that names no snapshot, and the apply deleted the item)
A6: green at ceaa1b9
A7: red at 3bd0f96: AssertionError: {...} != {...} (an item under a protected path was deleted)
A7: green at ceaa1b9
A8: red at 3bd0f96: AssertionError: Items in the second set but not the first: 'rules.example.json' (the example rules were not there)
A8: green at ceaa1b9
A9: red at 6a9abea: AssertionError: 0 != 1 : apply: deleted i005 (...) (the apply read rules the inventory never read, and the item went)
A9: green at 499dee8
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
each tool's own `os.nice`, which no test observes. Those mutants did not cover every check that
stands before a deletion: the review of #289 planted implementations the module let through, and
the mutants above cannot be replayed. The rows below replace that claim.

## Fix round

The review of #289 asked the apply to refuse a path it does not read canonically, to run no health
check outside the read allow list, and to delete nothing it did not check, and it asked for cases
that separate the implementations its plants showed the tests could not tell apart. The order of
work, each red committed before its fix:

- 62ce3dd: the allow list's `is-active` form, red. The fixture's health check names its unit after
  `--`, and A1 plants reads whose units are not written after `--` or do not begin with a letter or
  a digit, and a changing verb in the allowed form. The head refused the fixture's form, so every
  test that runs the inventory was red there: A1 and A2 with `AssertionError: 1 != 0` (the
  inventory refused its own health check), A3 to A7 and the health test with `the inventory
  failed`; A8 and the repository test stayed green. 4e9daac made the allow list require the `--`
  and a unit name that begins with a letter or a digit, and every test was green again.
- 6a9abea: every other case, red where the code was wrong and green on arrival where it was right.
  Then the fixes: 6566b2f, the tools refuse a path they do not read canonically; 499dee8, the apply
  runs its health checks through the read allow list only, from the rules the list names; 17d604d,
  each deletion reads its item again, through directories opened without following a link;
  d369557, a snapshot dated after the apply's clock is refused; b177cbd, a JSON file that holds a
  key twice is refused. Every test was green at b177cbd.
- 5480568: SPEC-060, ADR-060, the runbook, the schematic and the fragment; 8843ceb: the rows.

Each case is a subtest, or a block of its own, so each one's state was read at every commit:

```text
A3 a candidate an inventory names in a form the plan does not read canonically: red at 6a9abea (it was listed), green at 6566b2f
A4 a list that holds a key twice: red at 6a9abea (every approved item went), green at b177cbd
A5 a file changed at the same size, its modification time put back: green on arrival at 6a9abea
A5 an entry added inside an approved directory: green on arrival at 6a9abea
A5 an item replaced after the apply's checks: red at 6a9abea (the replacement went), green at 17d604d
A6 a snapshot before the inventory, written in another offset: green on arrival at 6a9abea
A6 a snapshot after an older inventory, written in another offset: green on arrival at 6a9abea
A6 a snapshot dated later than the apply's clock: red at 6a9abea (its item went), green at d369557
A7 an item under, or holding, a protected path, in a list the plan did not make: green on arrival at 6a9abea
A7 a pattern given as a protected path: green on arrival at 6a9abea
A7 an item's path in each of four forms the apply does not read canonically: red at 6a9abea (three reached what they named and deleted it; the fourth was refused as protected, not by its form), green at 6566b2f
A7 rules that write a path in a form the tools do not read canonically: red at 6a9abea (the inventory read them), green at 6566b2f
A7 a protected path given as a link: green on arrival at 6a9abea
A7 a sibling whose name only begins with a protected path's: green on arrival at 6a9abea
A7 an item that is itself a link, and a link inside a directory item: green on arrival at 6a9abea
A7 an item reached through a symbolic link: green on arrival at 6a9abea
A7 a directory above an item that becomes a link after the apply's checks: red at 6a9abea (the item's path was deleted), green at 17d604d
A9 rules other than the ones the inventory read: red at 6a9abea (the item went), green at 499dee8
A9 a changing command given as a health check, in rules the list names: red at 6a9abea (it ran, and the item went), green at 499dee8
```

The cases green on arrival pin implementations the review planted and the module had not told
apart: a digest over no content, a directory's digest not checked again, instants compared as text
or as wall clocks, a protected link's target left unresolved, a name's prefix taken for a path, and
an item that is a link followed.

DISCLOSURE, A1 (`test_the_inventory_runs_only_its_read_only_allow_list`): its body changed at
62ce3dd, after its green commit: four planted shapes joined its two. Red at 62ce3dd, green at
4e9daac.

DISCLOSURE, A3, A4, A5, A6 and A7: each body changed at 6a9abea, after its green commit, with the
cases above. A6 and A7 run their cases as subtests, and A7's cases under a protected path use a list
the plan did not make, since the apply now reads only the rules the inventory read. Each is green at
b177cbd. A2's and A8's bodies did not change.

DISCLOSURE, `test_a_health_check_red_after_an_apply_stops_the_scrub`: its body changed at 62ce3dd,
whose health check names its unit after `--`. Red there, green at 4e9daac.

DISCLOSURE, the delivery before the fix round: bodies that changed after their green commit,
ceaa1b9, measured by hashing each test's source at every commit. A3, A4, A5, A6 and A8 changed at
57652b4 and A3 again at de71b3c, with the cases the paragraph above names; A8's change reads the
example rules through the tools' own reader, which accepts them. A2's changed at e67aec8, where the
fixture's in-use unit took a plain name and A2 reads that unit by it; its assertions are unchanged.
A7's changed at 6d513bb, with the pattern case red at 82bbaa9. A1's did not change.

DISCLOSURE, the module's helpers changed at 62ce3dd (the fixture's health check names its unit
after `--`) and at 6a9abea (an approval's snapshot is dated when the approval is written; a list the
plan did not make is written with its own digest taken again; a tool may run in a given directory;
a digest line may carry a path as a list writes it; a health check's address can change the tree
while the apply reads it).

## Rows

`scripts/mutation-rows.d/S06000-S06099.json` holds 49 rows, S06001 to S06049, one for each check
that stands before a deletion. Each has an anchor that occurs exactly once, one mutant, and a killer
that selects one test. `python3 scripts/mutation_rows.py prove --band S06000-S06099` proved
them at 8843ceb, on the committed tree: `rows: examined 49: killed 49, survived 0, void 0`. Each
killer selected one test and passed without its mutant, and each target was restored byte for
byte, checked by its sha256.

| the checks | rows | killer |
|---|---|---|
| the list's own digest; the approval's list digest, approver, date and ids | S06001, S06003 to S06006 | A4 |
| the rules the list names; health checks through the read allow list | S06002, S06013 | A9 |
| the snapshot: named, with an offset, after the inventory as an instant, not after the clock | S06007 to S06012 | A6 |
| every item checked before the first deletion; the dry run by default | S06014, S06027 | A5 |
| an item's path canonical, not protected, not reached through a link | S06015 to S06017 | A7 |
| an item's digest computed again and compared; a package's digest and dry removal | S06018 to S06022 | A5 |
| each deletion: no link above, its item read again, a link inside or an item that is a link not followed | S06023 to S06026 | A5, A7 |
| a red health check stops the scrub | S06028 | the health test |
| the allow list: a unit's name, `--`, the one verb, the runner's refusal, checked before any runs | S06029 to S06033 | A1 |
| the rules: a canonical path, by its key; a protected pattern refused | S06034 to S06038 | A7 |
| a JSON key held twice | S06039 | A4 |
| a service's own rotation never listed | S06040 | A2 |
| the plan: a canonical path, within a boundary, holding, a protected link, the rules it read, an environment in use, an item inside another | S06041 to S06049 | A3, A7 |

Not held by a row, each with why: the Python-version guard for a directory item (no interpreter the
tests run under tells it apart); a directory removed through its parent's descriptor rather than by
path (the two differ only inside a window no test holds open); a package's removal run through a
shell with the same argument vector (no different argument can reach it while the package's name
is validated); and an item found gone since the list was made (nothing is left to delete).
