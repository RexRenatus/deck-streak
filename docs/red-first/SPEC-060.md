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
A10: red at 7ae7dbc: AssertionError: False is not true : apply: deleted x001 (...), 0 bytes (the apply parsed rules without a protected path, bound the list's rules by a second read, and the item that path protects went)
A10: green at 28df6ac
A11: red at f5ae5c8: AssertionError: None is not True (the inventory did not record the clock, and an unsynchronised clock refused nothing; the crossed-device test failed with it)
A11: green at 5342bc0
A12: red at c418299: AssertionError: Lists differ: [{...}] != [] (an item that is a mount point, or holds one, was listed, and the apply's check let it through)
A12: green at 8ea04d1
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

`scripts/mutation-rows.d/S06000-S06099.json` holds 67 rows, S06001 to S06067, one for each check
that stands before a deletion. Each has an anchor that occurs exactly once, one mutant, and a killer
that selects one test. `python3 scripts/mutation_rows.py prove --band S06000-S06099` proved
the first 49 at 8843ceb, on the committed tree: `rows: examined 49: killed 49, survived 0, void 0`,
and all 54 in the second fix round (below). Each killer selected one test and passed without its
mutant, and each target was restored byte for byte, checked by its sha256.

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
| a file a tool parses and binds, read once: the reader's digest, the inventory's record, the plan's rules check and its inventory's digest, the apply's rules check | S06050 to S06054 | A10 |

Not held by a row, each with why: a file system mounted inside an item, crossed for real (unprivileged
mounts are blocked here, so the device test fakes `st_dev` in the walk, the mount test fakes the
mount table, and no run crosses a real mount); the Python-version guard for a directory item (no interpreter the
tests run under tells it apart); a directory removed through its parent's descriptor rather than by
path (the two differ only inside a window no test holds open); a package's removal run through a
shell with the same argument vector (no different argument can reach it while the package's name
is validated); an item found gone since the list was made (nothing is left to delete); and the entry
check's modification time beside the deletion's re-measure (the digest carries the modification
time, so the two agree wherever they read one entry, and differ only where the path the re-measure
reads and the parent's descriptor name different entries, the interval SPEC-060 §7 discloses, which
no run can hold open).

## Fix round 2

The fix round bound the apply to the rules the list names, but each tool that bound a file by its
digest still parsed the file and then read it again to digest it: the inventory recorded, and the
plan and the apply checked, the digest of a second read, not of the bytes each had parsed, and the
plan named its inventory the same way. A file that changed between the two reads was parsed as one
content and bound as another, so the apply could act on rules the list does not name while its
rules check passed. The order of work, the red committed before its fix:

- 7ae7dbc: A10, and a test beyond the criteria, red. A10 runs each tool through a reader the test
  writes at run time: an audit hook in the tool's own process sees every open of one file, and just
  before the second open the file takes other bytes, in either order. Over the whole test file at
  7ae7dbc only these two tests failed, and the other eleven passed.
- 28df6ac: the fix. `read_json` in `inventory.py` reads a file once and returns its content with the
  SHA-256 of those bytes, `load_rules` carries that digest with the rules, and the tools record and
  check it: the inventory's `rules_digest`, the plan's rules check and its list's inventory digest,
  and the apply's rules check. `file_digest` went with its last caller, and S06047's anchor moved
  with the plan's check it mutates, in the same commit. Every test was green at 28df6ac.
- 6be15a9: the rows, S06050 to S06054; 6e79a2f: SPEC-060's A10 and §8, ADR-060's decision and the
  schematic.

Each case of A10 is a subtest, so each one's state was read at both commits:

```text
A10 the inventory, the rules and other rules each first: red at 7ae7dbc (it recorded the digest of rules it had not read its health checks from), green at 28df6ac
A10 the plan, the rules the inventory read first: red at 7ae7dbc (it refused them, 1 != 0, having bound the second read), green at 28df6ac
A10 the plan, rules naming one more package first: red at 7ae7dbc (it listed that package under the inventory's rules digest), green at 28df6ac
A10 the plan, its inventory and another inventory each first: red at 7ae7dbc (the list named the other inventory's digest), green at 28df6ac
A10 the apply, the rules the list names first: red at 7ae7dbc (it refused them, 1 != 0, having bound the second read), green at 28df6ac
A10 the apply, rules without the protected path first: red at 7ae7dbc (the protected item went while the rules check passed), green at 28df6ac
```

`test_each_tool_opens_each_file_it_binds_once` counts the opens of the rules in each tool, and of
the inventory in the plan, in one run each: red at 7ae7dbc (`AssertionError: 2 != 1` for each of the
four), green at 28df6ac. It stands beside A10 and carries no fence line, as the two tests beyond the
criteria above.

The rows S06050 to S06054 each install a second read where a tool binds a file: in the reader, the
inventory's record, the plan's rules check, the list's inventory digest and the apply's rules check.
Each is killed by A10. With S06047, re-anchored, they were proved at 6be15a9 with
`python3 scripts/mutation_rows.py prove --row ...`, each target restored byte for byte:
`rows: examined 6: killed 6, survived 0, void 0`.
The whole band was proved again at 6e79a2f, whose tools and tests the head keeps, with
`python3 scripts/mutation_rows.py prove --band S06000-S06099`, each killer selecting one test with
and without its mutant: `rows: examined 54: killed 54, survived 0, void 0`.

DISCLOSURE, the module's helpers changed at 7ae7dbc: `Host.run` can run a tool through the reader,
`Host.served` does so and counts the opens, and the module's docstring names the reader. No
criterion's test body changed in this round, and A10's is the same at 7ae7dbc and 28df6ac.

## Fix round 3

The deletion compared the item's entry, so a file rewritten below a directory item kept its entry
and was removed; a file the apply binds was read twice in three places the tests had not opened; and
the tools ran on a clock nobody had checked. The order of work, each red committed before its fix,
the whole test module run at each red commit:

- b83a634: A5's cases (a) to (c), red, and (d) added green; 5434854: the fix, `delete()` measures the item's digest again
  and leaves it when it differs. Every test was green at 5434854.
- 51f0944: A10's three cases (the apply's rules first, the plan's inventory, the apply's list) and
  the open counts of the apply's list and approval. These were green on arrival, because the head
  reads each of these files once: they pin the reads, and rows S06056 to S06058 install the second
  read and are killed by them.
- f5ae5c8: A11, red; 5342bc0: the fix, the inventory records and refuses a clock that does not read
  synchronised, the apply reads it again, and `measure` refuses an entry on another device.

The criterion A11 is new, and its lines are in the `red-first` block above; A5's earlier lines
stand there, so this round's are given here:

```text
A5: red at b83a634: AssertionError: 0 != 3 (an entry added below a directory item, a file rewritten inside it, and a file item rewritten with its mtime put back were each deleted)
A5: green at 5434854
```

Per case, as the subtests read at both commits:

```text
A5 (a) an entry added under the venv: red at b83a634 (deleted), green at 5434854
A5 (b) a file inside the venv rewritten: red at b83a634 (deleted), green at 5434854
A5 (c) a file item rewritten, mtime put back: red at b83a634 (deleted), green at 5434854
A5 (d) a file item rewritten: green on arrival, the entry's mtime moves; kept by the entry check's mtime and the re-measure together, and no single row reds it
A10 the apply's rules first: green on arrival; pinned by S06057
A10 the plan's inventory: green on arrival; pinned by S06058
A10 the apply's list: green on arrival; pinned by S06056
A11 the inventory records the clock: red at f5ae5c8, green at 5342bc0
A11 the inventory refuses an unsynchronised clock: red at f5ae5c8, green at 5342bc0
A11 the apply refuses an unsynchronised clock: red at f5ae5c8, green at 5342bc0
A11 the list carries the clock, the apply refuses a list without it: red at f5ae5c8, green at 5342bc0
A11 an entry on another device is never digested or removed: red at f5ae5c8, green at 5342bc0
```

Unprivileged mounts are blocked on the box, so the crossed-device case replaces `walk` with a seam
that reports one entry on another device; it does not cross a real mount (the first entry of "Not held by a row", in the first fix round's section).

The rows S06055 to S06065: the deletion's re-measure (S06055, killed by A5), the apply's list, the
apply's rules and the plan's inventory each read once (S06056 to S06058, A10), and the clock and
device refusals (S06059 to S06065, A11).

The whole band was proved at the head of this round with
`python3 scripts/mutation_rows.py prove --band S06000-S06099`: 62 killed and 3 void, because the
round's re-measure and the clock line in the plan's list had moved the anchors of S06019, S06020
and S06053. They were re-anchored, and each was proved with `--row`: killed, so 65 of 65 rows
are killed and each target was restored byte for byte, checked by its sha256.

DISCLOSURE, A1 (`test_the_inventory_runs_only_its_read_only_allow_list`): its body changed at 5342bc0, after its green commit: `("timedatectl", "show")` joined the read commands the inventory runs, since the inventory now reads the clock; no other assertion changed.

## Fix round 4

The device check compares each entry's device with the item's own, so it does not see a bind mount,
which shares the tree's device, or a file mounted over a file. The plan and the apply now read the
kernel's mount table and refuse an item that is a mount point or holds one. The order of work:

- merge of the base branch, in its own commit (e36c48b), conflicts only;
- c418299: A12, red, over the whole module, with stubs that compile: only the new test failed, by
  assertion; 8ea04d1: the fix, `plan.mounted` and its reader `plan.read_mountinfo`, used by the
  plan's listing and by the apply's entry check;
- 3eb2997: the rows S06066 and S06067.

```text
A12: red at c418299: AssertionError: Lists differ: [{...}] != [] (an item that is a mount point, or holds one, was listed, and the apply's check let it through)
A12: green at 8ea04d1
```

Per case, as the subtests read at both commits:

```text
A12 a control with no mount point: green at both commits
A12 an item that is a mount point: red at c418299, green at 8ea04d1
A12 an item holding a mount point: red at c418299, green at 8ea04d1
A12 a mount point that only shares an item's name as a prefix: green at both commits (it pins that it is not refused)
A12 a mount point with an escaped space, read unescaped: red at c418299, green at 8ea04d1
A12 a file mounted over a file: red at c418299, green at 8ea04d1
A12 nothing mounted: green at both commits
```

The mount table is faked through the reader seam, since the box that runs the tests refuses
unprivileged mounts; no case crosses a real mount. The rows S06066 (the check for a mount point
strictly under the item, killed by the holding case) and S06067 (the octal unescape, killed by the
escaped case), each proved with its full id: killed.

The statements that were absolute in the delivery were reworded: the apply checks everything before
its first deletion and then measures each item again at its deletion, stopping the run there with
earlier deletions kept, so no text promises a run that either completes or changes nothing. The runbook's order now puts the
list before the snapshot, since the plan reads no snapshot.

## Fix round 5

The fifth review found three gaps in the mount refusal and the criteria around it. The order of
work:

- merge of the base branch, in its own commit (9f5160b), conflicts only;
- a3dcc7a: two edits to A12's test, both green at the head (the plants below are what show them
  red): a subtest for a mount table that cannot be read, and a second escape in the escaped
  fixture's path (`old env two`);
- 1be2f05: the bind-mount subtest of A12, red over the whole module (only A12's test failed, by
  assertion); 19f4d59: the fix, the root-field clause of `plan.mounted` and the reason `plan.mount_reason`
  that the plan's listing and the apply's entry check both give;
- db55b72: the rows S06068 to S06071, each proved with its full id: killed.

```text
A12: red at 1be2f05: AssertionError: Lists differ: [{...}] != [] (an item inside a bind mount was listed, and the apply's check let it through)
A12: green at 19f4d59
```

Per case, as the subtests read:

```text
A12 a mount table that cannot be read refuses the item: not red (the refusal existed; the plants of S06068 and S06069 turn it red)
A12 an escaped mount point with two escapes in one path: not red (the unescape existed; the plant of S06071 turns it red)
A12 an item inside a bind mount of another directory: red at 1be2f05, green at 19f4d59
```

The plants of the unreadable table (the apply's refusal replaced by a pass, then the plan's skip
replaced by a pass) and of a single-escape unescape were each GREEN over the whole module before
the two edits and RED after them, each restored by its digest. The rows S06068 (the apply's
refusal), S06069 (the plan's skip), S06070 (the root-field clause) and S06071 (the unescape reads
every escape) are proved singly by full id: killed.

A5's criterion said the apply deletes nothing when one approved item changed after the list; it now
says between the list and the apply's checks, and states that a change after the checks stops the
run with earlier deletions kept. The runbook's reason for its order is corrected: the apply deletes an
item only while its digest is the list's, so a snapshot taken after the list holds what is deleted, unless the item changed after the list and changed back before the apply.

## Fix round 6

The sixth review found that a bind of a file system's root directory, `/` or a protected directory
that is itself a mount point, reads `/` in the mount table's root field, so the fifth round's clause
did not see it, and A12's fixture gave every mount one device, so its control was such a bind. The
order of work:

- 8f569b9: A12's test with each faked mount on its own device, and the whole-file-system case
  split in two: another file system mounted whole at an ancestor binds nothing (listed), and the
  root file system mounted whole a second time is refused. Red over the whole module: only A12
  failed, by assertion;
- 8fb8f0c: the fix, a second clause of `plan.mounted` for a file system the table lists mounted
  whole at two points;
- 253364e: the row S06072, proved with its full id: killed. The row S06070 is proved again by
  full id: killed.

```text
A12: red at 8f569b9: AssertionError: Lists differ: [{...}] != [] (an item inside a file system mounted whole at two points was listed, and the apply's check let it through)
A12: green at 8fb8f0c
```

Per case, as the subtest reads:

```text
A12 an item inside a bind mount of another directory: red at 8f569b9, green at 8fb8f0c (the edited subtest; its earlier cases are green at both)
```

Two records of the same criterion are given, as in rounds 4 and 5: the pair above is the latest,
and the earlier pairs stand for the cases they added. The documents state what the code now
refuses: A12 gains the file-system case, the disclosed cost names both hosts it refuses, and a
package is still removed on such a host, since the refusal is of path items.

## Fix round 7

The seventh review found that a directory bound at a protected path is deleted through its source:
an item in, or holding, the directory a bind mount shows lies beside the bind and not under it, so
no clause keyed on the item's own path refused it. The order of work:

- dde939b: A12's test gains the source direction in the bind subtest, with two controls (the bind
  replaced by another directory, and the bind on another device). Red over the whole module: only
  A12 failed, by assertion;
- b61d1dc: the fix, a third clause of `plan.mounted` that finds the directory a bind mount shows
  from the table's rows, and the reason for it in `mount_reason`;
- 35c11a3: the row S06073, proved with its full id: killed.

```text
A12: red at dde939b: AssertionError: Lists differ: [{...}] != [] (an item inside the source of a directory bound at another point was listed, and the apply's check let it through)
A12: green at b61d1dc
```

Per case, as the subtest reads:

```text
A12 an item inside a bind mount of another directory: red at dde939b, green at b61d1dc (the added cases; its earlier cases are green at both)
```

Two records of the same criterion are given, as in rounds 4 to 6: the pair above is the latest, and
the earlier pairs stand for the cases they added. The documents state what the code now refuses: A7,
R7 and A12 gain the directory a bind mount shows, and the disclosed cost names the host with a
directory bound elsewhere, which refuses every path item inside or holding that directory.
