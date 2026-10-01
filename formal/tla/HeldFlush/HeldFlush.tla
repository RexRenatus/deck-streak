------------------------------ MODULE HeldFlush ------------------------------
(***************************************************************************)
(* The held-notification queue and the flushers that drain it (#291).       *)
(*                                                                         *)
(* A celebration raised in the quiet window is held. A flusher takes the   *)
(* held queue only while the window is open (not quiet), abandons what is  *)
(* past its age limit, and sends the rest. Two kinds of flusher exist: the *)
(* flush that follows a sync, which fires at the sync times, and the       *)
(* scheduled flush, which fires at the scheduled times. The model checks   *)
(* each held item reaches the owner once or is abandoned by name, and      *)
(* that no item is sent twice across two overlapping flushers.             *)
(*                                                                         *)
(* Modelled claims (item names, never line numbers):                       *)
(*   router.rs::flush_with   - a flush returns at once while the window is *)
(*                             quiet; an item older than the age limit is  *)
(*                             abandoned (strictly older: equal is sent).  *)
(*   sync_cycle.rs::flush    - the after-sync flush fires only after a     *)
(*                             sync that ran and succeeded.                *)
(*   ledger.rs::held         - the queue is read in one snapshot.          *)
(*   ledger.rs::settle       - a sent item leaves the queue.               *)
(*                                                                         *)
(* Abstractions: time is a small integer clock; the window is quiet below  *)
(* QuietEnd; the owner's own window settings, the breaker, tiers, the      *)
(* recap line and the per-send failure path are not modelled.              *)
(***************************************************************************)
\* @phx covers crates/notifications/src/router.rs anchor=flush_with digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=flush digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/notifications/src/ledger.rs anchor=held digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/notifications/src/ledger.rs anchor=settle digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/notifications/src/router.rs anchor=take_lease digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/notifications/src/router.rs anchor=deliver digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/held_flush.rs anchor=perform digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #291
\* @phx property NoDoubleDelivery ramp=report
\* @phx property HeldReachesOrAbandons ramp=report
\* @phx witness witness/OnlyTheSyncFlusher.cfg kills=HeldReachesOrAbandons
\* @phx witness witness/NoSerialisation.cfg kills=NoDoubleDelivery
EXTENDS Naturals, FiniteSets

CONSTANTS Items,        \* the celebrations that may be raised
          Horizon,      \* the last clock position
          QuietEnd,     \* the window is quiet at every position below this
          AgeLimit,     \* a held item older than this is abandoned
          SyncAt,       \* the positions at which a sync-triggered flush may start
          SchedAt,      \* the positions at which a scheduled flush may start
          Serialised    \* TRUE when a running flush excludes a second one

Flushers == {"sync", "sched"}
Quiet(t) == t < QuietEnd

VARIABLES t,        \* the clock
          st,       \* each item: "none", "held", "sent" or "abandoned"
          heldAt,   \* the clock position at which each item was held
          sends,    \* how many times each item was sent
          pc,       \* each flusher: "idle" or "running"
          snap,     \* the items a running flusher still has to send
          lock,     \* the flusher holding the serialisation, or "none"
          fired     \* the flushers whose trigger at this position has been answered

vars == <<t, st, heldAt, sends, pc, snap, lock, fired>>

\* Whether flusher f has a trigger at the current clock position.
Fires(f) == IF f = "sync" THEN t \in SyncAt ELSE t \in SchedAt

TypeOK == /\ t \in 0..Horizon
          /\ st \in [Items -> {"none", "held", "sent", "abandoned"}]
          /\ heldAt \in [Items -> 0..Horizon]
          /\ sends \in [Items -> 0..2]
          /\ pc \in [Flushers -> {"idle", "running"}]
          /\ snap \in [Flushers -> SUBSET Items]
          /\ lock \in Flushers \cup {"none"}
          /\ fired \subseteq Flushers

Init == /\ t = 0
        /\ st = [i \in Items |-> "none"]
        /\ heldAt = [i \in Items |-> 0]
        /\ sends = [i \in Items |-> 0]
        /\ pc = [f \in Flushers |-> "idle"]
        /\ snap = [f \in Flushers |-> {}]
        /\ lock = "none"
        /\ fired = {}

\* The clock moves on only once every trigger at this position has been answered: a
\* scheduled or after-sync flush that is due runs or returns, and is never skipped.
Tick == /\ t < Horizon
        /\ \A f \in Flushers : Fires(f) => f \in fired
        /\ t' = t + 1
        /\ fired' = {}
        /\ UNCHANGED <<st, heldAt, sends, pc, snap, lock>>

\* A celebration raised while the window is quiet is held.
Hold(i) == /\ Quiet(t)
           /\ st[i] = "none"
           /\ st' = [st EXCEPT ![i] = "held"]
           /\ heldAt' = [heldAt EXCEPT ![i] = t]
           /\ UNCHANGED <<t, sends, pc, snap, lock, fired>>

\* A flush starts: it needs the window open, reads the held queue in one snapshot,
\* abandons what is past the age limit and keeps the rest to send.
Take(f) == LET held == {i \in Items : st[i] = "held"}
               old == {i \in held : t - heldAt[i] > AgeLimit}
           IN /\ Fires(f)
              /\ f \notin fired
              /\ pc[f] = "idle"
              /\ ~Quiet(t)
              /\ (Serialised => lock = "none")
              /\ pc' = [pc EXCEPT ![f] = "running"]
              /\ snap' = [snap EXCEPT ![f] = held \ old]
              /\ st' = [i \in Items |-> IF i \in old THEN "abandoned" ELSE st[i]]
              /\ lock' = IF Serialised THEN f ELSE lock
              /\ fired' = fired \cup {f}
              /\ UNCHANGED <<t, heldAt, sends>>

\* A due flush that returns at once: the window is quiet, or another flush holds the exclusion.
Skip(f) == /\ Fires(f)
           /\ f \notin fired
           /\ pc[f] = "idle"
           /\ \/ Quiet(t)
              \/ Serialised /\ lock # "none"
           /\ fired' = fired \cup {f}
           /\ UNCHANGED <<t, st, heldAt, sends, pc, snap, lock>>

\* A running flush sends one item of its snapshot; a sent item leaves the queue.
Send(f, i) == /\ pc[f] = "running"
              /\ i \in snap[f]
              /\ snap' = [snap EXCEPT ![f] = @ \ {i}]
              /\ sends' = [sends EXCEPT ![i] = @ + 1]
              /\ st' = [st EXCEPT ![i] = "sent"]
              /\ UNCHANGED <<t, heldAt, pc, lock, fired>>

Finish(f) == /\ pc[f] = "running"
             /\ snap[f] = {}
             /\ pc' = [pc EXCEPT ![f] = "idle"]
             /\ lock' = IF lock = f THEN "none" ELSE lock
             /\ UNCHANGED <<t, st, heldAt, sends, snap, fired>>

\* The run ends at the last clock position, with no flush mid-way.
Finished == /\ t = Horizon
            /\ \A f \in Flushers : pc[f] = "idle"
            /\ UNCHANGED vars

Next == \/ Tick
        \/ \E i \in Items : Hold(i)
        \/ \E f \in Flushers : Take(f) \/ Skip(f) \/ Finish(f)
        \/ \E f \in Flushers, i \in Items : Send(f, i)
        \/ Finished

Spec == Init /\ [][Next]_vars

\* No held item is sent twice.
NoDoubleDelivery == \A i \in Items : sends[i] <= 1

\* At the end of the run every item is sent once or abandoned: none is still held.
HeldReachesOrAbandons == (t = Horizon /\ \A f \in Flushers : pc[f] = "idle")
                         => \A i \in Items : st[i] # "held"

=============================================================================
