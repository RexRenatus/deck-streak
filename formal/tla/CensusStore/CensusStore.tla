---------------------------- MODULE CensusStore ----------------------------
(***************************************************************************)
(* Two censuses of one checkout over one store (SPEC-384, ADR-398; #692).  *)
(*                                                                         *)
(* The real tree's settle census compiles in a store keyed by the lock     *)
(* file, the toolchain pin, its command and the tree root. Two censuses of *)
(* one checkout share that key: two test runs, or a mutation row's run     *)
(* beside a suite, each judging the tree as it stands when it compiles, so *)
(* the two can judge two trees. Each census takes the store's lock,        *)
(* removes every unit of the tree's path packages, compiles and reads the  *)
(* members over its passes, decides its verdict, removes the path units   *)
(* again and releases the lock. The model checks that an accepted verdict *)
(* read only member units of its own tree, and that no member unit is in  *)
(* the store right after a census's removal.                               *)
(*                                                                         *)
(* Modelled claims (file and item, never line numbers):                    *)
(*   xp_census.rs::census_on_store   - takes the lock (Acquire), runs the  *)
(*                                     census, removes the path units      *)
(*                                     after its last pass (Finish) and    *)
(*                                     drops the lock (Release).           *)
(*   xp_census.rs::hold              - opens the store's lock file and     *)
(*                                     takes its exclusive lock, waiting   *)
(*                                     while another census holds it       *)
(*                                     (Acquire), and tags the store so    *)
(*                                     that cargo cleans it.               *)
(*   xp_census.rs::removal           - every unit of every path package    *)
(*                                     leaves the store; the pinned units  *)
(*                                     stay (Remove, Finish).              *)
(*   xp_census.rs::store_key         - two censuses of one checkout with   *)
(*                                     one lock file, toolchain pin and    *)
(*                                     command share one store, whatever   *)
(*                                     tree each judges (one store).       *)
(*   xp_census.rs::not_compiled_here - a member unit cargo answers fresh   *)
(*                                     whose files this census compiled by *)
(*                                     no earlier pass is refused (Read).  *)
(*                                                                         *)
(* Abstractions, each confirmed in the code:                               *)
(*   - A unit is named by its kind: cargo names a unit's files by a hash   *)
(*     of its package, features and profile, never of its source, so the  *)
(*     member unit of two trees has one name, and a Compile replaces the   *)
(*     unit of that name whichever tree compiled it. The refusal compares  *)
(*     names, so it cannot tell another tree's unit from its own: that is  *)
(*     what the lock is for.                                               *)
(*   - Pinned stands for every unit of a package the lock file pins from a *)
(*     registry or git: its content is the same for every tree, so a Read *)
(*     of another tree's pinned unit is the reuse the store exists for.    *)
(*   - A Read is a pass reading a unit's messages. Cargo may answer it     *)
(*     from the store without a Compile when a unit of that name is there  *)
(*     (a fresh unit), or the census compiles it first.                    *)
(*   - The sixteen passes, the selections and the pairs are folded into    *)
(*     the Compile and Read steps: each reads and compiles by name.        *)
(*   - A census that cannot hold or empty its store refuses by name and    *)
(*     reads nothing; such a run is a stuttering step here.                *)
(*                                                                         *)
(* Switches, each TRUE in MCCensusStore.cfg, which models the code:        *)
(*   Locked - a census holds the store's lock from Acquire to Release.     *)
(*   Pruned - Remove and Finish take the path units out of the store.      *)
(***************************************************************************)
\* @phx covers crates/progression/tests/xp_census.rs anchor=census_on_store digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/progression/tests/xp_census.rs anchor=hold digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/progression/tests/xp_census.rs anchor=removal digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/progression/tests/xp_census.rs anchor=store_key digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/progression/tests/xp_census.rs anchor=not_compiled_here digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #692
\* @phx property AcceptedVerdictReadsItsOwnMembers ramp=report
\* @phx property NoMemberSurvivesTheRemoval ramp=report
\* @phx witness witness/AcceptedVerdictReadsItsOwnMembers.cfg kills=AcceptedVerdictReadsItsOwnMembers
\* @phx witness witness/NoMemberSurvivesTheRemoval.cfg kills=NoMemberSurvivesTheRemoval
EXTENDS Naturals, FiniteSets

CONSTANTS Censuses,  \* the censuses of one checkout
          Trees,     \* the trees they can judge
          Nobody,    \* no census holds the store's lock
          Locked,    \* switch: a census holds the store's lock
          Pruned     \* switch: the removal takes the path units out

ASSUME /\ Nobody \notin Censuses
       /\ Locked \in BOOLEAN
       /\ Pruned \in BOOLEAN

Kinds == {"member", "pinned"}

\* A unit in the store: its name (its kind) and the tree whose census compiled it.
Units == [kind : Kinds, tree : Trees]

Steps == {"idle", "holding", "removed", "passes", "finished", "released"}

Verdicts == {"none", "accepted", "refused"}

VARIABLES tree,      \* the tree each census judges
          store,     \* the units in the one store
          holder,    \* the census that holds the store's lock, or Nobody
          pc,        \* each census's step
          compiled,  \* the unit names each census compiled
          read,      \* the units each census read
          refusal,   \* whether each census refused a unit it read
          verdict    \* each census's verdict

vars == <<tree, store, holder, pc, compiled, read, refusal, verdict>>

TypeOK == /\ tree \in [Censuses -> Trees]
          /\ store \subseteq Units
          /\ holder \in Censuses \cup {Nobody}
          /\ pc \in [Censuses -> Steps]
          /\ compiled \in [Censuses -> SUBSET Kinds]
          /\ read \in [Censuses -> SUBSET Units]
          /\ refusal \in [Censuses -> BOOLEAN]
          /\ verdict \in [Censuses -> Verdicts]

Init == /\ tree \in [Censuses -> Trees]
        /\ store = {}
        /\ holder = Nobody
        /\ pc = [c \in Censuses |-> "idle"]
        /\ compiled = [c \in Censuses |-> {}]
        /\ read = [c \in Censuses |-> {}]
        /\ refusal = [c \in Censuses |-> FALSE]
        /\ verdict = [c \in Censuses |-> "none"]

\* The store with every path package's unit taken out.
PinnedOnly(s) == {u \in s : u.kind = "pinned"}

\* xp_census.rs::census_on_store: the census takes the store's lock, waiting while another holds it.
Acquire(c) == /\ pc[c] = "idle"
              /\ Locked => holder = Nobody
              /\ holder' = IF Locked THEN c ELSE holder
              /\ pc' = [pc EXCEPT ![c] = "holding"]
              /\ UNCHANGED <<tree, store, compiled, read, refusal, verdict>>

\* xp_census.rs::removal, before the first pass.
Remove(c) == /\ pc[c] = "holding"
             /\ store' = IF Pruned THEN PinnedOnly(store) ELSE store
             /\ pc' = [pc EXCEPT ![c] = "removed"]
             /\ UNCHANGED <<tree, holder, compiled, read, refusal, verdict>>

\* A pass compiles the unit named k for its census's tree; it replaces the unit of that name.
Compile(c, k) == /\ pc[c] \in {"removed", "passes"}
                 /\ store' = {u \in store : u.kind # k} \cup {[kind |-> k, tree |-> tree[c]]}
                 /\ compiled' = [compiled EXCEPT ![c] = @ \cup {k}]
                 /\ pc' = [pc EXCEPT ![c] = "passes"]
                 /\ UNCHANGED <<tree, holder, read, refusal, verdict>>

\* A pass reads the unit named k as cargo answers it from the store, compiled or not.
\* xp_census.rs::not_compiled_here: a member unit whose name this census never compiled is refused.
Read(c, k) == /\ pc[c] \in {"removed", "passes"}
              /\ \E u \in store :
                    /\ u.kind = k
                    /\ read' = [read EXCEPT ![c] = @ \cup {u}]
              /\ refusal' = [refusal EXCEPT ![c] = @ \/ (k = "member" /\ k \notin compiled[c])]
              /\ pc' = [pc EXCEPT ![c] = "passes"]
              /\ UNCHANGED <<tree, store, holder, compiled, verdict>>

\* The census decides its verdict once it has read a member, and removes the path units again.
Finish(c) == /\ pc[c] = "passes"
             /\ \E u \in read[c] : u.kind = "member"
             /\ verdict' = [verdict EXCEPT ![c] = IF refusal[c] THEN "refused" ELSE "accepted"]
             /\ store' = IF Pruned THEN PinnedOnly(store) ELSE store
             /\ pc' = [pc EXCEPT ![c] = "finished"]
             /\ UNCHANGED <<tree, holder, compiled, read, refusal>>

\* The census drops the store's lock.
Release(c) == /\ pc[c] = "finished"
              /\ holder' = IF holder = c THEN Nobody ELSE holder
              /\ pc' = [pc EXCEPT ![c] = "released"]
              /\ UNCHANGED <<tree, store, compiled, read, refusal, verdict>>

\* Every census has released the store.
Done == /\ \A c \in Censuses : pc[c] = "released"
        /\ UNCHANGED vars

Next == \/ \E c \in Censuses : Acquire(c) \/ Remove(c) \/ Finish(c) \/ Release(c)
        \/ \E c \in Censuses, k \in Kinds : Compile(c, k) \/ Read(c, k)
        \/ Done

Spec == Init /\ [][Next]_vars

\* An accepted verdict read only member units its own tree's census compiled (SPEC-384 R2, R4).
AcceptedVerdictReadsItsOwnMembers ==
    \A c \in Censuses :
        verdict[c] = "accepted" =>
            \A u \in read[c] : u.kind = "member" => u.tree = tree[c]

\* Right after a census's removal, the store holds no member unit (SPEC-384 R1, R2).
NoMemberSurvivesTheRemoval ==
    \A c \in Censuses : pc[c] = "removed" => \A u \in store : u.kind # "member"

=============================================================================
