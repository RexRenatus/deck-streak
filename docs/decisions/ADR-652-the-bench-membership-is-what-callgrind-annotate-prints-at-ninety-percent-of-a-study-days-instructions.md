---
status: accepted
decision-makers: "the maintainers"
---

# ADR-652: the bench membership is what callgrind_annotate prints at ninety percent of a study day's instructions

## Context and Problem Statement

SPEC-618 put CodSpeed benches inside the required `ci` check, but which functions earn a bench is
still chosen by hand (`perf/benchmarks.txt`, 11 names). A bench that measures a cold function
costs review time and proves little, and a hot function with no bench can grow slower unseen. The
census measures the choice: one recorded study day gives each function its share of the
instructions a learner's day runs, and every pull request checks that the benches still reach the
functions that carry most of them.

## Decision Drivers

- Membership is measured, never chosen, and re-derived each release from the release's own code.
- A pull request's check needs no build and no profiler: it reads committed files and the tree.
- A release is never held by the census, and the files a check reads cannot drift a release behind
  unseen.
- A broken instrument reads VOID, never an empty and passing membership.

## Decisions, and the alternatives each was chosen against

### D1. Valgrind's callgrind, not the CodSpeed instrument

The census needs a per-function exclusive instruction table it can read. callgrind counts
instructions, the measure CodSpeed's CPU simulation is built on, from our own run:
`valgrind --tool=callgrind --collect-atstart=no --toggle-collect='*study_day::measured*'`. The
manual says `--toggle-collect` toggles collection on entering and leaving the function and that
collection then starts off (sections 6.2.2 and 6.3.3); the flag states it. The Valgrind version is
recorded beside the membership, and a listing from another version is refused.

- Chosen: callgrind, because its table is ours to read and its counts do not move with load.
- CodSpeed's flame graphs: rejected because they live in its dashboard and no export of them was
  read, so a check could not hold them.
- A sampling profiler over the native binary: rejected because samples are noisy where instruction
  counts repeat run after run.

### D2. The membership is the list callgrind_annotate prints at the share

`callgrind_annotate --inclusive=no --threshold=<S> --auto=no` prints functions by exclusive cost
and stops when the sum of the printed shares is at least S (manual 6.6). That list is M. `derive`
reads it, normalizes each name (a Rust legacy hash suffix `::h` and 16 hex digits removed, the
path and the file kept, entries one name and file share summed) and checks that the list stops
where the threshold says, so a listing printed at another threshold, or cut short, is refused.

- Chosen: the printed list, because it carries no formula of our own to defend.
- A share threshold computed by the script over a whole listing: rejected because it would restate
  callgrind_annotate's rule in a second place that could drift from it.
- Inclusive cost: rejected because a caller's inclusive cost counts its callees again, so `main`
  alone would reach any share.

### D3. Reach inside each bench's measured function, and files from an untoggled run

Each Rust bench's measured body is the `#[inline(never)]` function `<bench>_measured`. Its reach
run collects only inside it (`--toggle-collect='*<bench>_measured*'`), so setup and input closures
are no reach, and a bench with 0 instructions inside its toggle reads VOID (an inlined or renamed
function leaves the toggle nothing to match). An untoggled run of each bench binary lists the
source files it reaches, setup included, which only the bench plan's applicability reads.

- Chosen: the toggle for reach and the untoggled run for files, because reach must not credit
  setup, while applicability must err toward running a bench whose setup changed.
- One untoggled run for both: rejected because setup would count as reach.

### D4. S is 90%, fixed after a measurement report

The figure is proposed at 90% of exclusive instructions and fixed only after the report SPEC-630
R8 names: determinism, the curve at 80, 90, 95 and 99% split into our code and third-party code,
reach by the seven Rust benches, and churn across 50 dev commits. 90% stands while |M(90%)| is 60
functions or fewer; above that the curve goes to the maintainers, because a lower share is a
weaker gate.

- Chosen: 90% with an adoption rule, because it covers most of a day's work while a flat curve is
  caught before it demands broad benches.
- 99%, the tool's default: rejected because the tail of a study day is long, and it would demand a
  bench for nearly every function the day touches.

### D5. Derived at the release pull request's head, judged by the tags' dev parents

A `perf-census` job of `ci.yml` applies only on the release run, checks out the release pull
request's head and records it as `derived_at`. A release merges into main with a merge commit and
its tag sits there, so the tag is never on dev, while its second parent is that head; the merge ref
is kept by no branch and is never an ancestor of a later HEAD. `check`'s `stale` arm admits
`derived_at` only when it is an ancestor of HEAD and the dev parent of the newest SemVer release
tag or the one before it, so a refresh pull request has one release cycle to land. With one tag
only its dev parent qualifies, and a tag with no second parent reads VOID.

- Chosen: the head and the two newest tags, because the head is the commit the release run
  measured and the grace spans exactly the window between a tag and its refresh.
- The merge ref: rejected because no later HEAD holds it.
- Any ancestor of HEAD: rejected because the files could then lag any number of releases.
- A job inside `ci`'s needs: rejected because the census informs and must never hold a release.

### D6. The check runs in hygiene, report-only first

`scripts/perf-census.py check` runs in the `hygiene` job's python stage with no build and no
Valgrind, by five arms: `shape`, `unreached`, `stale`, `void` and the informational `edge`. It is
report-only (prints, exits 0) until the first refresh pull request; `--enforce` then makes
`shape`, `unreached`, `stale` and `void` exit 1. Every git read goes through the root the caller
passes, which the tests fill with a throwaway repository.

- Chosen: the hygiene stage, because hygiene is already in `ci`'s needs and holds the full history.
- A job of its own: rejected because it would add a required job for a read a stage already makes.
- Enforcing at once: rejected because the first membership files, committed in this pull request,
  are derived before any release tag names their head.

### D7. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani: a standard-library script over files and git, with no
protocol, state machine, interleaving or engine path.

## Consequences

- Good, because the benches follow the code a study day runs, re-measured each release.
- Good, because deleting a bench a hot function needs reads red on its pull request once enforced.
- Bad, because a release with a new member needs its refresh pull request to add a bench before the
  next release is tagged.
- Bad, because names are matched by text, so a symbol that differs between the study day and a
  bench binary reads `unreached` until a reviewer reads it by name.

## Confirmation

- `scripts/tests/test_perf_census.py`, run by the gate python stage, holds `derive`'s parse and
  write half and each arm of `check` (SPEC-630 A1 to A6).
- The release run's `perf-census` artifact and the first refresh pull request show the membership
  derived at a release head.

## What would make this wrong

- callgrind_annotate stops printing at another rule: D2's stop check would refuse every listing.
- A release merges with something other than a merge commit: every tag would read VOID.
- CodSpeed exports a per-function table a check can read: D1 could then read one instrument.

## More Information

SPEC-630; SPEC-618 and ADR-640 (the benches and the bench plan).
