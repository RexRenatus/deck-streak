---------------------------- MODULE WalletFloor ----------------------------
\* @phx covers crates/economy/src/wallet.rs anchor=credit_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=credit_once_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=settle_mint_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=purchase_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=debit_floored_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=refund_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=debit_capped_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=insert digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/economy/src/wallet.rs anchor=summed digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/kernel/src/db.rs anchor=write digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #106
\* @phx property FloorHolds ramp=report
\* @phx property OneMovementPerKey ramp=report
\* @phx property CreditOnceEver ramp=report
\* @phx property SettledMintNeverFalls ramp=report
\* @phx witness witness/a-floor-read-outside-the-write.cfg kills=FloorHolds
\* @phx witness witness/a-once-ever-guard-read-on-its-own-day.cfg kills=CreditOnceEver
\* @phx witness witness/a-ledger-with-no-unique-key.cfg kills=OneMovementPerKey
\* @phx witness witness/a-closed-day-mint-that-follows-its-base.cfg kills=SettledMintNeverFalls
(***************************************************************************)
\* The coin wallet (#106): N tasks, each running one of the wallet's seven ports once, over one
\* coin_ledger, while the fold closes study days. The balance is never stored; it is the ledger's
\* sum, read each time (wallet.rs::summed).
\*
\* Each port is ONE transaction of the code, and the model takes it in three steps:
\* - Begin: the task takes the write lock. db.rs::write opens every write with BEGIN IMMEDIATE,
\*   which takes SQLite's write lock at its first statement, so no other write runs until the
\*   holder ends (wallet.rs module doc: "one read and one write inside ONE BEGIN IMMEDIATE");
\* - ReadWrite: the port's reads (the key, the balance, the day's sums) and its one write, all
\*   from the ledger as the lock holder sees it;
\* - Commit: the task releases the lock.
\* A defect switch, ReadInsideWrite = FALSE, gives the three ports that read the balance a Snap step
\* before Begin, where the balance is read outside the write; the fixed model has no Snap step.
\*
\* The ledger maps each key, (study day, source), to the movements that hold it, in insert order.
\* One reference per source stands for the code's (source, reference) pairs: the properties
\* compare keys, and two references of one source never share a key. The sources are the mint,
\* a once-ever credit, a credit, a refund and a debit, so a key never mixes two ports' meanings.
\* A request of 0 or less writes nothing in every port (the `<= 0` arms), so the model draws
\* requests from 1..MaxAmount; the mint's amount is drawn from 0..MaxAmount, since a settle of 0
\* lowers a held mint. The loss cap's float share is the integer (w * 3) \div 10, which equals the
\* code's truncation for every wallet this model reaches.
(***************************************************************************)
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS ReadInsideWrite, UniqueKey, OnceGuardAllDays, ClosedKeepsHeld, NActors, NDays, MaxAmount

Actors == 1..NActors
Days == 1..NDays
Amounts == 1..MaxAmount
Sources == {"mint", "once", "credit", "refund", "debit"}
Keys == Days \X Sources

\* wallet.rs: WALLET_FLOOR; rules.rs: DAILY_LOSS_CAP_COINS
Floor == 0
CapCoins == 100

Min(x, y) == IF x <= y THEN x ELSE y
Max(x, y) == IF x >= y THEN x ELSE y

\* rules.rs::daily_loss_cap and rules.rs::clip_debit
LossCap(w) == IF w <= 0 THEN 0 ELSE Min(CapCoins, (w * 3) \div 10)
ClipPaid(r, w, c) == IF r <= 0 THEN 0 ELSE Max(0, Min(Min(r, w), c))

Ports == {"Credit", "Refund", "CreditOnce", "Purchase", "DebitFloored", "DebitCapped"}
Ops ==
    [port : Ports, day : Days, amount : Amounts]
        \cup [port : {"SettleMint"}, day : Days, amount : 0..MaxAmount]
NoOp == [port |-> "none", day |-> 0, amount |-> 0]

SourceOf(o) ==
    CASE o.port = "SettleMint" -> "mint"
      [] o.port = "CreditOnce" -> "once"
      [] o.port = "Credit" -> "credit"
      [] o.port = "Refund" -> "refund"
      [] OTHER -> "debit"

VARIABLES ledger, lock, pc, op, snap, closed, fell

vars == <<ledger, lock, pc, op, snap, closed, fell>>

RECURSIVE SumSeq(_)
SumSeq(s) == IF s = <<>> THEN 0 ELSE Head(s) + SumSeq(Tail(s))

RECURSIVE NegSeq(_)
NegSeq(s) == IF s = <<>> THEN 0 ELSE (IF Head(s) < 0 THEN -Head(s) ELSE 0) + NegSeq(Tail(s))

RECURSIVE SumKeys(_, _)
SumKeys(l, S) ==
    IF S = {} THEN 0 ELSE LET k == CHOOSE x \in S : TRUE IN SumSeq(l[k]) + SumKeys(l, S \ {k})

RECURSIVE NegKeys(_, _)
NegKeys(l, S) ==
    IF S = {} THEN 0 ELSE LET k == CHOOSE x \in S : TRUE IN NegSeq(l[k]) + NegKeys(l, S \ {k})

\* wallet.rs::summed, summed_before and debited_on_day
Balance == SumKeys(ledger, Keys)
SumBefore(d) == SumKeys(ledger, {k \in Keys : k[1] < d})
DebitedOn(d) == NegKeys(ledger, {k \in Keys : k[1] = d})

\* a day's mint movement, 0 while none is written
MintOf(l, d) == IF l[<<d, "mint">>] = <<>> THEN 0 ELSE Head(l[<<d, "mint">>])

TypeOK ==
    /\ DOMAIN ledger = Keys
    /\ \A k \in Keys :
          /\ Len(ledger[k]) \in 0..NActors
          /\ \A i \in 1..Len(ledger[k]) : ledger[k][i] \in -MaxAmount..MaxAmount
    /\ lock \in Actors \cup {0}
    /\ pc \in [Actors -> {"idle", "read", "held", "wrote", "done"}]
    /\ op \in [Actors -> Ops \cup {NoOp}]
    /\ snap \in [Actors -> Int]
    /\ closed \in [Days -> BOOLEAN]
    /\ fell \in BOOLEAN

Init ==
    /\ ledger = [k \in Keys |-> <<>>]
    /\ lock = 0
    /\ pc = [a \in Actors |-> "idle"]
    /\ op = [a \in Actors |-> NoOp]
    /\ snap = [a \in Actors |-> 0]
    /\ closed = [d \in Days |-> FALSE]
    /\ fell = FALSE

\* wallet.rs::insert: INSERT ... ON CONFLICT (study_day, source, reference) DO NOTHING. With the
\* unique index the conflict writes nothing; without it, every insert lands.
Insert(k, delta) ==
    ledger' = IF UniqueKey /\ ledger[k] # <<>>
              THEN ledger
              ELSE [ledger EXCEPT ![k] = Append(@, delta)]

\* the balance a port that reads it sees: inside the write, or the defect's snapshot
BalanceSeen(a) == IF ReadInsideWrite THEN Balance ELSE snap[a]

BalanceReader(o) == o.port \in {"Purchase", "DebitFloored", "DebitCapped"}

\* the defect only: the balance read in a step of its own, before the lock is taken
Snap(a) ==
    /\ ~ReadInsideWrite
    /\ pc[a] = "idle"
    /\ \E o \in Ops :
          /\ BalanceReader(o)
          /\ op' = [op EXCEPT ![a] = o]
    /\ snap' = [snap EXCEPT ![a] = Balance]
    /\ pc' = [pc EXCEPT ![a] = "read"]
    /\ UNCHANGED <<ledger, lock, closed>>

\* db.rs::write: BEGIN IMMEDIATE takes the write lock
Begin(a) ==
    /\ lock = 0
    /\ \/ /\ pc[a] = "idle"
          /\ \E o \in Ops :
                /\ (ReadInsideWrite \/ ~BalanceReader(o))
                /\ op' = [op EXCEPT ![a] = o]
       \/ /\ pc[a] = "read"
          /\ UNCHANGED op
    /\ lock' = a
    /\ pc' = [pc EXCEPT ![a] = "held"]
    /\ UNCHANGED <<ledger, snap, closed>>

\* wallet.rs::credit_on (and refund_on, which is credit_on): the insert alone decides
CreditWrite(o, k) == Insert(k, o.amount)

\* wallet.rs::credit_once_on: any movement of the source and reference, on ANY study day, answers
\* AlreadyCredited; the defect reads only the request's own day
CreditOnceWrite(o, k) ==
    LET ever == IF OnceGuardAllDays
                THEN \E d \in Days : ledger[<<d, "once">>] # <<>>
                ELSE ledger[k] # <<>>
    IN IF ever THEN UNCHANGED ledger ELSE Insert(k, o.amount)

\* wallet.rs::settle_mint_on: a closed day's mint, or a raise, settles at held.max(amount); an open
\* day's lower settle keeps what the wallet cannot give back. The defect's closed arm takes amount.
SettleMintWrite(o, k) ==
    IF ledger[k] = <<>>
    THEN IF o.amount > 0 THEN Insert(k, o.amount) ELSE UNCHANGED ledger
    ELSE LET held == Head(ledger[k])
             room == Max(Balance - Floor, 0)
             settled == IF closed[o.day] \/ o.amount >= held
                        THEN IF ClosedKeepsHeld THEN Max(held, o.amount) ELSE o.amount
                        ELSE Max(o.amount, held - room)
         IN ledger' = IF settled # held
                      THEN [ledger EXCEPT ![k] = [i \in 1..Len(@) |-> settled]]
                      ELSE ledger

\* wallet.rs::purchase_on: a held key answers AlreadyBought; a price the balance cannot pay above
\* the floor is refused
PurchaseWrite(a, o, k) ==
    IF ledger[k] # <<>> \/ BalanceSeen(a) - o.amount < Floor
    THEN UNCHANGED ledger
    ELSE Insert(k, -o.amount)

\* wallet.rs::debit_floored_on: pays what the balance holds above the floor, forgives the rest
DebitFlooredWrite(a, o, k) ==
    IF ledger[k] # <<>>
    THEN UNCHANGED ledger
    ELSE Insert(k, -Min(o.amount, Max(BalanceSeen(a) - Floor, 0)))

\* wallet.rs::debit_capped_on: clipped to the balance above the floor and to the day's remaining
\* loss cap, from the wallet at the day's start
DebitCappedWrite(a, o, k) ==
    IF ledger[k] # <<>>
    THEN UNCHANGED ledger
    ELSE LET remainder == LossCap(SumBefore(o.day)) - DebitedOn(o.day)
         IN Insert(k, -ClipPaid(o.amount, BalanceSeen(a) - Floor, remainder))

ReadWrite(a) ==
    /\ pc[a] = "held"
    /\ lock = a
    /\ LET o == op[a]
           k == <<o.day, SourceOf(o)>>
       IN CASE o.port \in {"Credit", "Refund"} -> CreditWrite(o, k)
            [] o.port = "CreditOnce" -> CreditOnceWrite(o, k)
            [] o.port = "SettleMint" -> SettleMintWrite(o, k)
            [] o.port = "Purchase" -> PurchaseWrite(a, o, k)
            [] o.port = "DebitFloored" -> DebitFlooredWrite(a, o, k)
            [] OTHER -> DebitCappedWrite(a, o, k)
    /\ pc' = [pc EXCEPT ![a] = "wrote"]
    /\ UNCHANGED <<lock, op, snap, closed>>

Commit(a) ==
    /\ pc[a] = "wrote"
    /\ lock' = 0
    /\ pc' = [pc EXCEPT ![a] = "done"]
    /\ op' = [op EXCEPT ![a] = NoOp]
    /\ snap' = [snap EXCEPT ![a] = 0]
    /\ UNCHANGED <<ledger, closed>>

\* the fold closes a study day; a closed day stays closed
Close(d) ==
    /\ ~closed[d]
    /\ closed' = [closed EXCEPT ![d] = TRUE]
    /\ UNCHANGED <<ledger, lock, pc, op, snap>>

Step ==
    \/ \E a \in Actors : Snap(a) \/ Begin(a) \/ ReadWrite(a) \/ Commit(a)
    \/ \E d \in Days : Close(d)

\* history: did any step lower a closed day's mint movement
Fell == fell \/ \E d \in Days : closed[d] /\ MintOf(ledger', d) < MintOf(ledger, d)

Finished == (\A a \in Actors : pc[a] = "done") /\ UNCHANGED vars

Next ==
    \/ Step /\ fell' = Fell
    \/ Finished

Spec == Init /\ [][Next]_vars

\* R2, wallet.rs module doc: "no port reads outside the write lock"; the summed balance never
\* falls below the floor
FloorHolds == Balance >= Floor

\* wallet.rs module doc: "one movement per (study day, source, reference), is the migration's
\* unique index, and every insert does nothing on that conflict"
OneMovementPerKey == \A k \in Keys : Len(ledger[k]) <= 1

\* wallet.rs::credit_once_on: a once-ever credit's source and reference hold at most one movement
\* over every study day
CreditOnceEver ==
    \A d1, d2 \in Days :
        \A i \in 1..Len(ledger[<<d1, "once">>]) :
            \A j \in 1..Len(ledger[<<d2, "once">>]) : d1 = d2 /\ i = j

\* wallet.rs::settle_mint_on: once a day is closed, its mint movement's delta never decreases
SettledMintNeverFalls == ~fell
=============================================================================
