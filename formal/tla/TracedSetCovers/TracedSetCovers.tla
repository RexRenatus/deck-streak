--------------------------- MODULE TracedSetCovers ---------------------------
\* A sentinel run's trace and the scoped sets read from it (SPEC-420 R7, R8; #829).
\*
\* The runner's process runs the tests in order and traces the target's lines in-process. Every
\* process a test starts, and every process those start, may run a scratch sitecustomize that
\* traces the lines it executes and writes one record when it exits or replaces its image. The
\* records land in one scratch directory. A process may also exit with no record: os._exit, a
\* shell or any other program, a forked child, an interpreter that cannot trace. Processes may
\* outlive the runner. The judge reads the directory once, after the runner exits, and a mutant
\* of a line then runs only the tests in that line's scoped set, or the line runs whole. The model
\* checks that no test any of whose processes executed a line is left out of that line's set.
\*
\* Modelled claims (file and item, never line numbers):
\*   mutation_python.py::child_trace - a record holds the process's token, its parent's token
\*       (the environment's, which each traced process overwrites with its own for its
\*       children), its pid and ppid, the test the environment names, the target lines it
\*       executed, how many processes it started (Spawn) and whether it replaced its image
\*       (Exec). It writes by os.replace, so a record appears whole or not at all (Write); a
\*       forked child never writes; a process that does not trace writes nothing (Vanish).
\*       "a process that writes no record leaves its parent's count short, so its test joins
\*       every scoped set of the file."
\*   mutation_python.py::child_trace - an exec writes the record first and the new image traces
\*       afresh under the same pid and ppid, naming the replaced process's token as its parent,
\*       so no record is its parent (par = Orphan) and the replaced record carries exec (Exec).
\*   mutation_python.py::scoped_sets - read once after the runner exits (Judge): each test's
\*       lines are the root's and its records' lines; "A process that started more processes
\*       than it has direct children, or one that replaced its image, joins its test to every
\*       set". A process's direct children are the records naming its token and its pid; the
\*       root's are counted per test (Joiners).
\*   mutation_python.py::scope_of - a line no test traced runs whole ("unreached"); otherwise
\*       the line runs its traced tests and every joiner (Whole, Scoped).
\*
\* Abstractions, each confirmed in the code, and each one only widens a set or runs a file whole:
\*   - The root traces lines and starts processes only inside a test. A line it runs outside any
\*     test runs whole ("import"), and a process outside any test with no record, or a replaced
\*     image there, makes the trace partial, so every mutant of the file runs whole.
\*   - The root's own exec and a thread a test leaves running join that test to every set; a run
\*     whose test count differs from the sentinel's, a failed trace, a missing root record and an
\*     unreadable record each make the trace partial. None of them can drop a test.
\*   - A start is counted once: scoped_sets reads subprocess.Popen and the os.posix_spawn it makes
\*     as one start. A count above the true one only adds joiners.
\*   - An exec that fails leaves the process running, and it writes again at exit under the same
\*     token. Its record already carries exec, so its test joins whatever it writes after.
\*   - The judge's listing races a write only as a write before or after it, since each record
\*     appears whole. The model reads one snapshot of the written records.
\*   - A line a descendant executes belongs to the test its record names, the test whose process
\*     started its chain (ran). A test that observes another test's lingering process is the
\*     other test's executor, not its own.
\*   - Tokens never repeat (each is eight random bytes), so a pid the system reuses never merges
\*     two processes' children. A test that failed on the sentinel is skipped in a whole run as in
\*     a scoped one (Judge.judge's readers, which scope_of removes), so the model's tests are the
\*     others.
\*   - Slots: a process takes the least free descendant slot, and none starts once MaxDesc are
\*     taken.
\*
\* The bound: two tests, since with one test every line a test executed is either traced by that
\* same test or runs whole, so the count could be removed and no test dropped; and one target
\* line, since each line's conjunct reads only that line's events and the line-free joiners, so a
\* violation with more lines projects onto one line by deleting the other lines' steps.
\*
\* @phx covers scripts/mutation_python.py anchor=child_trace digest=sha256:89641e6b21234ddc433bc34639b9f425c1be9c670fd6b0d90cabc73ce74401db
\* @phx covers scripts/mutation_python.py anchor=scoped_sets digest=sha256:353a692092005990ef283d6ae451b90349878614930192b90f2179931abf5368
\* @phx cites #829
\* @phx property NoExecutorDropped ramp=report
\* @phx witness witness/NoSpawnCount.cfg kills=NoExecutorDropped

EXTENDS Naturals, FiniteSets

CONSTANTS NT,             \* the tests, run in order 1..NT
          MaxDesc,        \* the descendant processes the model can start
          NL,             \* the target's lines
          SkipSpawnCount  \* TRUE drops the per-parent count from the joiners (the witness)

ASSUME /\ NT \in Nat \ {0}
       /\ MaxDesc \in Nat
       /\ NL \in Nat \ {0}
       /\ SkipSpawnCount \in BOOLEAN

Tests == 1..NT
Lines == 1..NL
Desc == 1..MaxDesc
Root == 0                \* the runner's process, as a parent
Orphan == MaxDesc + 1    \* a parent no record is: the token a replaced image names
Phases == {"unborn", "live", "written", "gone"}

VARIABLES cur,          \* the test the root is in, or will begin next; NT + 1 once all ran
          inTest,       \* the root is inside test cur
          rootLines,    \* each test's lines the root traced in-process
          rootSpawned,  \* each test's processes the root started
          rootDone,     \* the runner exited
          st,           \* each descendant slot's phase
          par,          \* the record a descendant's record names as its parent
          tst,          \* the test a descendant's record names
          lns,          \* the target lines a descendant executed
          spw,          \* the processes a descendant started
          exd,          \* a descendant replaced its image
          ran,          \* history: the lines each test's processes executed
          read,         \* the judge has read the directory
          seen          \* the records the judge read

vars == <<cur, inTest, rootLines, rootSpawned, rootDone, st, par, tst, lns, spw, exd, ran,
          read, seen>>

TypeOK == /\ cur \in 1..(NT + 1)
          /\ inTest \in BOOLEAN
          /\ rootLines \in [Tests -> SUBSET Lines]
          /\ rootSpawned \in [Tests -> 0..MaxDesc]
          /\ rootDone \in BOOLEAN
          /\ st \in [Desc -> Phases]
          /\ par \in [Desc -> {Orphan, Root} \cup Desc]
          /\ tst \in [Desc -> Tests]
          /\ lns \in [Desc -> SUBSET Lines]
          /\ spw \in [Desc -> 0..MaxDesc]
          /\ exd \in [Desc -> BOOLEAN]
          /\ ran \in [Tests -> SUBSET Lines]
          /\ read \in BOOLEAN
          /\ seen \subseteq Desc

Init == /\ cur = 1
        /\ inTest = FALSE
        /\ rootLines = [t \in Tests |-> {}]
        /\ rootSpawned = [t \in Tests |-> 0]
        /\ rootDone = FALSE
        /\ st = [d \in Desc |-> "unborn"]
        /\ par = [d \in Desc |-> Root]
        /\ tst = [d \in Desc |-> 1]
        /\ lns = [d \in Desc |-> {}]
        /\ spw = [d \in Desc |-> 0]
        /\ exd = [d \in Desc |-> FALSE]
        /\ ran = [t \in Tests |-> {}]
        /\ read = FALSE
        /\ seen = {}

Unborn == {d \in Desc : st[d] = "unborn"}

\* The least free slot; used only when Unborn is not empty.
Slot == CHOOSE d \in Unborn : \A e \in Unborn : d <= e

Begin == /\ ~rootDone
         /\ ~inTest
         /\ cur <= NT
         /\ inTest' = TRUE
         /\ UNCHANGED <<cur, rootLines, rootSpawned, rootDone, st, par, tst, lns, spw, exd, ran,
                        read, seen>>

End == /\ inTest
       /\ cur' = cur + 1
       /\ inTest' = FALSE
       /\ UNCHANGED <<rootLines, rootSpawned, rootDone, st, par, tst, lns, spw, exd, ran, read,
                      seen>>

\* The root executes line l inside test cur, traced in-process.
RootLine(l) == /\ inTest
               /\ rootLines' = [rootLines EXCEPT ![cur] = @ \cup {l}]
               /\ ran' = [ran EXCEPT ![cur] = @ \cup {l}]
               /\ UNCHANGED <<cur, inTest, rootSpawned, rootDone, st, par, tst, lns, spw, exd,
                              read, seen>>

\* The root starts a process inside test cur; the environment names the test and the root.
RootSpawn == /\ inTest
             /\ Unborn # {}
             /\ rootSpawned' = [rootSpawned EXCEPT ![cur] = @ + 1]
             /\ st' = [st EXCEPT ![Slot] = "live"]
             /\ par' = [par EXCEPT ![Slot] = Root]
             /\ tst' = [tst EXCEPT ![Slot] = cur]
             /\ UNCHANGED <<cur, inTest, rootLines, rootDone, lns, spw, exd, ran, read, seen>>

\* A live descendant starts a process, which inherits its test and names it as its parent.
Spawn(p) == /\ st[p] = "live"
            /\ Unborn # {}
            /\ st' = [st EXCEPT ![Slot] = "live"]
            /\ par' = [par EXCEPT ![Slot] = p]
            /\ tst' = [tst EXCEPT ![Slot] = tst[p]]
            /\ spw' = [spw EXCEPT ![p] = @ + 1]
            /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, lns, exd, ran, read,
                           seen>>

\* A live descendant replaces its image: it writes its record with exec, and the new image
\* traces afresh under the same test, its parent's token naming no record.
Exec(p) == /\ st[p] = "live"
           /\ Unborn # {}
           /\ st' = [st EXCEPT ![p] = "written", ![Slot] = "live"]
           /\ par' = [par EXCEPT ![Slot] = Orphan]
           /\ tst' = [tst EXCEPT ![Slot] = tst[p]]
           /\ exd' = [exd EXCEPT ![p] = TRUE]
           /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, lns, spw, ran, read,
                          seen>>

\* A live descendant executes line l, for the test its record names.
Line(p, l) == /\ st[p] = "live"
              /\ lns' = [lns EXCEPT ![p] = @ \cup {l}]
              /\ ran' = [ran EXCEPT ![tst[p]] = @ \cup {l}]
              /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, st, par, tst, spw,
                             exd, read, seen>>

\* A live descendant exits and its record appears whole.
Write(p) == /\ st[p] = "live"
            /\ st' = [st EXCEPT ![p] = "written"]
            /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, par, tst, lns, spw,
                           exd, ran, read, seen>>

\* A live descendant exits with no record.
Vanish(p) == /\ st[p] = "live"
             /\ st' = [st EXCEPT ![p] = "gone"]
             /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, par, tst, lns, spw,
                            exd, ran, read, seen>>

\* The runner exits once every test ran; its descendants may still be running.
RootFinish == /\ ~rootDone
              /\ ~inTest
              /\ cur = NT + 1
              /\ rootDone' = TRUE
              /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, st, par, tst, lns, spw, exd,
                             ran, read, seen>>

\* The judge reads the directory once, after the runner exits.
Judge == /\ rootDone
         /\ ~read
         /\ read' = TRUE
         /\ seen' = {d \in Desc : st[d] = "written"}
         /\ UNCHANGED <<cur, inTest, rootLines, rootSpawned, rootDone, st, par, tst, lns, spw,
                        exd, ran>>

Finished == /\ read
            /\ UNCHANGED vars

Next == \/ Begin
        \/ End
        \/ \E l \in Lines : RootLine(l)
        \/ RootSpawn
        \/ \E p \in Desc : \/ Spawn(p)
                           \/ Exec(p)
                           \/ Write(p)
                           \/ Vanish(p)
                           \/ \E l \in Lines : Line(p, l)
        \/ RootFinish
        \/ Judge
        \/ Finished

Spec == Init /\ [][Next]_vars

\* The records the judge read that name p as their parent; the root's are counted per test.
DirectOf(p) == {d \in seen : par[d] = p}
DirectOfRoot(t) == {d \in seen : par[d] = Root /\ tst[d] = t}

\* (mutation_python.py::scoped_sets: "A process that started more processes than it has direct
\* children, or one that replaced its image, joins its test to every set".)
CountJoiners ==
    IF SkipSpawnCount
    THEN {}
    ELSE {t \in Tests : rootSpawned[t] > Cardinality(DirectOfRoot(t))}
         \cup {tst[d] : d \in {e \in seen : spw[e] > Cardinality(DirectOf(e))}}

Joiners == CountJoiners \cup {tst[d] : d \in {e \in seen : exd[e]}}

\* The tests whose traced lines hold l: the root's in-process trace and the records read.
Traced(l) == {t \in Tests : l \in rootLines[t]} \cup {tst[d] : d \in {e \in seen : l \in lns[e]}}

\* (mutation_python.py::scope_of: a line no test traced runs whole, "unreached".)
Whole(l) == Traced(l) = {}

Scoped(l) == Traced(l) \cup Joiners

\* Once the judge has read, every test any of whose processes executed a line is in that line's
\* scoped set, or the line runs whole. Processes still running after the read may execute more;
\* each of those has a parent whose count is short or an exec flag, so its test joins.
NoExecutorDropped ==
    read => \A l \in Lines : \A t \in Tests :
                l \in ran[t] => (Whole(l) \/ t \in Scoped(l))

=============================================================================
