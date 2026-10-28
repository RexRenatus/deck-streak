------------------------------- MODULE OpLog -------------------------------
\* @phx cites #15
\* @phx property NoLostReview ramp=report
\* @phx property ExactlyOnce ramp=report
\* @phx property MonotoneLog ramp=report
\* @phx property NoSkip ramp=report
\* @phx property Convergence ramp=report
\* @phx property CanonicalCardFold ramp=report
\* @phx property NoSilentOverwrite ramp=report
\* @phx property FieldConverges ramp=report
\* @phx property CausalHlc ramp=report
\* @phx property RebootstrapKeepsPending ramp=report
\* @phx property RevokedDeviceRefused ramp=report
\* @phx property RetractOnlyOwn ramp=report
\* @phx property LocalIsPrefix ramp=report
\* @phx property UndoOwnOnly ramp=report
\* @phx property CausalHlcAtMake ramp=report
\* @phx witness witness/an-answer-written-before-its-commit.cfg kills=NoLostReview
\* @phx witness witness/a-pending-op-dropped-on-send.cfg kills=NoLostReview
\* @phx witness witness/a-judge-with-no-origin-seq-key.cfg kills=ExactlyOnce
\* @phx witness witness/a-seq-assigned-outside-the-writers-transaction.cfg kills=NoSkip
\* @phx witness witness/a-field-won-by-arrival-order.cfg kills=FieldConverges
\* @phx witness witness/a-loser-not-recorded.cfg kills=NoSilentOverwrite
\* @phx witness witness/a-card-folded-in-server-order.cfg kills=CanonicalCardFold
\* @phx witness witness/a-clock-that-ignores-what-it-saw.cfg kills=CausalHlc
\* @phx witness witness/a-replace-that-drops-pending.cfg kills=RebootstrapKeepsPending
\* @phx witness witness/a-revoked-token-still-pushes.cfg kills=RevokedDeviceRefused
\* @phx witness witness/an-undo-that-deletes-from-the-log.cfg kills=MonotoneLog
\* @phx witness witness/a-pull-that-skips-an-operation.cfg kills=LocalIsPrefix
\* @phx witness witness/a-pull-that-skips-never-reaches-the-head.cfg kills=Convergence
\* @phx witness witness/a-retract-naming-another-devices-operation.cfg kills=RetractOnlyOwn
\* @phx witness witness/an-undo-blind-to-another-devices-write.cfg kills=UndoOwnOnly
\* @phx witness witness/a-clock-past-only-the-oldest-clock-it-saw.cfg kills=CausalHlcAtMake
\*
\* The operation log, ahead of its code (SPEC-504, ADR-522). The server keeps one log of
\* operations; each device keeps a durable queue of the operations it made and has not had
\* answered, a cursor into the log and its pulled copy of it. Nothing in the tree runs this protocol
\* yet: the sync core that will lands later, and then gains covers lines on its judge, the
\* writer's commit and the device's pull and verdict handlers, each re-read against this model.
\*
\* What the model abstracts, and why:
\* - An operation is a review of the one card, an edit of one field of the one note, or an undo of
\*   the device's own earlier operation. It carries its origin device, its per-device sequence
\*   number, its hybrid clock, the clock it had seen when it was made (`saw`), and, once
\*   committed, its log position `pos`. Its key is (hlc, origin, seq), a strict total order.
\* - The device's base is abstracted as its pulled copy `local`: the card's fold and each field's
\*   register are folds over it in arrival order (`CardFold`, `RegVal`), the way the device applies
\*   what it pulls.
\* - A push is sent and judged in one step: the judge reads the token, then the (origin, seq) key.
\*   The device sends its queue's head and waits for its answer before the next.
\* - A crash of the server loses what is uncommitted and every answer not yet read; a crash of a
\*   device loses the answers it had not read. Both are bounded.
\* - Each recorded defect is one value of `Bug`; "none" is the designed protocol. Each witness
\*   switches one on.
\* - No SYMMETRY, CONSTRAINT or VIEW narrows the space.

EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS Devices,        \* device ids, numbers, so they order keys
          Fields,         \* the note's fields
          Script,         \* [Devices -> Seq(template)]: what each device makes, in order
          MaxHlc,         \* a bound on the clock
          MaxCrash,       \* device crashes, each device
          MaxServerCrash, \* server crashes
          MaxLoss,        \* answers lost, at most one per operation when 1
          Revocable,      \* the devices whose token can be revoked
          Bug             \* the recorded defect switched on, or "none"

VARIABLES log,       \* the server's committed log, in commit order
          inflight,  \* operations judged and not yet committed (lost on a server crash)
          acks,      \* answers sent and not yet read: [id, v]
          rejected,  \* ids refused by the token check, kept with their bytes
          conflicts, \* ids of concurrent losers kept in the note's conflict record
          nextPos,   \* a position counter read only by the "SeqOutside" defect
          next,      \* [Devices -> Nat]: the next sequence number, durable
          pending,   \* [Devices -> Seq(Op)]: made and not yet answered, durable
          cursor,    \* [Devices -> Nat]: the last log position pulled, durable
          local,     \* [Devices -> Seq(Op)]: the pulled copy, durable
          hlcLast,   \* [Devices -> Nat]: the device's last clock, durable
          revoked,   \* [Devices -> BOOLEAN]: the device's token is revoked
          dcrash,    \* [Devices -> Nat]: crashes so far
          scrash,    \* server crashes so far
          lost,      \* ids whose answer was lost
          made       \* history: every operation made

vars == <<log, inflight, acks, rejected, conflicts, nextPos, next, pending, cursor, local,
          hlcLast, revoked, dcrash, scrash, lost, made>>

None == "none"
Vals == <<"a", "b", "c">>
Review == [kind |-> "review", field |-> None, val |-> None, of |-> 0]
Edit(f, v) == [kind |-> "edit", field |-> f, val |-> v, of |-> 0]
Undo(k) == [kind |-> "undo", field |-> None, val |-> None, of |-> k]

\* The configurations' scripts.
MainScript == [d \in Devices |-> <<Review, Edit("f0", Vals[d])>>]
ThreeScript == [d \in Devices |-> IF d = 1 THEN <<Review>> ELSE <<Edit("f0", Vals[d])>>]
LiveScript == [d \in Devices |-> IF d = 1 THEN <<Review>> ELSE <<Edit("f0", "b")>>]
EditsScript == [d \in Devices |-> IF d = 1 THEN <<Edit("f0", "a"), Undo(1)>>
                                  ELSE <<Edit("f0", "b"), Edit("f1", "c")>>]

Range(s) == {s[i] : i \in DOMAIN s}
Id(o) == <<o.origin, o.seq>>
Ids(s) == {Id(s[i]) : i \in DOMAIN s}
Max2(a, b) == IF a >= b THEN a ELSE b
KeyLt(a, b) == \/ a.hlc < b.hlc
               \/ a.hlc = b.hlc /\ a.origin < b.origin
               \/ a.hlc = b.hlc /\ a.origin = b.origin /\ a.seq < b.seq

Undone(s) == {s[i].target : i \in {j \in DOMAIN s : s[j].kind = "undo"}}
LiveEdits(s, f) ==
    {s[i] : i \in {j \in DOMAIN s : s[j].kind = "edit" /\ s[j].field = f /\ Id(s[j]) \notin Undone(s)}}
LiveReviews(s) ==
    {s[i] : i \in {j \in DOMAIN s : s[j].kind = "review" /\ Id(s[j]) \notin Undone(s)}}
Winner(S) == CHOOSE w \in S : \A x \in S \ {w} : KeyLt(x, w)
WinnerVal(S) == IF S = {} THEN "init" ELSE Winner(S).val

SeenOf(d) == LET hs == {local[d][i].hlc : i \in DOMAIN local[d]}
             IN IF hs = {} THEN 0
                ELSE IF Bug = "SeenLeast" THEN CHOOSE m \in hs : \A x \in hs : m <= x
                ELSE CHOOSE m \in hs : \A x \in hs : x <= m

\* The device's apply of an edit or an undo to one field's register, in arrival order.
RECURSIVE RegFold(_, _, _, _)
RegFold(s, f, cands, cur) ==
    IF s = <<>> THEN cur
    ELSE LET o == Head(s) IN
         IF o.kind = "edit" /\ o.field = f THEN
             LET c2 == cands \cup {o}
             IN RegFold(Tail(s), f, c2, IF Bug = "ArrivalWins" THEN o.val ELSE WinnerVal(c2))
         ELSE IF o.kind = "undo" /\ o.field = f THEN
             LET c2 == {x \in cands : Id(x) # o.target}
             IN RegFold(Tail(s), f, c2, IF Bug = "UndoBlind" THEN o.prev ELSE WinnerVal(c2))
         ELSE RegFold(Tail(s), f, cands, cur)
RegVal(d, f) == RegFold(local[d], f, {}, "init")

\* The device's fold of the card's reviews, in arrival order: each review is placed by its key.
InsertSorted(o, s) ==
    LET k == Cardinality({j \in DOMAIN s : KeyLt(s[j], o)})
    IN SubSeq(s, 1, k) \o <<o>> \o SubSeq(s, k + 1, Len(s))
RECURSIVE CardFold(_, _)
CardFold(s, acc) ==
    IF s = <<>> THEN acc
    ELSE LET o == Head(s) IN
         CardFold(Tail(s),
                  IF o.kind = "review"
                  THEN IF Bug = "ServerOrderFold" THEN Append(acc, o) ELSE InsertSorted(o, acc)
                  ELSE IF o.kind = "undo" THEN SelectSeq(acc, LAMBDA x : Id(x) # o.target)
                  ELSE acc)

Behind(d) == {i \in DOMAIN log : log[i].pos > cursor[d]}
MaxPos == LET ps == {log[i].pos : i \in DOMAIN log}
          IN IF ps = {} THEN 0 ELSE CHOOSE m \in ps : \A x \in ps : x <= m

Init == /\ log = <<>>
        /\ inflight = {}
        /\ acks = {}
        /\ rejected = {}
        /\ conflicts = {}
        /\ nextPos = 1
        /\ next = [d \in Devices |-> 1]
        /\ pending = [d \in Devices |-> <<>>]
        /\ cursor = [d \in Devices |-> 0]
        /\ local = [d \in Devices |-> <<>>]
        /\ hlcLast = [d \in Devices |-> 0]
        /\ revoked = [d \in Devices |-> FALSE]
        /\ dcrash = [d \in Devices |-> 0]
        /\ scrash = 0
        /\ lost = {}
        /\ made = {}

\* Make: the device records a review, an edit or an undo of its own earlier write, durably, with
\* a clock past both its own last clock and every clock it has seen.
Make(d) ==
    /\ next[d] <= Len(Script[d])
    /\ LET t == Script[d][next[d]]
           saw == SeenOf(d)
           h == IF Bug = "BlindClock" THEN hlcLast[d] + 1 ELSE Max2(hlcLast[d], saw) + 1
           others == {Id(local[d][i]) : i \in {j \in DOMAIN local[d] :
                          local[d][j].origin # d /\ local[d][j].kind = "edit"}}
           tgt == IF t.kind # "undo" THEN <<0, 0>>
                  ELSE IF Bug = "RetractOther" /\ others # {} THEN CHOOSE x \in others : TRUE
                  ELSE <<d, t.of>>
           mine == IF t.kind = "undo" THEN CHOOSE m \in made : Id(m) = <<d, t.of>> ELSE Review
           fld == IF t.kind = "undo" THEN mine.field ELSE t.field
           prv == IF t.kind = "undo" THEN mine.prev
                  ELSE IF t.kind = "edit" THEN RegVal(d, t.field) ELSE None
           op == [origin |-> d, seq |-> next[d], hlc |-> h, saw |-> saw, kind |-> t.kind,
                  field |-> fld, val |-> t.val, prev |-> prv, target |-> tgt, pos |-> 0]
       IN /\ h <= MaxHlc
          /\ made' = made \cup {op}
          /\ pending' = [pending EXCEPT ![d] = Append(@, op)]
          /\ next' = [next EXCEPT ![d] = @ + 1]
          /\ hlcLast' = [hlcLast EXCEPT ![d] = h]
    /\ UNCHANGED <<log, inflight, acks, rejected, conflicts, nextPos, cursor, local, revoked,
                   dcrash, scrash, lost>>

\* Push: the device sends its queue's head; the judge reads the token, then the (origin, seq) key.
Push(d) ==
    /\ pending[d] # <<>>
    /\ LET o == Head(pending[d]) IN
       /\ ~(\E x \in inflight : Id(x) = Id(o))
       /\ ~(\E a \in acks : a.id = Id(o))
       /\ IF revoked[d] /\ Bug # "RevokedPushes" THEN
             /\ rejected' = rejected \cup {Id(o)}
             /\ acks' = acks \cup {[id |-> Id(o), v |-> "rejected"]}
             /\ UNCHANGED <<inflight, nextPos, pending>>
          ELSE IF Id(o) \in Ids(log) /\ Bug # "NoKey" THEN
             /\ acks' = acks \cup {[id |-> Id(o), v |-> "duplicate"]}
             /\ UNCHANGED <<inflight, nextPos, pending, rejected>>
          ELSE
             /\ inflight' = inflight \cup {[o EXCEPT !.pos = IF Bug = "SeqOutside" THEN nextPos ELSE 0]}
             /\ nextPos' = IF Bug = "SeqOutside" THEN nextPos + 1 ELSE nextPos
             /\ acks' = IF Bug = "AnswerBeforeCommit"
                        THEN acks \cup {[id |-> Id(o), v |-> "accepted"]} ELSE acks
             /\ pending' = IF Bug = "DropOnSend" THEN [pending EXCEPT ![d] = Tail(@)] ELSE pending
             /\ UNCHANGED rejected
    /\ UNCHANGED <<log, conflicts, next, cursor, local, hlcLast, revoked, dcrash, scrash, lost, made>>

\* Commit: the writer appends the judged operation at the next position, records the loser of a
\* concurrent edit, and answers.
Commit(o) ==
    /\ o \in inflight
    /\ LET p == IF Bug = "SeqOutside" THEN o.pos ELSE Len(log) + 1
           live == LiveEdits(log, o.field)
       IN /\ log' = IF o.kind = "undo" /\ Bug = "UndoDeletes"
                    THEN SelectSeq(log, LAMBDA x : Id(x) # o.target)
                    ELSE Append(log, [o EXCEPT !.pos = p])
          /\ conflicts' = IF /\ o.kind = "edit" /\ Bug # "NoLoser" /\ live # {}
                             /\ Winner(live).origin # o.origin
                          THEN conflicts \cup {IF KeyLt(Winner(live), o) THEN Id(Winner(live)) ELSE Id(o)}
                          ELSE conflicts
    /\ inflight' = inflight \ {o}
    /\ acks' = IF Bug = "AnswerBeforeCommit" THEN acks ELSE acks \cup {[id |-> Id(o), v |-> "accepted"]}
    /\ UNCHANGED <<rejected, nextPos, next, pending, cursor, local, hlcLast, revoked, dcrash, scrash,
                   lost, made>>

\* Verdict: the device reads an answer and drops the answered operation from its queue.
Verdict(d) ==
    \E a \in acks :
       /\ a.id[1] = d
       /\ acks' = acks \ {a}
       /\ pending' = [pending EXCEPT ![d] = SelectSeq(@, LAMBDA x : Id(x) # a.id)]
       /\ UNCHANGED <<log, inflight, rejected, conflicts, nextPos, next, cursor, local, hlcLast,
                      revoked, dcrash, scrash, lost, made>>

LoseReply ==
    \E a \in acks :
       /\ MaxLoss > 0
       /\ a.id \notin lost
       /\ acks' = acks \ {a}
       /\ lost' = lost \cup {a.id}
       /\ UNCHANGED <<log, inflight, rejected, conflicts, nextPos, next, pending, cursor, local,
                      hlcLast, revoked, dcrash, scrash, made>>

\* Pull: the device copies the committed operation at the next position past its cursor.
Pull(d) ==
    /\ Behind(d) # {}
    /\ \E i \in Behind(d) :
          /\ Bug = "PullSkips" \/ \A j \in Behind(d) : log[i].pos <= log[j].pos
          /\ local' = [local EXCEPT ![d] = Append(@, log[i])]
          /\ cursor' = [cursor EXCEPT ![d] = log[i].pos]
    /\ UNCHANGED <<log, inflight, acks, rejected, conflicts, nextPos, next, pending, hlcLast,
                   revoked, dcrash, scrash, lost, made>>

\* Rebootstrap: a device that is behind replaces its base with the server's log, keeping its queue.
Rebootstrap(d) ==
    /\ Behind(d) # {}
    /\ local' = [local EXCEPT ![d] = log]
    /\ cursor' = [cursor EXCEPT ![d] = MaxPos]
    /\ pending' = IF Bug = "ReplaceDropsPending" THEN [pending EXCEPT ![d] = <<>>] ELSE pending
    /\ UNCHANGED <<log, inflight, acks, rejected, conflicts, nextPos, next, hlcLast, revoked,
                   dcrash, scrash, lost, made>>

Revoke(d) ==
    /\ d \in Revocable
    /\ ~revoked[d]
    /\ revoked' = [revoked EXCEPT ![d] = TRUE]
    /\ UNCHANGED <<log, inflight, acks, rejected, conflicts, nextPos, next, pending, cursor, local,
                   hlcLast, dcrash, scrash, lost, made>>

CrashDevice(d) ==
    /\ dcrash[d] < MaxCrash
    /\ dcrash' = [dcrash EXCEPT ![d] = @ + 1]
    /\ acks' = {a \in acks : a.id[1] # d}
    /\ UNCHANGED <<log, inflight, rejected, conflicts, nextPos, next, pending, cursor, local,
                   hlcLast, revoked, scrash, lost, made>>

CrashServer ==
    /\ scrash < MaxServerCrash
    /\ scrash' = scrash + 1
    /\ inflight' = {}
    /\ acks' = {}
    /\ UNCHANGED <<log, rejected, conflicts, nextPos, next, pending, cursor, local, hlcLast, revoked,
                   dcrash, lost, made>>

Quiet == \A d \in Devices : next[d] > Len(Script[d])

\* The intended end: everything made, answered and pulled. The step stutters only there.
Done == /\ Quiet
        /\ inflight = {}
        /\ acks = {}
        /\ \A d \in Devices : pending[d] = <<>> /\ Behind(d) = {}
        /\ UNCHANGED vars

Next == \/ \E d \in Devices : Make(d) \/ Push(d) \/ Verdict(d) \/ Pull(d) \/ Rebootstrap(d)
                              \/ Revoke(d) \/ CrashDevice(d)
        \/ \E o \in inflight : Commit(o)
        \/ LoseReply
        \/ CrashServer
        \/ Done

Fairness == /\ \A d \in Devices : WF_vars(Push(d)) /\ WF_vars(Verdict(d)) /\ WF_vars(Pull(d))
            /\ WF_vars(\E o \in inflight : Commit(o))

Spec == Init /\ [][Next]_vars /\ Fairness

\* ---- Properties ----

\* Every review made ends in the server's log, or in the refusals with its bytes, or is still in
\* its device's durable queue.
NoLostReview ==
    \A o \in made : o.kind = "review" =>
        \/ Id(o) \in Ids(pending[o.origin])
        \/ Id(o) \in Ids(log)
        \/ Id(o) \in rejected

\* No (origin, seq) is applied twice: a re-push reads Duplicate.
ExactlyOnce ==
    \A i, j \in DOMAIN log : i # j => Id(log[i]) # Id(log[j])

\* The log only extends, and every cursor only grows.
MonotoneLog ==
    [][/\ Len(log') >= Len(log)
       /\ SubSeq(log', 1, Len(log)) = log
       /\ \A d \in Devices : cursor'[d] >= cursor[d]]_vars

\* A pull by cursor sees every committed operation at or before the cursor.
NoSkip ==
    \A d \in Devices : \A i \in DOMAIN log :
        log[i].pos <= cursor[d] => Id(log[i]) \in Ids(local[d])

\* Two quiescent devices reach the head with nothing left to send (under the fairness in Spec).
Convergence ==
    Quiet ~> \A d \in Devices : local[d] = log /\ pending[d] = <<>>

\* A card's fold is its live review set in key order, whatever order the reviews arrived in.
CanonicalCardFold ==
    \A d \in Devices :
        LET f == CardFold(local[d], <<>>)
        IN /\ \A i \in 1..(Len(f) - 1) : KeyLt(f[i], f[i + 1])
           /\ Range(f) = LiveReviews(local[d])

\* Every committed edit that lost to a concurrent one is in the conflict record.
NoSilentOverwrite ==
    \A f \in Fields : \A e \in LiveEdits(log, f) :
        e = Winner(LiveEdits(log, f)) \/ Id(e) \in conflicts

\* Each field's register is the max-key live edit, whatever the arrival order.
FieldConverges ==
    \A d \in Devices, f \in Fields : RegVal(d, f) = WinnerVal(LiveEdits(local[d], f))

\* An operation made after its device saw a clock is past it; a device's own clock only rises.
CausalHlc ==
    \A o \in made :
        /\ o.hlc > o.saw
        /\ \A p \in made : p.origin = o.origin /\ p.seq < o.seq => p.hlc < o.hlc

\* A queued operation leaves its device's queue only with its own answer: a base replace keeps it.
RebootstrapKeepsPending ==
    [][\A d \in Devices : \A o \in Range(pending[d]) :
          Id(o) \notin Ids(pending'[d]) =>
              \E a \in acks : a.id = Id(o) /\ a.id[1] = d /\ a \notin acks']_vars

\* The judge admits nothing from a device whose token is revoked.
RevokedDeviceRefused ==
    [][\A o \in inflight' \ inflight : ~revoked[o.origin]]_vars

\* An undo names only its own device's operation.
RetractOnlyOwn ==
    \A i \in DOMAIN log : log[i].kind = "undo" => log[i].target[1] = log[i].origin

\* Each device's copy is exactly the log's prefix up to its cursor.
LocalIsPrefix ==
    \A d \in Devices : local[d] = SubSeq(log, 1, cursor[d])

\* An undo touches only its own write: after an undo, the field reads the max-key write still
\* live, never a value restored over another device's write.
UndoOwnOnly ==
    \A d \in Devices, f \in Fields :
        (\E i \in DOMAIN local[d] : local[d][i].kind = "undo" /\ local[d][i].field = f)
            => RegVal(d, f) = WinnerVal(LiveEdits(local[d], f))

\* The causal claim read from the state: the step that makes an operation gives it a clock past
\* every clock in its device's pulled copy at that step. It reads the copy, never the operation's
\* own `saw`, so a wrong seen clock written into `saw` cannot agree with itself here.
CausalHlcAtMake ==
    [][\A o \in made' \ made :
          \A i \in DOMAIN local[o.origin] : local[o.origin][i].hlc < o.hlc]_vars

=============================================================================
