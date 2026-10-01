------------------------------ MODULE HeldFlush ------------------------------
(***************************************************************************)
(* The held-notification queue and the flushers that drain it (#291).       *)
(*                                                                         *)
(* A celebration raised in the quiet window is held. A flusher takes the   *)
(* held queue only while the window is open (not quiet), abandons what is  *)
(* past its age limit, and sends the rest. Two kinds of flusher exist: the *)
(* flush that follows a sync, which fires at the sync times, and the       *)
(* scheduled flush, which fires at the scheduled times. The model checks   *)
(* each held item reaches the owner at most once and is never lost without *)
(* a name, across two overlapping flushers, a lease that lapses while its  *)
(* holder still sends, and a flusher that dies between its push and its    *)
(* settle.                                                                 *)
(*                                                                         *)
(* Modelled claims (item names, never line numbers):                       *)
(*   router.rs::flush_with   - a flush returns at once while the window is *)
(*                             quiet; an item older than the age limit is  *)
(*                             abandoned (strictly older: equal is sent).  *)
(*   sync_cycle.rs::flush    - the after-sync flush fires only after a     *)
(*                             sync that ran and succeeded.                *)
(*   ledger.rs::held         - the queue is read in one snapshot.          *)
(*   ledger.rs::settle       - a sent item leaves the queue, by the token  *)
(*                             of the flush that claimed it.               *)
(*   ledger.rs::claim_held   - the take moves held -> sending with the     *)
(*                             flush's token, matching only held rows.     *)
(*   ledger.rs::lapsed_claims - a sending row whose claim lapsed is        *)
(*                             abandoned by name, never pushed.            *)
(*                                                                         *)
(* Claimed = TRUE is the design at head: the take claims each row it will  *)
(* send, and a lapsed foreign claim is abandoned as "may have been sent".   *)
(* Claimed = FALSE is the round-1 design (the lease alone), kept as the    *)
(* witness the same checker must find double-sending.                      *)
(*                                                                         *)
(* Abstractions: time is a small integer clock; the window is quiet below  *)
(* QuietEnd; the owner's own window settings, the breaker, tiers, the      *)
(* recap line, the per-send failure path and the queue bound are not       *)
(* modelled. A crash is a flusher that stops at any step and keeps its     *)
(* lease until it lapses.                                                  *)
(***************************************************************************)

\* @phx covers crates/notifications/src/router.rs anchor=flush_with digest=sha256:b69fcee3ea8aae945713a66651cb12d6fa94ad2b04be7079dc66013aecf81a76
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=flush digest=sha256:1605c377223f24c33169b84f764e5f19827a3cc438966d0dac77678485a6caf9
\* @phx covers crates/notifications/src/ledger.rs anchor=held digest=sha256:b9b5e751c9664dc610325dce86c9e5f6adcaf60a009dbac7a705e201377559ae
\* @phx covers crates/notifications/src/ledger.rs anchor=settle digest=sha256:95f76e72966a185f5e72a64fbfd4e573a25ad4d079dd0e131507645743c5c2d4
\* @phx covers crates/notifications/src/router.rs anchor=take_lease digest=sha256:1ef34d6cd3f2f23db86a019bef7db16bb88ee22632d14c98037b4aae475c1121
\* @phx covers crates/notifications/src/router.rs anchor=deliver digest=sha256:0c33494c8ba6ded864660206d655abfa1c033156639826eff2f01562959cd332
\* @phx covers crates/coordination/src/held_flush.rs anchor=perform digest=sha256:ce5564d067389de1f76446b44708cc4aa7e1fb1a73e6776eece05603afba8fe3
\* @phx cites #291
\* @phx property NoDoubleDelivery ramp=report
\* @phx property HeldReachesOrAbandons ramp=report
\* @phx witness witness/OnlyTheSyncFlusher.cfg kills=HeldReachesOrAbandons
\* @phx witness witness/NoSerialisation.cfg kills=NoDoubleDelivery
\* @phx witness witness/LeaseLapses.cfg kills=NoDoubleDelivery
EXTENDS Naturals, FiniteSets

CONSTANTS Items,        \* the celebrations that may be raised
          Horizon,      \* the last clock position
          QuietEnd,     \* the window is quiet at every position below this
          AgeLimit,     \* a held item older than this is abandoned
          Lease,        \* how long a flush's lease lasts, in clock positions
          SyncAt,       \* the positions at which a sync-triggered flush may start
          SchedAt,      \* the positions at which a scheduled flush may start
          Serialised,   \* TRUE when a running flush's unlapsed lease excludes a second one
          Claimed       \* TRUE when the take claims each row it sends (held -> sending)

Flushers == {"sync", "sched"}
Quiet(t) == t < QuietEnd

VARIABLES t,          \* the clock
          st,         \* each item: "none", "held", "sending", "sent" or "abandoned"
          claim,      \* the flusher that claimed each item, or "none"
          claimUntil, \* the instant each claim lapses
          heldAt,     \* the clock position at which each item was held
          sends,      \* how many times each item reached the owner
          pc,         \* each flusher: "idle" or "running"
          snap,       \* the items a running flusher still has to push
          pend,       \* the item a flusher has pushed and not yet marked (at most one)
          lock,       \* the flusher that last took the lease, or "none"
          leaseUntil, \* the instant that lease lapses
          fired       \* the flushers whose trigger at this position has been answered

vars == <<t, st, claim, claimUntil, heldAt, sends, pc, snap, pend, lock, leaseUntil, fired>>

\* Whether flusher f has a trigger at the current clock position.
Fires(f) == IF f = "sync" THEN t \in SyncAt ELSE t \in SchedAt

\* Whether the lease is free at the current position: never taken, or lapsed (a lease that
\* lapses this instant is taken over).
LeaseFree == lock = "none" \/ t >= leaseUntil

TypeOK == /\ t \in 0..Horizon
          /\ st \in [Items -> {"none", "held", "sending", "sent", "abandoned"}]
          /\ claim \in [Items -> Flushers \cup {"none"}]
          /\ claimUntil \in [Items -> 0..(Horizon + Lease)]
          /\ heldAt \in [Items -> 0..Horizon]
          /\ sends \in [Items -> 0..Horizon]
          /\ pc \in [Flushers -> {"idle", "running"}]
          /\ snap \in [Flushers -> SUBSET Items]
          /\ pend \in [Flushers -> SUBSET Items]
          /\ \A f \in Flushers : Cardinality(pend[f]) <= 1
          /\ lock \in Flushers \cup {"none"}
          /\ leaseUntil \in 0..(Horizon + Lease)
          /\ fired \subseteq Flushers

Init == /\ t = 0
        /\ st = [i \in Items |-> "none"]
        /\ claim = [i \in Items |-> "none"]
        /\ claimUntil = [i \in Items |-> 0]
        /\ heldAt = [i \in Items |-> 0]
        /\ sends = [i \in Items |-> 0]
        /\ pc = [f \in Flushers |-> "idle"]
        /\ snap = [f \in Flushers |-> {}]
        /\ pend = [f \in Flushers |-> {}]
        /\ lock = "none"
        /\ leaseUntil = 0
        /\ fired = {}

\* The clock moves on only once every trigger at this position has been answered: a
\* scheduled or after-sync flush that is due runs or returns, and is never skipped.
Tick == /\ t < Horizon
        /\ \A f \in Flushers : Fires(f) => f \in fired
        /\ t' = t + 1
        /\ fired' = {}
        /\ UNCHANGED <<st, claim, claimUntil, heldAt, sends, pc, snap, pend, lock, leaseUntil>>

\* A celebration raised while the window is quiet is held.
Hold(i) == /\ Quiet(t)
           /\ st[i] = "none"
           /\ st' = [st EXCEPT ![i] = "held"]
           /\ heldAt' = [heldAt EXCEPT ![i] = t]
           /\ UNCHANGED <<t, claim, claimUntil, sends, pc, snap, pend, lock, leaseUntil, fired>>

\* A flush starts: it needs the window open and a free lease, reads the held queue in one
\* transaction, abandons what is past the age limit, and keeps the rest to send. When rows are
\* claimed, a sending row whose claim lapsed is abandoned by name ("may have been sent") and is
\* never pushed, and each row this flush keeps moves held -> sending with its token.
Take(f) == LET held == {i \in Items : st[i] = "held"}
               old == {i \in held : t - heldAt[i] > AgeLimit}
               keep == held \ old
               lapsed == IF Claimed
                         THEN {i \in Items : st[i] = "sending" /\ t >= claimUntil[i]}
                         ELSE {}
           IN /\ Fires(f)
              /\ f \notin fired
              /\ pc[f] = "idle"
              /\ ~Quiet(t)
              /\ (Serialised => LeaseFree)
              /\ pc' = [pc EXCEPT ![f] = "running"]
              /\ snap' = [snap EXCEPT ![f] = keep]
              /\ st' = [i \in Items |->
                          IF i \in old \/ i \in lapsed THEN "abandoned"
                          ELSE IF Claimed /\ i \in keep THEN "sending"
                          ELSE st[i]]
              /\ claim' = [i \in Items |-> IF Claimed /\ i \in keep THEN f ELSE claim[i]]
              /\ claimUntil' = [i \in Items |->
                                  IF Claimed /\ i \in keep THEN t + Lease ELSE claimUntil[i]]
              /\ lock' = IF Serialised THEN f ELSE lock
              /\ leaseUntil' = IF Serialised THEN t + Lease ELSE leaseUntil
              /\ fired' = fired \cup {f}
              /\ UNCHANGED <<t, heldAt, sends, pend>>

\* A due flush that returns at once: the window is quiet, or another flush holds the lease.
Skip(f) == /\ Fires(f)
           /\ f \notin fired
           /\ pc[f] = "idle"
           /\ \/ Quiet(t)
              \/ Serialised /\ ~LeaseFree
           /\ fired' = fired \cup {f}
           /\ UNCHANGED <<t, st, claim, claimUntil, heldAt, sends, pc, snap, pend, lock, leaseUntil>>

\* A running flush pushes one item of its snapshot: the push reaches the owner, and nothing
\* in the queue has changed yet.
Push(f, i) == /\ pc[f] = "running"
              /\ pend[f] = {}
              /\ i \in snap[f]
              /\ snap' = [snap EXCEPT ![f] = @ \ {i}]
              /\ pend' = [pend EXCEPT ![f] = {i}]
              /\ sends' = [sends EXCEPT ![i] = @ + 1]
              /\ UNCHANGED <<t, st, claim, claimUntil, heldAt, pc, lock, leaseUntil, fired>>

\* The flush settles the item it pushed. With rows claimed the settle matches the flush's own
\* claim, so a row another flush abandoned or claimed is left as it is; without, it is by id.
Mark(f, i) == /\ pc[f] = "running"
              /\ i \in pend[f]
              /\ pend' = [pend EXCEPT ![f] = {}]
              /\ st' = IF Claimed /\ ~(st[i] = "sending" /\ claim[i] = f)
                       THEN st
                       ELSE [st EXCEPT ![i] = "sent"]
              /\ UNCHANGED <<t, claim, claimUntil, heldAt, sends, pc, snap, lock, leaseUntil, fired>>

\* The release deletes the lease by its token, so a flush whose lease lapsed and was taken over
\* leaves the new holder's lease alone.
Finish(f) == /\ pc[f] = "running"
             /\ snap[f] = {}
             /\ pend[f] = {}
             /\ pc' = [pc EXCEPT ![f] = "idle"]
             /\ lock' = IF lock = f THEN "none" ELSE lock
             /\ UNCHANGED <<t, st, claim, claimUntil, heldAt, sends, snap, pend, leaseUntil, fired>>

\* A running flusher dies at any step: its snapshot and its pending push are gone, its rows keep
\* whatever state they were in, and its lease stays until it lapses.
Crash(f) == /\ pc[f] = "running"
            /\ pc' = [pc EXCEPT ![f] = "idle"]
            /\ snap' = [snap EXCEPT ![f] = {}]
            /\ pend' = [pend EXCEPT ![f] = {}]
            /\ UNCHANGED <<t, st, claim, claimUntil, heldAt, sends, lock, leaseUntil, fired>>

\* The run ends at the last clock position, with no flush mid-way.
Finished == /\ t = Horizon
            /\ \A f \in Flushers : pc[f] = "idle"
            /\ UNCHANGED vars

Next == \/ Tick
        \/ \E i \in Items : Hold(i)
        \/ \E f \in Flushers : Take(f) \/ Skip(f) \/ Finish(f) \/ Crash(f)
        \/ \E f \in Flushers, i \in Items : Push(f, i) \/ Mark(f, i)
        \/ Finished

Spec == Init /\ [][Next]_vars

\* No held item reaches the owner twice.
NoDoubleDelivery == \A i \in Items : sends[i] <= 1

\* At the end of the run no item is still held, and a sending one has a claim that lapses no earlier
\* than the end, so no trigger could have abandoned it: every
\* other item was delivered, or abandoned by name. A claim that lapsed inside the run and was
\* never abandoned is a silent loss.
HeldReachesOrAbandons == (t = Horizon /\ \A f \in Flushers : pc[f] = "idle")
                         => \A i \in Items : /\ st[i] # "held"
                                             /\ st[i] = "sending" => claimUntil[i] >= Horizon

=============================================================================
