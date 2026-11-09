---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The rows plan's shape is checked in its own reader, and a wrong one is refused on stderr

## Context and Problem Statement

`scripts/mutation_rows.py` reads a `--rows-from` plan in `chosen`, which `prove`, `legs` and
`merge` each call. At the base this delivery is cut from, `chosen` parses the plan and adds
`plan.get("rows", [])` to the ids the other selectors name, with no check of the plan's shape or of
its `rows` value. A plan that is JSON but not an object (an array, a string, a number, a boolean or
null) raises `AttributeError` at that `.get`; a `rows` value that is a number or null raises
`TypeError` there; a `rows` string is read as its characters and an object as its keys; an entry
that is a number raises `TypeError` where the unknown ids are joined into a refusal, and one that
is an array or an object raises `TypeError` at the id lookup. Each crash is an uncaught traceback
and exit 1, because the three verbs catch only `OSError`, `json.JSONDecodeError`,
`PopulationRefused` and `UnresolvableTarget` around `chosen`, and `dispatch` catches none of them
(#950).

The retired list had the same fault, cured by its own reader, `retirement_records`, whose wrong
shapes `dispatch` prints as one `mutation_rows: REFUSED: <sentence>` line on stderr with exit 2
(ADR-482 D2). ADR-482 D6 left the plan's reader for an issue of its own, and #950 asks that a plan
of the wrong shape is refused the same way. How is it refused without changing what a well-formed
plan selects, how an unknown id is refused, or how an unreadable plan is refused?

## Decision Drivers

- Every refusal of a plan's shape reads as the retired list's: one named line on stderr, exit 2,
  no traceback, no row proved, and nothing written.
- A well-formed plan selects exactly the rows it did, and every refusal the verbs already print on
  stdout stays where it is.
- The runner stays standard-library Python, with only the imports the missing-tool census admits.
- No line the Lean entry `lean/RetiredListShape` covers changes, and no existing row's anchor moves.
- One test per wrong shape, red at the base for the shape's reason, and one row per behaviour.

## Decision Outcome, and what each decision was chosen against

### D1. One reader checks the plan's shape

`plan_rows` checks the plan's shape where `chosen` read `plan.get("rows", [])`: the plan is an
object, its `rows` absent or an array, and each entry a string. The first wrong shape is refused by
a sentence naming the file as `--rows-from` gave it and the shape it needed, an entry by its index
from 0. The shape and the words are the retired list's reader's (ADR-482 D2): `must hold a
top-level object`, `` `rows` is not an array of row ids `` and `rows[<i>] is not a string`. An
absent `rows` still selects no row from the plan, and any other key is still ignored.

- Chosen against: a check in each of the three verbs, because it makes three copies of one reading that can drift apart.
- Chosen against: a JSON schema library, because the runner imports the standard library alone and `test_mutation_rows_missing_tool.py` admits only its current imports.
- Chosen against: reading an object's keys or a string's characters as ids, as the base did, because a plan of that shape is a writer's fault, never a selection.

### D2. The refusal is its own exception, printed by dispatch on stderr

The refusal is `PlanRefused`, a `ValueError` of its own, and `dispatch` prints it as
`mutation_rows: REFUSED: <sentence>` on stderr and returns 2 for `prove`, `legs` and `merge`, as
#950 asks and as `retired` prints a misshapen retired list. Because it is not a
`PopulationRefused`, the verbs' own `except` tuples let it pass to `dispatch`, so a verb prints
nothing of its own and writes no report and no step output.

- Chosen against: raising `PopulationRefused`, because each verb would print it on stdout as `<verb>: REFUSED:`, not the line #950 asks for, and the plan's shape is not the population's.
- Chosen against: moving every plan refusal to stderr, an unreadable plan and a text that is not JSON among them, because it changes refusals no issue names, which the verbs' tests read on stdout.

### D3. Tests and rows

Five criteria are red at the base by assertion (A1 to A5), each by the exit status the base's
traceback gives, and one guard is not red (A6): it pins that a well-formed plan selects its rows
and that an unknown id is refused, which the base already did, against the new checks. Nine rows,
`S64500` to `S64508`, pin the three checks, the reader's call, its `rows` key and its return, and
each verb's exit status. The delivery is one push.

- Chosen against: leaving the new checks to the Python mutation legs alone, with no hand-proved rows, because those legs judge only the mutants of one pull request's own diff, while a committed row is proved again by the rows leg of a later pull request that changes the runner, so a check or an exit status dropped later is still caught.

### D4. Formal not required

The reader decides no verdict: it returns the plan's `rows` unchanged or refuses. ADR-482 D5's
reason for a Lean entry, a pure function that decides a gate verdict, does not hold, and the cure
leaves the span `lean/RetiredListShape` covers byte-identical.

- Chosen against: a Lean entry for the reader, because its port would restate three type checks whose every arm a test and a row already pin.

## Consequences

- Good, because no wrong plan shape ends a verb in a traceback.
- Good, because every refusal of a plan's shape reads like the retired list's: one line on stderr,
  exit 2, nothing proved or written.
- Bad, because a plan's refusal is now on two streams by its fault: its shape on stderr, an
  unreadable text and an unknown id on stdout.
- Bad, because the new process tests add to the mapped suite's cost for this file in the Python
  mutation legs.

## Confirmation

`scripts/tests/test_mutation_rows.py` plants each wrong shape and reads, for `prove`, `legs` and
`merge` as processes, the whole stderr line, exit 2 and an empty stdout, then that no report and no
step output was written, then the reader's sentence in this process (A1 to A5); A6 reads a
well-formed plan as before. The nine rows of `S64500-S64599` read KILLED by id, and the pull
request's Python mutation legs judge the diff's mutants at its head.

## What would make this wrong

- A plan writer that emits `rows` entries of another shape on purpose, objects carrying an id, say.
- A caller that reads a refusal of a plan's shape on stdout.

## More Information

Issue #950, following #421; SPEC-645; ADR-482; SPEC-465; SPEC-039; SPEC-417.
