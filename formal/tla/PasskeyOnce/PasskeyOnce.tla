---------------------------- MODULE PasskeyOnce ----------------------------
\* @phx covers crates/identity/src/passkeys.rs anchor=take digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/identity/src/linking.rs anchor=take digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/identity/src/passkeys.rs anchor=advance digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #627
\* @phx property SingleUse ramp=report
\* @phx property CounterAdvances ramp=report
\* @phx witness witness/a-take-that-keeps-its-entry.cfg kills=SingleUse
\* @phx witness witness/a-counter-read-then-written-in-two-steps.cfg kills=CounterAdvances
\*
\* A passkey ceremony or a link code taken once, and a passkey's counter advanced by one
\* compare-and-swap (SPEC-359 R3, R4, R7 and R8; ADR-370). Request tasks run concurrently: each
\* redeem, registration finish and sign-in finish takes an entry out of an in-memory store, and each
\* sign-in finish then writes the credential's stored counter. The stores and the counter's row are
\* the shared state; "an entry is taken once" and "the counter is read and advanced in one
\* compare-and-swap" are claims that nothing happens between the find and the removal, and between
\* the comparison and the write.
\*
\* What the model abstracts, and why:
\* - One set of entries, `Codes`, stands for both stores. The ceremony store and the link-code store
\*   have one shape: a list under one mutex, an insert that pushes a fresh random id's digest, and a
\*   take that finds and removes in one critical section. A link code is never asserted; the model
\*   lets any taken entry be asserted, which only adds behaviours.
\* - An entry is issued once: its id is 32 fresh random bytes, so no two inserts share one.
\* - Expire stands for every removal with no taker: an entry past its lifetime (a take of it
\*   refuses `challenge_expired` or `link_code_expired`) and the oldest ceremony evicted by a ninth.
\* - A take of an id no live entry has refuses and changes nothing: a stuttering step.
\* - One credential, one row. The registration's insert sets the row's first counter to whatever
\*   the authenticator reports (`stored`, chosen at Init). A deletion of the row (the owner's
\*   removal, the erase) only ends assertions, which removes behaviours, so it is not modelled.
\* - A presented counter is any value up to `MaxCounter`: a cloned or replaying authenticator may
\*   present any counter, so the model never assumes the authenticator advances its own.
\* - A sign-in's start reads the owner's passkeys, counter included, into the ceremony's state
\*   (`passkeys.rs::Passkeys::start_sign_in`, `held`); the rule reads that value (`seen`), and the
\*   compare-and-swap compares the row with it. The library's signature verification, the
\*   credential lookup and the session it opens write nothing shared, so they are not modelled.
\*
\* The switches are the fixed design when TRUE: `TakeRemoves` that a take removes the entry it
\* finds in the critical section that finds it, and `OneSwap` that the counter's write is ONE
\* `UPDATE ... WHERE counter = <the value the rule read>`, so a row another task advanced in between
\* refuses `counter_regressed`.
\*
\* The action-to-code map, by `file::item`:
\* - Issue -> `passkeys.rs::Ceremonies::insert` and `linking.rs::LinkCodes::insert`, each under the
\*   store's mutex.
\* - Take -> `passkeys.rs::Ceremonies::take` and `linking.rs::LinkCodes::take`: the position is
\*   found and the entry removed while the one `MutexGuard` is held.
\* - Assert -> `passkeys.rs::Passkeys::verify_assertion`, which applies `counter_advances` to the
\*   counter the ceremony read, then `passkeys.rs::Passkeys::advance`, the compare-and-swap; a row
\*   that no longer holds the read value answers `counter_regressed`.
\* - Release -> the end of a redeem or a registration finish, or a verification that refused.
\* - Expire -> a take of an entry `CEREMONY_LIFETIME` or `LINK_CODE_LIFETIME` old, which removes
\*   and refuses it, and the eviction of the oldest ceremony beyond `MAX_LIVE_CEREMONIES`.

EXTENDS Naturals, FiniteSets

CONSTANTS TakeRemoves, OneSwap, NTasks, NCodes, MaxCounter

Tasks == 1..NTasks
Codes == 1..NCodes
Counters == 0..MaxCounter
None == 0

VARIABLES issued, live, seen, takers, holds, stored, accepted

vars == <<issued, live, seen, takers, holds, stored, accepted>>

TypeOK ==
    /\ issued \subseteq Codes
    /\ live \subseteq issued
    /\ seen \in [Codes -> Counters]
    /\ takers \in [Codes -> SUBSET Tasks]
    /\ holds \in [Tasks -> Codes \cup {None}]
    /\ stored \in Counters
    /\ accepted \subseteq (Codes \X Tasks \X Counters)

Init ==
    /\ issued = {}
    /\ live = {}
    /\ seen = [c \in Codes |-> 0]
    /\ takers = [c \in Codes |-> {}]
    /\ holds = [t \in Tasks |-> None]
    /\ stored \in Counters
    /\ accepted = {}

\* The counter rule, as SPEC-359 R8 states it (`passkeys.rs::counter_advances`): "an assertion is
\* accepted when the presented and stored counters are both zero or the presented one is greater".
Rule(read, presented) == (read = 0 /\ presented = 0) \/ presented > read

\* An entry is inserted under a fresh id; a sign-in's start reads the stored counter into it.
Issue(c) ==
    /\ c \notin issued
    /\ issued' = issued \cup {c}
    /\ live' = live \cup {c}
    /\ seen' = [seen EXCEPT ![c] = stored]
    /\ UNCHANGED <<takers, holds, stored, accepted>>

\* An entry leaves the store with no taker: expired, or evicted as the oldest.
Expire(c) ==
    /\ c \in live
    /\ live' = live \ {c}
    /\ UNCHANGED <<issued, seen, takers, holds, stored, accepted>>

\* ONE step under the store's mutex: the live entry is found, removed and given to the task.
Take(t, c) ==
    /\ holds[t] = None
    /\ c \in live
    /\ live' = IF TakeRemoves THEN live \ {c} ELSE live
    /\ takers' = [takers EXCEPT ![c] = @ \cup {t}]
    /\ holds' = [holds EXCEPT ![t] = c]
    /\ UNCHANGED <<issued, seen, stored, accepted>>

\* ONE compare-and-swap step: the presented counter `n` passes the rule against the value the
\* ceremony read, and the row still holds that value; the row then holds `n`. Otherwise the
\* assertion is refused `counter_regressed` and the row is unchanged.
Assert(t, n) ==
    /\ holds[t] # None
    /\ LET c == holds[t]
           swaps == Rule(seen[c], n) /\ (OneSwap => stored = seen[c])
       IN  /\ stored' = IF swaps THEN n ELSE stored
           /\ accepted' = IF swaps THEN accepted \cup {<<c, t, n>>} ELSE accepted
    /\ holds' = [holds EXCEPT ![t] = None]
    /\ UNCHANGED <<issued, live, seen, takers>>

\* A task that took a link code or a registration ceremony, or whose verification refused, ends
\* without an assertion.
Release(t) ==
    /\ holds[t] # None
    /\ holds' = [holds EXCEPT ![t] = None]
    /\ UNCHANGED <<issued, live, seen, takers, stored, accepted>>

\* Every entry has been issued and has left its store, and no task holds one: the runs end here.
Quiescent == issued = Codes /\ live = {} /\ \A t \in Tasks : holds[t] = None

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ \E c \in Codes : Issue(c) \/ Expire(c)
    \/ \E t \in Tasks, c \in Codes : Take(t, c)
    \/ \E t \in Tasks, n \in Counters : Assert(t, n)
    \/ \E t \in Tasks : Release(t)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* No ceremony or link code is ever accepted by two tasks ("taken out of the store before
\* verification, so it is used at most once", SPEC-359 R7; "a code is taken once", R4;
\* `passkeys.rs::Ceremonies::take`, `linking.rs::LinkCodes::take`).
SingleUse == \A c \in Codes : Cardinality(takers[c]) <= 1

\* No two accepted assertions for the one credential share a counter the rule says must advance:
\* only a counter of zero may be accepted twice ("two assertions carrying one counter cannot both
\* pass", SPEC-359 R8; `passkeys.rs::Passkeys::advance`).
CounterAdvances ==
    \A x, y \in accepted : (x # y /\ x[3] = y[3]) => x[3] = 0

\* Evidence that the guarded situations are reached.
TwoTasksTakeTwoEntries ==
    \E c, d \in Codes :
        /\ c # d
        /\ takers[c] # {}
        /\ takers[d] # {}
        /\ takers[c] # takers[d]
AnAssertionAdvancesTheCounter == \E x \in accepted : x[3] > 0
TwoZeroCountersAreAccepted == Cardinality({x \in accepted : x[3] = 0}) >= 2
=============================================================================
