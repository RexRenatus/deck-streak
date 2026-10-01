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
(* holder still sends, a flusher that dies between its push and its       *)
(* settle, and a flush whose work answers an error, a push pending or not. *)
(*                                                                         *)
(* Modelled claims (item names, never line numbers):                       *)
(*   router.rs::flush_with   - a flush returns at once while the window is *)
(*                             quiet; an item older than the age limit is  *)
(*                             abandoned (strictly older: equal is sent);  *)
(*                             after an error it names what it pushed and  *)
(*                             gives back only what it never pushed.       *)
(*   router.rs::deliver      - a push that answered delivered puts its row *)
(*                             in the flush's pending set before the write *)
(*                             that settles it, every row of a recap too.  *)
(*   router.rs::render_pushed, router.rs::flush_reactions - the same, for  *)
(*                             a full render and for a held reaction.      *)
(*   sync_cycle.rs::flush    - the after-sync flush fires only after a     *)
(*                             sync that ran and succeeded.                *)
(*   ledger.rs::held         - the queue is read in one snapshot.          *)
(*   ledger.rs::settle_claimed - a sent item leaves the queue, by the      *)
(*                             token of the flush that claimed it.         *)
(*   ledger.rs::claim_held   - the take moves held -> sending with the     *)
(*                             flush's token, matching only held rows.     *)
(*   ledger.rs::lapsed_claims - a sending row whose claim lapsed is        *)
(*                             abandoned by name, never pushed.            *)
(*   ledger.rs::abandon_pushed - a row the failed flush pushed is          *)
(*                             abandoned by its token, its claim kept.     *)
(*   ledger.rs::release_claims - every other row the failed flush still    *)
(*                             claims goes back to held.                   *)
(*                                                                         *)
(* Claimed = TRUE is the design at head: the take claims each row it will  *)
(* send, and a lapsed foreign claim is abandoned as "may have been sent".   *)
(* Claimed = FALSE is the round-1 design (the lease alone), kept as the    *)
(* witness the same checker must find double-sending. NamedOnFail = TRUE   *)
(* is the design at head for a flush that fails; FALSE is the round-2      *)
(* design (every claimed row given back), kept as a witness too.           *)
(*                                                                         *)
(* Abstractions: time is a small integer clock; the window is quiet below  *)
(* QuietEnd; the owner's own window settings, the breaker, tiers, the      *)
(* queue bound and a push the transport answers as failed (held again for  *)
(* a retry) are not modelled. The recap line is one push that reaches      *)
(* several items; the model pushes one item at a time, and a failed flush  *)
(* names each item of a recap it pushed as it names its one pending item.  *)
(* A crash is a flusher that stops at any step and keeps its lease until   *)
(* it lapses.                                                              *)
(***************************************************************************)

\* @phx covers crates/notifications/src/router.rs anchor=flush_with digest=sha256:5f33914fd28d7a51534214bce2d92b9376944c8de0e7f0713d4e7970645ee40c
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=flush digest=sha256:1605c377223f24c33169b84f764e5f19827a3cc438966d0dac77678485a6caf9
\* @phx covers crates/notifications/src/ledger.rs anchor=held digest=sha256:97376a0d613769622abb5be37b7ed6ac5fb86000532d7113cdcd664fdeca0898
\* @phx covers crates/notifications/src/ledger.rs anchor=settle digest=sha256:95f76e72966a185f5e72a64fbfd4e573a25ad4d079dd0e131507645743c5c2d4
\* @phx covers crates/notifications/src/ledger.rs anchor=claim_held digest=sha256:42ff80c0883f9903a23c150616238b44851549386810b589999349c20366f6ff
\* @phx covers crates/notifications/src/ledger.rs anchor=lapsed_claims digest=sha256:c645ec2952b1f61c8953ae26db85d8f50e99b16cfea3a3d7022a963aef7d74d2
\* @phx covers crates/notifications/src/ledger.rs anchor=settle_claimed digest=sha256:930648efce24b7368e94285d8bebef82cc3fda59bfbc8a3f0b1883c0c717fb75
\* @phx covers crates/notifications/src/ledger.rs anchor=relatch digest=sha256:a8be3d6390eadc2935759690a58211a96e0e368dcfe83ee322c03326ae377bd0
\* @phx covers crates/notifications/src/ledger.rs anchor=abandon_lapsed digest=sha256:cc59e6b150e7166ee6babbe89c2f4024848c7682b317f67aa49e8209cb8a4500
\* @phx covers crates/notifications/src/ledger.rs anchor=abandon_claimed digest=sha256:e02f7fd65f3b6a5e08311ad6859f1535d3924f5affd7a72c05d2def7b8223b85
\* @phx covers crates/notifications/src/router.rs anchor=take_lease digest=sha256:3db2271e2474f29ffb47e4a5847b4e077a96c382e780b61014edc77da8977c03
\* @phx covers crates/notifications/src/router.rs anchor=deliver digest=sha256:0238633101b62fdcd80e0b4e38d9c2a16d4e0e2cb79bcb004a7dc29b266460c1
\* @phx covers crates/notifications/src/router.rs anchor=render_pushed digest=sha256:64d484a7511ef1cdfeb8749febe66a8c25c033b5f32357ea7e67befac33b88d4
\* @phx covers crates/notifications/src/router.rs anchor=flush_reactions digest=sha256:13137019ed5ae0b93cbbf7a3d4a06908b73df641bf54139acee344515d637845
\* @phx covers crates/notifications/src/ledger.rs anchor=abandon_pushed digest=sha256:27b148a1223b16cc2a4c97d882c56cdfec475427c64746a739ddce072f129d99
\* @phx covers crates/notifications/src/ledger.rs anchor=release_claims digest=sha256:6b4130909dc46b10d1ad4ed744290bb4b260c8a45190966f4d2fe224ed9ba05c
\* @phx covers crates/coordination/src/held_flush.rs anchor=perform digest=sha256:ce5564d067389de1f76446b44708cc4aa7e1fb1a73e6776eece05603afba8fe3
\* @phx cites #291
\* @phx property NoDoubleDelivery ramp=report
\* @phx property HeldReachesOrAbandons ramp=report
\* @phx witness witness/OnlyTheSyncFlusher.cfg kills=HeldReachesOrAbandons
\* @phx witness witness/NoSerialisation.cfg kills=NoDoubleDelivery
\* @phx witness witness/LeaseLapses.cfg kills=NoDoubleDelivery
\* @phx witness witness/ReleaseOnFail.cfg kills=NoDoubleDelivery
EXTENDS Naturals, FiniteSets

CONSTANTS Items,        \* the celebrations that may be raised
          Horizon,      \* the last clock position
          QuietEnd,     \* the window is quiet at every position below this
          AgeLimit,     \* a held item older than this is abandoned
          Lease,        \* how long a flush's lease lasts, in clock positions
          SyncAt,       \* the positions at which a sync-triggered flush may start
          SchedAt,      \* the positions at which a scheduled flush may start
          Serialised,   \* TRUE when a running flush's unlapsed lease excludes a second one
          Claimed,      \* TRUE when the take claims each row it sends (held -> sending)
          NamedOnFail   \* TRUE when a failed flush names its pushed row rather than give it back

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

\* The flush's work answers an error after its take committed (router.rs::deliver returns through
\* `?`): before any push, between two items, or after a push reached and before its settle
\* committed, so a push may be pending. flush_with then deletes the lease by its token. With
\* NamedOnFail, the design at head, it first names each row the flush pushed and did not settle,
\* matched by the flush's token (ledger.rs::abandon_pushed: abandoned with its claim kept, which a
\* recap reads as "may have been sent"), and only then gives every other row still sending under
\* the token back to held (ledger.rs::release_claims): a row whose push was attempted never returns
\* to held, and a row the flush never pushed is sent by a later flush. Without NamedOnFail, the
\* round-2 design, release_claims gives back every row, the pushed one included. The flush's set of
\* pushed rows is `pend`: a row it pushed and settled has left the queue, so the set's other rows
\* no longer match the token.
Fail(f) == LET mine == {i \in Items : Claimed /\ st[i] = "sending" /\ claim[i] = f}
               named == IF NamedOnFail THEN mine \cap pend[f] ELSE {}
           IN /\ pc[f] = "running"
              /\ pc' = [pc EXCEPT ![f] = "idle"]
              /\ snap' = [snap EXCEPT ![f] = {}]
              /\ pend' = [pend EXCEPT ![f] = {}]
              /\ st' = [i \in Items |->
                          IF i \in named THEN "abandoned"
                          ELSE IF i \in mine THEN "held"
                          ELSE st[i]]
              /\ claim' = [i \in Items |-> IF i \in mine \ named THEN "none" ELSE claim[i]]
              /\ lock' = IF lock = f THEN "none" ELSE lock
              /\ UNCHANGED <<t, claimUntil, heldAt, sends, leaseUntil, fired>>

\* The run ends at the last clock position, with no flush mid-way.
Finished == /\ t = Horizon
            /\ \A f \in Flushers : pc[f] = "idle"
            /\ UNCHANGED vars

Next == \/ Tick
        \/ \E i \in Items : Hold(i)
        \/ \E f \in Flushers : Take(f) \/ Skip(f) \/ Finish(f) \/ Crash(f) \/ Fail(f)
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
