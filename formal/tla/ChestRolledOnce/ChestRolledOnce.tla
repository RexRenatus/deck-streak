---------------------------- MODULE ChestRolledOnce ----------------------------
\* @phx covers crates/quests/src/chests.rs anchor=grant_session_chests_on digest=sha256:0735b3d15465f1307c12f3037afe7c31a7fa7cfcca67c1e7d0e1740f2b2a01ab
\* @phx covers crates/quests/src/chest_store.rs anchor=insert_chest digest=sha256:57e9fcc9f8651b8a7e8717b234ba2f4b0c4c2920ab0231d3a41d48687517a5f9
\* @phx covers crates/quests/src/chest_store.rs anchor=set_pity digest=sha256:5229f5dd55355ac3fb12b6d15e6326b0ddadea3fb28017c69274566eea866e0c
\* @phx covers crates/kernel/src/db.rs anchor=write digest=sha256:c3d700eda268a6f46c7eea0aabcd2f62d8fc0aeffbe03d1cd1e8437a88bdb916
\* @phx cites #102, #103
\* @phx property OneChestPerKey ramp=report
\* @phx property PityEqualsChests ramp=report
\* @phx property FailedDrawWritesNothing ramp=report
\* @phx witness witness/a-chest-table-with-no-unique-key.cfg kills=OneChestPerKey
\* @phx witness witness/a-pity-written-apart-from-its-chest.cfg kills=PityEqualsChests
\* @phx witness witness/a-chest-written-before-its-draw.cfg kills=FailedDrawWritesNothing
(***************************************************************************)
\* A session's chest is rolled once (#102, #103): N recomputes, each granting the session chests
\* of one study day through chests.rs::grant_session_chests_on, over one set of chest keys. The
\* chests table and its unique key are migration 008101's; the pity counters are its pity row.
\*
\* A recompute is ONE write of its caller, and the model takes it in steps:
\* - Begin: the recompute takes the write lock. db.rs::write opens every write with BEGIN
\*   IMMEDIATE, which takes SQLite's write lock at once, so no other write runs until the holder
\*   ends; the grant then reads the day's chests (chest_store::chests_of_day) inside that write;
\* - Grant: one session of the request, in order. A session whose key the day already held is
\*   skipped with no draw. Otherwise the grant draws; a failed draw returns before anything is
\*   written for that session ("Both draws are taken before anything is written, so a failed one
\*   leaves nothing", chests.rs::grant_session_chests_on). A good draw inserts the chest, and the
\*   insert writes nothing when the key is held (chest_store::insert_chest, ON CONFLICT DO
\*   NOTHING); the pity moves only when the insert took a row (chest_store::set_pity);
\* - Commit or Rollback: the caller ends its write. After a failed draw the chests granted before
\*   it stay in the caller's write, and whether that write commits is the caller's decision, so
\*   both ends are open. Rollback is also any end that keeps nothing, a process that dies holding
\*   the write included: SQLite keeps no uncommitted write.
\*
\* What is abstracted:
\* - a key stands for the code's (study day, origin, session start); the model holds one day's
\*   session chests. The code's drift guard skips a session within one session gap of a held
\*   session chest; the model skips only the exact key, which skips less, so it has every
\*   behaviour the code has;
\* - the request is any sequence of keys, repeats included: the port takes any slice of
\*   sessions, and the day's read does not see the chests its own pass inserts;
\* - the day's cap (per_day_max) only stops a pass early, and is left out;
\* - the two draws, rarity then payout, are one draw that can fail: either failing is the port's
\*   Err;
\* - the pity row's two counters are one count of the chests that moved them: pity.rs::Pity::after
\*   advances or resets each counter once per chest, so a chest without its move, or a move
\*   without its chest, puts the counters out of step with the stored chests;
\* - a store error is left out: the port returns it with the caller's write open, and the model's
\*   caller commits only after a whole pass or a draw's Err. A caller that kept its write after a
\*   store error between the insert and the pity could keep a chest without its move, so a caller
\*   rolls back on one.
\*
\* Three defect switches, each FALSE in a witness: UniqueKey (the migration's unique key),
\* PityInTheChestsWrite (set_pity runs in the caller's write, beside the insert) and DrawFirst (the
\* draw is taken before the insert).
(***************************************************************************)
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS UniqueKey, PityInTheChestsWrite, DrawFirst, NActors, NKeys, ReqLen

Actors == 1..NActors
Keys == 1..NKeys
NoActor == 0

VARIABLES
    chests,       \* the committed chests: each key's stored rows, in insert order; TRUE is a rolled row
    pity,         \* the committed pity count
    holder,       \* the recompute holding the write lock, or NoActor
    pc,           \* each recompute's place: "idle", "run", "err" (its draw failed), "end"
    req,          \* the holder's request: the keys of its sessions, in order
    idx,          \* the holder's next session in req
    seen,         \* the keys the holder's read of the day found held
    txChests,     \* the holder's uncommitted chests, as chests is
    txPity,       \* the holder's uncommitted pity moves
    failedFresh   \* history: each key whose draw failed while the holder's write held no chest for it

vars == <<chests, pity, holder, pc, req, idx, seen, txChests, txPity, failedFresh>>

TypeOK ==
    /\ chests \in [Keys -> Seq(BOOLEAN)]
    /\ pity \in Nat
    /\ holder \in Actors \cup {NoActor}
    /\ pc \in [Actors -> {"idle", "run", "err", "end"}]
    /\ req \in [1..ReqLen -> Keys]
    /\ idx \in 1..(ReqLen + 1)
    /\ seen \subseteq Keys
    /\ txChests \in [Keys -> Seq(BOOLEAN)]
    /\ txPity \in Nat
    /\ failedFresh \subseteq Keys

NoChests == [k \in Keys |-> <<>>]

Init ==
    /\ chests = NoChests
    /\ pity = 0
    /\ holder = NoActor
    /\ pc = [a \in Actors |-> "idle"]
    /\ req = [i \in 1..ReqLen |-> 1]
    /\ idx = 1
    /\ seen = {}
    /\ txChests = NoChests
    /\ txPity = 0
    /\ failedFresh = {}

\* What the holder's write sees of key k: the committed rows and its own.
HeldInWrite(k) == Len(chests[k]) + Len(txChests[k]) > 0

\* chest_store::insert_chest: under the unique key, a held key writes nothing.
Inserts(k) == ~(UniqueKey /\ HeldInWrite(k))

\* The chest row `rolled` and, when the insert took it, the pity move: in the caller's write,
\* or with the defect, the pity in a write of its own that commits at once.
WriteChest(k, rolled) ==
    IF Inserts(k)
    THEN /\ txChests' = [txChests EXCEPT ![k] = Append(@, rolled)]
         /\ IF PityInTheChestsWrite
            THEN /\ txPity' = txPity + 1
                 /\ pity' = pity
            ELSE /\ txPity' = txPity
                 /\ pity' = pity + 1
    ELSE UNCHANGED <<txChests, txPity, pity>>

\* The holder's locals, cleared when its write ends.
Idle ==
    /\ req' = [i \in 1..ReqLen |-> 1]
    /\ idx' = 1
    /\ seen' = {}

\* db.rs::write, then chests_of_day: the lock and the day's read, inside the write.
Begin(a) ==
    /\ holder = NoActor
    /\ pc[a] = "idle"
    /\ \E r \in [1..ReqLen -> Keys] : req' = r
    /\ holder' = a
    /\ pc' = [pc EXCEPT ![a] = "run"]
    /\ idx' = 1
    /\ seen' = {k \in Keys : Len(chests[k]) > 0}
    /\ txChests' = NoChests
    /\ txPity' = 0
    /\ UNCHANGED <<chests, pity, failedFresh>>

\* A session whose key the day's read found held: skipped, with no draw.
Skip(a) ==
    /\ holder = a
    /\ pc[a] = "run"
    /\ idx <= ReqLen
    /\ req[idx] \in seen
    /\ idx' = idx + 1
    /\ UNCHANGED <<chests, pity, holder, pc, req, seen, txChests, txPity, failedFresh>>

\* A good draw, then the insert and the pity (or, with the defect, the insert before the draw).
Roll(a) ==
    /\ holder = a
    /\ pc[a] = "run"
    /\ idx <= ReqLen
    /\ req[idx] \notin seen
    /\ WriteChest(req[idx], TRUE)
    /\ idx' = idx + 1
    /\ UNCHANGED <<chests, holder, pc, req, seen, failedFresh>>

\* A failed draw: the port returns its Err. Taken first, it writes nothing; with the defect the
\* insert ran before it, and its row holds no roll.
Fail(a) ==
    /\ holder = a
    /\ pc[a] = "run"
    /\ idx <= ReqLen
    /\ req[idx] \notin seen
    /\ IF DrawFirst
       THEN UNCHANGED <<txChests, txPity, pity>>
       ELSE WriteChest(req[idx], FALSE)
    /\ failedFresh' = IF HeldInWrite(req[idx])
                      THEN failedFresh
                      ELSE failedFresh \cup {req[idx]}
    /\ pc' = [pc EXCEPT ![a] = "err"]
    /\ UNCHANGED <<chests, holder, req, idx, seen>>

\* Every session of the request was granted or skipped: the pass ends.
EndPass(a) ==
    /\ holder = a
    /\ pc[a] = "run"
    /\ idx > ReqLen
    /\ pc' = [pc EXCEPT ![a] = "end"]
    /\ UNCHANGED <<chests, pity, holder, req, idx, seen, txChests, txPity, failedFresh>>

\* The caller commits its write: after the whole pass, or after the Err, keeping what was granted.
Commit(a) ==
    /\ holder = a
    /\ pc[a] \in {"end", "err"}
    /\ chests' = [k \in Keys |-> chests[k] \o txChests[k]]
    /\ pity' = pity + txPity
    /\ holder' = NoActor
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ txChests' = NoChests
    /\ txPity' = 0
    /\ Idle
    /\ UNCHANGED failedFresh

\* The caller's write ends keeping nothing: a rollback, or a process that dies holding it.
Rollback(a) ==
    /\ holder = a
    /\ pc[a] \in {"run", "end", "err"}
    /\ holder' = NoActor
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ txChests' = NoChests
    /\ txPity' = 0
    /\ Idle
    /\ UNCHANGED <<chests, pity, failedFresh>>

Next ==
    \E a \in Actors :
        \/ Begin(a)
        \/ Skip(a)
        \/ Roll(a)
        \/ Fail(a)
        \/ EndPass(a)
        \/ Commit(a)
        \/ Rollback(a)

Spec == Init /\ [][Next]_vars

RECURSIVE RowsUpTo(_)
\* The number of stored chest rows over keys 1..n.
RowsUpTo(n) == IF n = 0 THEN 0 ELSE Len(chests[n]) + RowsUpTo(n - 1)

\* At most one chest per key, whatever the recomputes, their order and their failed draws: the
\* unique key is the existence check (chest_store::insert_chest, "a conflict writes nothing").
OneChestPerKey == \A k \in Keys : Len(chests[k]) <= 1

\* The pity moves with its chest and never without it: "a chest and the pity counters after it are
\* stored together or not at all" (chests.rs module doc).
PityEqualsChests == pity = RowsUpTo(NKeys)

\* A failed draw writes nothing: every stored chest holds a roll ("a failed one leaves nothing",
\* chests.rs::grant_session_chests_on).
FailedDrawWritesNothing == \A k \in Keys : \A i \in 1..Len(chests[k]) : chests[k][i]

\* Reachability, beside the witnesses: a chest is stored; a draw fails; a key whose draw failed
\* while unheld is rolled later.
AChestIsStored == \E k \in Keys : Len(chests[k]) = 1
ADrawFails == \E a \in Actors : pc[a] = "err"
AFailedKeyIsRolledLater == \E k \in failedFresh : Len(chests[k]) = 1

=============================================================================
