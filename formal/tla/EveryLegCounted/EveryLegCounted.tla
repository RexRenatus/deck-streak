--------------------------- MODULE EveryLegCounted ---------------------------
(***************************************************************************)
(* A run's Rust mutants, dealt into legs that finish in any order, and the *)
(* verdict that counts them (SPEC-362, ADR-373; #220, #691).               *)
(*                                                                         *)
(* The plan deals each listed mutant to one leg. Each leg is a job of one  *)
(* matrix: it finishes whole, partial or not at all, and uploads what it   *)
(* has to the run's artifact store under its own name. "Re-run failed      *)
(* jobs" runs a failed leg again, as a new attempt, and the verdict again. *)
(* The verdict reads one report per leg from the store and decides pass,   *)
(* fail or VOID. The model checks that a pass means every leg the plan     *)
(* dealt a mutant reported whole, and that the reports the verdict read    *)
(* hold each listed mutant exactly once, whatever the plan dealt.          *)
(*                                                                         *)
(* Modelled claims (file and item, never line numbers):                    *)
(*   mutation-verdict.py::shards        - the plan deals mutant i to leg   *)
(*                                        i mod count, legs 0..count-1;    *)
(*                                        the deal is chosen once, freely  *)
(*                                        (Plan), since the properties     *)
(*                                        must hold for any sizing defect. *)
(*   mutation-verdict.py::whole_reports - every planned leg 0..n-1 is      *)
(*                                        promised; a leg the plan gave no *)
(*                                        mutant and that left nothing is  *)
(*                                        not started; any other leg with  *)
(*                                        no report, or a partial one, is  *)
(*                                        VOID by name (LegVoid).          *)
(*   mutation-verdict.py::partition     - a mutant tested in more legs     *)
(*                                        than the plan lists it FAILS;    *)
(*                                        when no leg was VOID, one tested *)
(*                                        in fewer is VOID (NeverTested).  *)
(*   mutation-verdict.py::examined_sum  - when nothing was VOID, the       *)
(*                                        reports' counts must equal the   *)
(*                                        plan's listing (CountMismatch).  *)
(*   mutation-verdict.py::population_gaps - the tool's listing is the      *)
(*                                        population: a listed mutant the  *)
(*                                        plan's legs do not hold is VOID  *)
(*                                        (Gap).                           *)
(*   mutation-verdict.py::Verdict.close - any failure is FAIL, else any    *)
(*                                        VOID is VOID, else the verdict   *)
(*                                        passes (Judge).                  *)
(*                                                                         *)
(* Abstractions, each confirmed in the code:                               *)
(*   - A whole report holds exactly the mutants dealt to the leg whose     *)
(*     slice it ran (ran), which is another leg's when a leg is wired to   *)
(*     the wrong slice; a partial report is VOID whatever it holds.        *)
(*   - The verdict's other checks (a leg's baseline log against the        *)
(*     per-mutant timeout, the memory cap, a missed mutant) only add a     *)
(*     FAIL or a VOID, so they are left out: every pass the code reaches,  *)
(*     the model reaches too, and a property of a pass stays sound.        *)
(*   - How a re-run leg's upload meets its earlier attempt's: ci.yml's     *)
(*     upload of mutation-rust-shard-<k> sets no overwrite, and the        *)
(*     verdict downloads mutation-rust-shard-* with merge-multiple: true.  *)
(*     The action's documentation says an upload fails when the name       *)
(*     exists unless overwrite is set, and does not say how a re-run       *)
(*     attempt meets an earlier attempt's artifact. So the verdict may     *)
(*     read any attempt the store holds for a leg, or none of them         *)
(*     (Verdict): the model is sound for every reading.                    *)
(*   - A mutant the plan dealt to no leg is Dropped, one past the last     *)
(*     leg, so that every comparison is between integers.                 *)
(*                                                                         *)
(* Switches, each FALSE in MCEveryLegCounted.cfg, which models the code:   *)
(*   CountReported - the verdict promises only the legs whose report       *)
(*                   arrived, never every planned leg (#220's class at     *)
(*                   leg size).                                            *)
(*   TrustThePlan  - the verdict takes the plan's legs as the population   *)
(*                   and never holds them to the tool's listing.           *)
(***************************************************************************)
\* @phx covers scripts/mutation-verdict.py anchor=shards digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=whole_reports digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=partition digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=examined_sum digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers scripts/mutation-verdict.py anchor=population_gaps digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #220, #691
\* @phx property PassMeansEveryLegCounted ramp=report
\* @phx property PassMeansThePartition ramp=report
\* @phx witness witness/PassMeansEveryLegCounted.cfg kills=PassMeansEveryLegCounted
\* @phx witness witness/PassMeansThePartition.cfg kills=PassMeansThePartition
EXTENDS Naturals, FiniteSets

CONSTANTS Mutants,        \* the mutants the tool lists
          NLegs,          \* the legs the plan sizes the run at
          MaxAttempts,    \* the attempts a leg may take, the first included
          CountReported,  \* switch: promise only the legs whose report arrived
          TrustThePlan    \* switch: never hold the plan's legs to the listing

ASSUME /\ NLegs \in Nat \ {0}
       /\ MaxAttempts \in Nat \ {0}
       /\ CountReported \in BOOLEAN
       /\ TrustThePlan \in BOOLEAN

Legs == 0..(NLegs - 1)

\* A mutant the plan dealt to no leg.
Dropped == NLegs

Kinds == {"whole", "partial"}

\* One upload of a leg's report: the attempt that made it, the slice it ran, whole or partial.
Report == [attempt : 1..MaxAttempts, ran : Legs, kind : Kinds]

\* The verdict read no report for a leg.
None == [attempt |-> 0, ran |-> 0, kind |-> "none"]

Verdicts == {"unplanned", "pending", "pass", "fail", "void"}

VARIABLES dealt,    \* the plan: each mutant's leg, or Dropped
          store,    \* the run's artifact store: each leg's uploads, every attempt kept
          attempt,  \* each leg's current attempt
          done,     \* whether each leg's current attempt has finished
          verdict,  \* the verdict job's word
          pick      \* the report the verdict read for each leg, or None

vars == <<dealt, store, attempt, done, verdict, pick>>

TypeOK == /\ dealt \in [Mutants -> Legs \cup {Dropped}]
          /\ store \in [Legs -> SUBSET Report]
          /\ attempt \in [Legs -> 1..MaxAttempts]
          /\ done \in [Legs -> BOOLEAN]
          /\ verdict \in Verdicts
          /\ pick \in [Legs -> Report \cup {None}]

Init == /\ dealt = [m \in Mutants |-> Dropped]
        /\ store = [l \in Legs |-> {}]
        /\ attempt = [l \in Legs |-> 1]
        /\ done = [l \in Legs |-> FALSE]
        /\ verdict = "unplanned"
        /\ pick = [l \in Legs |-> None]

\* mutation-verdict.py::shards: the plan is written once, before any leg runs.
Plan == /\ verdict = "unplanned"
        /\ dealt' \in [Mutants -> Legs \cup {Dropped}]
        /\ verdict' = "pending"
        /\ UNCHANGED <<store, attempt, done, pick>>

\* Leg l's current attempt ends, having run slice r, and uploads a whole report, a partial one, or
\* nothing (a runner shut down before its upload step).
LegFinish(l, r, k) ==
    /\ verdict = "pending"
    /\ ~done[l]
    /\ store' = IF k = "nothing"
                THEN store
                ELSE [store EXCEPT ![l] = @ \cup {[attempt |-> attempt[l], ran |-> r, kind |-> k]}]
    /\ done' = [done EXCEPT ![l] = TRUE]
    /\ UNCHANGED <<dealt, attempt, verdict, pick>>

\* The mutants the plan dealt to leg r.
Slice(r) == {m \in Mutants : dealt[m] = r}

\* mutation-verdict.py::whole_reports: the legs the verdict promises. The code walks every planned
\* leg 0..n-1; under CountReported only the legs whose report the verdict read.
Promised(p) == IF CountReported THEN {l \in Legs : p[l] # None} ELSE Legs

\* A promised leg with no report while the plan dealt it a mutant, or with a partial report.
LegVoid(p) == \E l \in Promised(p) : \/ p[l] = None /\ Slice(l) # {}
                                     \/ p[l] # None /\ p[l].kind = "partial"

\* The promised legs whose report the verdict read whole.
Whole(p) == {l \in Promised(p) : p[l] # None /\ p[l].kind = "whole"}

\* mutation-verdict.py::partition: in how many whole reports a mutant was tested, and how many
\* times the plan's legs list it.
Tested(p, m) == Cardinality({l \in Whole(p) : dealt[m] = p[l].ran})
Listed(m) == IF dealt[m] \in Legs THEN 1 ELSE 0

Fail(p) == \E m \in Mutants : Tested(p, m) > Listed(m)

NeverTested(p) == /\ ~LegVoid(p)
                  /\ \E m \in Mutants : Tested(p, m) < Listed(m)

\* mutation-verdict.py::examined_sum: the mutants the whole reports count, summed over the reports.
RECURSIVE Counted(_, _)
Counted(p, legs) == IF legs = {}
                    THEN 0
                    ELSE LET l == CHOOSE x \in legs : TRUE
                         IN Cardinality(Slice(p[l].ran)) + Counted(p, legs \ {l})

CountMismatch(p) == /\ ~LegVoid(p)
                    /\ ~NeverTested(p)
                    /\ Counted(p, Whole(p)) # Cardinality({m \in Mutants : dealt[m] \in Legs})

\* mutation-verdict.py::population_gaps: every listed mutant must be in the plan's legs once.
Gap == /\ ~TrustThePlan
       /\ \E m \in Mutants : dealt[m] = Dropped

\* mutation-verdict.py::Verdict.close: a failure is FAIL whatever else was found.
Judge(p) == IF Fail(p) THEN "fail"
            ELSE IF LegVoid(p) \/ NeverTested(p) \/ CountMismatch(p) \/ Gap THEN "void"
            ELSE "pass"

\* The verdict job, once every leg's current attempt has finished, reads one report per leg: any
\* attempt the store holds for it, or none.
Verdict == /\ verdict = "pending"
           /\ \A l \in Legs : done[l]
           /\ \E p \in [Legs -> Report \cup {None}] :
                 /\ \A l \in Legs : p[l] \in store[l] \cup {None}
                 /\ pick' = p
                 /\ verdict' = Judge(p)
           /\ UNCHANGED <<dealt, store, attempt, done>>

\* Whether "Re-run failed jobs" can run leg l again: an attempt is left and it has no whole report.
CanRerun(l) == /\ attempt[l] < MaxAttempts
               /\ ~\E r \in store[l] : r.kind = "whole"

\* "Re-run failed jobs" runs leg l as a new attempt, and the verdict again.
Rerun(l) == /\ verdict \in {"void", "fail"}
            /\ CanRerun(l)
            /\ attempt' = [attempt EXCEPT ![l] = @ + 1]
            /\ done' = [done EXCEPT ![l] = FALSE]
            /\ verdict' = "pending"
            /\ UNCHANGED <<dealt, store, pick>>

\* The run ends once the verdict is decided and no leg can be run again.
Finished == /\ verdict \in {"pass", "fail", "void"}
            /\ verdict = "pass" \/ ~\E l \in Legs : CanRerun(l)
            /\ UNCHANGED vars

Next == \/ Plan
        \/ \E l \in Legs, r \in Legs, k \in {"whole", "partial", "nothing"} : LegFinish(l, r, k)
        \/ Verdict
        \/ \E l \in Legs : Rerun(l)
        \/ Finished

Spec == Init /\ [][Next]_vars

\* A pass means every leg the plan dealt a mutant has a whole report in the store.
\* (mutation-verdict.py::whole_reports: "Under --shard-reports it promised every shard's, from 0 to
\* n-1 (R18)".)
PassMeansEveryLegCounted ==
    verdict = "pass" =>
        \A l \in Legs : (\E m \in Mutants : dealt[m] = l) => \E r \in store[l] : r.kind = "whole"

\* The whole reports the verdict read that hold mutant m.
HeldBy(m) == {l \in Legs : pick[l] # None /\ pick[l].kind = "whole" /\ dealt[m] = pick[l].ran}

\* A pass means the plan dealt every listed mutant to a leg, and the reports the verdict read hold
\* each exactly once. (mutation-verdict.py::population_gaps: "The listing, uploaded beside the plan
\* or named by --listed, is the population".)
PassMeansThePartition ==
    verdict = "pass" =>
        \A m \in Mutants : /\ dealt[m] # Dropped
                           /\ Cardinality(HeldBy(m)) = 1

=============================================================================
