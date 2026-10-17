--------------------------- MODULE LateKillCounted ---------------------------
\* The memory scope's reads after the command, against the kernel's out-of-memory killer (#469).
\* scripts/memory_scope.py runs a command inside a scope that also holds the script's own process,
\* and writes one record the mutation verdict reads. A process the command started can outlive the
\* command, and the kernel can stop any process in the scope at the cap at any step. The script
\* reads the kill counters in memory.events and then the peak in memory.peak, two separate reads,
\* and writes the record once. The cure reads cgroup.procs first, at most Reads times, and reads the
\* counts only once the scope holds the script alone; past the bound the record is not in force.
\*
\* Items, by file and item:
\*   memory_scope.py::run                    Return, and the record's Write
\*   memory_scope.py::alone_after            Wait: at most Reads reads, and the bound's void
\*   memory_scope.py::others_after           Wait's reading of the list: the script alone, or others
\*   memory_scope.py::counts_after           ReadEvents, then ReadPeak
\*   mutation-verdict.py::read_scope_record  Verdict: void unless done and in force
\*   mutation-verdict.py::memory_cap         Verdict: clean at no kill, else counted
\*
\* Abstractions: the refusals before the command, the arms of others_after that find the list not
\* whole, and counts_after's own voids each write a record that is not in force, so each is a void
\* path and always safe; oom moves with oom_kill, which the verdict reads together; no process
\* joins the scope from outside the command's tree; the kernel raises oom_kill in the same step as
\* its victim leaves the scope, an order this tree does not measure (ADR-484 D1); a pause between
\* two reads is a step any other action may interleave with.
\* @phx covers scripts/memory_scope.py anchor=run digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/memory_scope.py anchor=counts_after digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/memory_scope.py anchor=alone_after digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/memory_scope.py anchor=others_after digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=read_scope_record digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=memory_cap digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #469, #439
\* @phx property EveryKillIsCountedOrTheRecordIsVoid ramp=report
\* @phx property NoKillIsReadAsACleanRun ramp=report
\* @phx property AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid ramp=report
\* @phx property AKillAfterBothReadsIsCountedOrTheRecordIsVoid ramp=report
\* @phx witness witness/EveryKillIsCountedOrTheRecordIsVoid.cfg kills=EveryKillIsCountedOrTheRecordIsVoid
\* @phx witness witness/NoKillIsReadAsACleanRun.cfg kills=NoKillIsReadAsACleanRun
\* @phx witness witness/AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid.cfg kills=AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid
\* @phx witness witness/AKillAfterBothReadsIsCountedOrTheRecordIsVoid.cfg kills=AKillAfterBothReadsIsCountedOrTheRecordIsVoid
EXTENDS Naturals, FiniteSets

CONSTANTS Cmd,             \* the command run() starts
          Started,         \* the processes the command's tree may start
          Wrapper,         \* the script's own process, in the scope beside them
          Reads,           \* run()'s reads bound: the most reads of cgroup.procs
          ReadBeforeAlone  \* TRUE is the base's run(): the counts are read at the command's exit

ASSUME /\ Cmd \notin Started
       /\ Wrapper \notin Started \cup {Cmd}
       /\ Reads \in Nat
       /\ Reads > 0
       /\ ReadBeforeAlone \in BOOLEAN

Tree == {Cmd} \cup Started
MaxKills == Cardinality(Tree) + 1
Places == {"command", "wait", "events", "peak", "write", "written"}

VARIABLES live,     \* the command's processes alive in the scope
          begun,    \* every process of the command's tree that has started
          wrapper,  \* the script's own process: "live" or "killed"
          pc,       \* the script's place in run(), in the code's order
          tries,    \* the reads of cgroup.procs taken that listed another process, with a read left
          counter,  \* the kernel's oom_kill count in memory.events
          seen,     \* the oom_kill count the script read
          voided,   \* the wait's bound passed with another process listed
          record,   \* the record the verdict reads
          owed,     \* history: the counter at the last kill while a process of the command lived
          between,  \* history: such kills after the counters' read and before the peak's read
          after     \* history: such kills after both reads

vars == <<live, begun, wrapper, pc, tries, counter, seen, voided, record, owed, between, after>>

TypeOK == /\ begun \subseteq Tree
          /\ Cmd \in begun
          /\ live \subseteq begun
          /\ wrapper \in {"live", "killed"}
          /\ pc \in Places
          /\ tries \in 0..(Reads - 1)
          /\ counter \in 0..MaxKills
          /\ seen \in 0..MaxKills
          /\ voided \in BOOLEAN
          /\ record \in [state : {"running", "done"}, inForce : BOOLEAN, kills : 0..MaxKills]
          /\ owed \in 0..MaxKills
          /\ between \in 0..MaxKills
          /\ after \in 0..MaxKills

\* memory_scope.py::run: the record is begun in force before the command runs.
Init == /\ live = {Cmd}
        /\ begun = {Cmd}
        /\ wrapper = "live"
        /\ pc = "command"
        /\ tries = 0
        /\ counter = 0
        /\ seen = 0
        /\ voided = FALSE
        /\ record = [state |-> "running", inForce |-> TRUE, kills |-> 0]
        /\ owed = 0
        /\ between = 0
        /\ after = 0

\* memory_scope.py::run: the command runs, and any live process of its tree may start another.
Start(p) == /\ p \in Started \ begun
            /\ live # {}
            /\ begun' = begun \cup {p}
            /\ live' = live \cup {p}
            /\ UNCHANGED <<wrapper, pc, tries, counter, seen, voided, record, owed, between, after>>

\* A process of the command's tree ends by itself; End(Cmd) is the command's exit.
End(q) == /\ q \in live
          /\ live' = live \ {q}
          /\ UNCHANGED <<begun, wrapper, pc, tries, counter, seen, voided, record, owed, between, after>>

\* The kernel's out-of-memory killer stops a live process in the scope, the script's own included
\* until it has written its record, and raises oom_kill in the same step.
Kill(v) == /\ \/ v \in live
              \/ (v = Wrapper /\ wrapper = "live" /\ pc # "written")
           /\ counter' = counter + 1
           /\ live' = live \ {v}
           /\ wrapper' = IF v = Wrapper THEN "killed" ELSE wrapper
           /\ owed' = IF live # {} THEN counter + 1 ELSE owed
           /\ between' = IF live # {} /\ pc = "peak" THEN between + 1 ELSE between
           /\ after' = IF live # {} /\ pc \in {"write", "written"} THEN after + 1 ELSE after
           /\ UNCHANGED <<begun, pc, tries, seen, voided, record>>

\* memory_scope.py::run: subprocess.run returns once the command itself has exited; the base reads
\* the counts at once, and the cure waits first.
Return == /\ wrapper = "live"
          /\ pc = "command"
          /\ Cmd \notin live
          /\ pc' = IF ReadBeforeAlone THEN "events" ELSE "wait"
          /\ UNCHANGED <<live, begun, wrapper, tries, counter, seen, voided, record, owed, between, after>>

\* memory_scope.py::alone_after and memory_scope.py::others_after: one read of cgroup.procs. The
\* script alone ends the wait; another process listed with a read left pauses for the next read;
\* another process listed at the last read writes the record not in force.
Wait == /\ wrapper = "live"
        /\ pc = "wait"
        /\ IF live = {}
              THEN /\ pc' = "events"
                   /\ UNCHANGED <<tries, voided>>
              ELSE IF tries + 1 < Reads
                      THEN /\ tries' = tries + 1
                           /\ UNCHANGED <<pc, voided>>
                      ELSE /\ pc' = "write"
                           /\ voided' = TRUE
                           /\ UNCHANGED tries
        /\ UNCHANGED <<live, begun, wrapper, counter, seen, record, owed, between, after>>

\* memory_scope.py::counts_after: memory.events is read, through events_of.
ReadEvents == /\ wrapper = "live"
              /\ pc = "events"
              /\ seen' = counter
              /\ pc' = "peak"
              /\ UNCHANGED <<live, begun, wrapper, tries, counter, voided, record, owed, between, after>>

\* memory_scope.py::counts_after: memory.peak is read, through peak_of, a separate read.
ReadPeak == /\ wrapper = "live"
            /\ pc = "peak"
            /\ pc' = "write"
            /\ UNCHANGED <<live, begun, wrapper, tries, counter, seen, voided, record, owed, between, after>>

\* memory_scope.py::run: the record is written once, from the count read, or not in force with
\* zero counts when the wait's bound passed.
Write == /\ wrapper = "live"
         /\ pc = "write"
         /\ record' = [state |-> "done", inForce |-> ~voided, kills |-> IF voided THEN 0 ELSE seen]
         /\ pc' = "written"
         /\ UNCHANGED <<live, begun, wrapper, tries, counter, seen, voided, owed, between, after>>

\* The run has ended: no process of the command lives, and the script wrote its record or was killed.
Finished == /\ live = {}
            /\ (pc = "written" \/ wrapper = "killed")
            /\ UNCHANGED vars

Next == \/ \E p \in Started : Start(p)
        \/ \E q \in Tree : End(q)
        \/ \E v \in Tree \cup {Wrapper} : Kill(v)
        \/ Return
        \/ Wait
        \/ ReadEvents
        \/ ReadPeak
        \/ Write
        \/ Finished

Spec == Init /\ [][Next]_vars

\* mutation-verdict.py::read_scope_record voids a record that is not done or not in force, and
\* mutation-verdict.py::memory_cap reads one with no kill as a clean run, else counts its kills.
Verdict == IF record.state # "done" \/ ~record.inForce
              THEN "void"
              ELSE IF record.kills = 0 THEN "clean" ELSE "counted"

\* #469: every kill the kernel records while any of the command's processes lives is counted in the
\* record, or the record is void.
EveryKillIsCountedOrTheRecordIsVoid == Verdict # "void" => record.kills >= owed

\* #469: such a kill is never read as a clean run.
NoKillIsReadAsACleanRun == Verdict = "clean" => owed = 0

\* #469's first window: a kill after the counters' read and before the peak's read.
AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid == Verdict # "void" => between = 0

\* #469's second window: a kill after both reads, while a process of the command lives.
AKillAfterBothReadsIsCountedOrTheRecordIsVoid == Verdict # "void" => after = 0

=============================================================================
