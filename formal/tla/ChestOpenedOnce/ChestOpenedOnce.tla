---------------------------- MODULE ChestOpenedOnce ----------------------------
\* @phx covers crates/quests/src/chests.rs anchor=open_chest_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chests.rs anchor=settle_epic_choice_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chests.rs anchor=sweep_stale_chests_on digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chest_store.rs anchor=mark_opened digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chest_store.rs anchor=mark_resolved digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chest_store.rs anchor=settle_choice digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/quests/src/chest_store.rs anchor=sweep_resolve digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/kernel/src/db.rs anchor=write digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #102, #103
\* @phx property OneRewardPerChest ramp=report
\* @phx property AResolvedChestStaysResolved ramp=report
\* @phx witness witness/an-open-unguarded-on-the-chests-state.cfg kills=OneRewardPerChest
\* @phx witness witness/a-prize-granted-before-its-settle.cfg kills=OneRewardPerChest
\* @phx witness witness/an-open-split-across-two-writes.cfg kills=OneRewardPerChest
\* @phx witness witness/a-resolve-undone-by-an-unguarded-open.cfg kills=AResolvedChestStaysResolved
(***************************************************************************)
\* A chest is opened, chosen and swept once (#102, #103): N callers over one set of stored chests,
\* each call one of chests.rs::open_chest_on, chests.rs::settle_epic_choice_on and
\* chests.rs::sweep_stale_chests_on. The chests table is migration 008101's.
\*
\* A call runs in ONE write of its caller, and the model takes it in steps:
\* - db.rs::write opens every write with BEGIN IMMEDIATE, which takes SQLite's write lock at once,
\*   so no other write runs until the holder ends; a call's first step takes the lock and its last
\*   gives it back;
\* - an open: chest_store::mark_opened, guarded on a sealed or vaulted chest ("a second open
\*   changes nothing and pays nothing"); an open that took no row, or an Epic's open, ends there.
\*   Any other rarity then takes chest_store::mark_resolved and the open answers its payout, which
\*   the caller grants in its write: the payout is answered whatever the resolve took;
\* - a choice: chest_store::settle_choice, guarded on an opened Epic with no choice, and only when
\*   it took the row, the prize ("The settle is a guarded update, taken before anything is
\*   granted, so a choice settled before grants nothing", chests.rs::settle_epic_choice_on);
\* - a sweep: chest_store::unresolved_chests reads the chests to resolve, then each takes
\*   chest_store::sweep_resolve, guarded on an unresolved chest, and pays its payout or an
\*   untapped Epic's fallback only when it took the row ("a chest another write resolved first is
\*   skipped and paid nothing", chests.rs::sweep_stale_chests_on).
\*
\* What is abstracted:
\* - a reward is the chest's payout, its prize (a token or a freeze) or an untapped Epic's
\*   fallback: each is one credit through the caller. A token prize is also held once by the
\*   unique index xp_tokens_one_per_chest; a freeze has no such index, so the model counts both;
\* - vaulted behaves as sealed in every guard, and is left out;
\* - the sweep's day rules (a chest of today, a vaulted chest of the day before) only skip more,
\*   so its read keeps any non-empty subset of the unresolved chests, resolved in any order;
\* - the choice's read refuses a chest that is not an Epic; a rarity is fixed at its roll, so
\*   that read is taken with the choice's first write, and a non-Epic's choice changes nothing;
\* - the freeze cap only turns a freeze into a token: both are one prize;
\* - a rollback keeps nothing of its call and is the call never made, so it is left out.
\*
\* Three defect switches, each FALSE in a witness: GuardedOpen (mark_opened's state guard),
\* SettleFirst (the choice settles before its prize is granted; the predecessor granted first)
\* and OneWrite (a call's steps run in one write of its caller).
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS GuardedOpen, SettleFirst, OneWrite, NActors, NChests

Actors == 1..NActors
Chests == 1..NChests
NoActor == 0

VARIABLES
    state,        \* each chest's state: "sealed", "opened" or "resolved"
    epic,         \* each chest's rarity, fixed at its roll: TRUE for an Epic
    chosen,       \* each chest's choice made
    rewards,      \* each chest's credited rewards
    everResolved, \* history: each chest that was ever resolved
    holder,       \* the caller holding the write lock, or NoActor
    pc,           \* each caller's place: "idle", "resolve", "prize", "settle" or "sweep"
    target,       \* each caller's chest, for an open's or a choice's later step
    todo          \* each sweeping caller's chests still to resolve, from its read

vars == <<state, epic, chosen, rewards, everResolved, holder, pc, target, todo>>

TypeOK ==
    /\ state \in [Chests -> {"sealed", "opened", "resolved"}]
    /\ epic \in [Chests -> BOOLEAN]
    /\ chosen \in [Chests -> BOOLEAN]
    /\ rewards \in [Chests -> Nat]
    /\ everResolved \in [Chests -> BOOLEAN]
    /\ holder \in Actors \cup {NoActor}
    /\ pc \in [Actors -> {"idle", "resolve", "prize", "settle", "sweep"}]
    /\ target \in [Actors -> Chests]
    /\ todo \in [Actors -> SUBSET Chests]

Init ==
    /\ state = [c \in Chests |-> "sealed"]
    /\ epic \in [Chests -> BOOLEAN]
    /\ chosen = [c \in Chests |-> FALSE]
    /\ rewards = [c \in Chests |-> 0]
    /\ everResolved = [c \in Chests |-> FALSE]
    /\ holder = NoActor
    /\ pc = [a \in Actors |-> "idle"]
    /\ target = [a \in Actors |-> 1]
    /\ todo = [a \in Actors |-> {}]

\* A call's first step may start: under one write, no other caller holds the lock.
Free == ~OneWrite \/ holder = NoActor

\* A later step of caller a: under one write, a holds the lock.
Mine(a) == ~OneWrite \/ holder = a

\* A first step that continues the call: under one write, a takes the lock.
Take(a) == holder' = IF OneWrite THEN a ELSE NoActor

\* A step that ends the call gives the lock back.
Release == holder' = NoActor

\* Resolves chest c, and keeps its history.
Resolve(c) ==
    /\ state' = [state EXCEPT ![c] = "resolved"]
    /\ everResolved' = [everResolved EXCEPT ![c] = TRUE]

\* chests.rs::open_chest_on, first step: chest_store::mark_opened. Guarded, it takes a sealed
\* chest only. An open that took no row (AlreadyOpened) or an Epic's (ChoicePending) ends here.
OpenChest(a, c) ==
    /\ pc[a] = "idle"
    /\ Free
    /\ LET taken == ~GuardedOpen \/ state[c] = "sealed"
       IN /\ state' = IF taken THEN [state EXCEPT ![c] = "opened"] ELSE state
          /\ IF taken /\ ~epic[c]
             THEN /\ pc' = [pc EXCEPT ![a] = "resolve"]
                  /\ target' = [target EXCEPT ![a] = c]
                  /\ Take(a)
             ELSE /\ UNCHANGED <<pc, target>>
                  /\ Release
    /\ UNCHANGED <<epic, chosen, rewards, everResolved, todo>>

\* The open's second step: chest_store::mark_resolved, guarded on an opened chest, then the
\* payout the open answers for its caller to grant, whatever the resolve took.
ResolveOpened(a) ==
    /\ pc[a] = "resolve"
    /\ Mine(a)
    /\ LET c == target[a]
       IN /\ IF state[c] = "opened"
             THEN Resolve(c)
             ELSE UNCHANGED <<state, everResolved>>
          /\ rewards' = [rewards EXCEPT ![c] = @ + 1]
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ Release
    /\ UNCHANGED <<epic, chosen, target, todo>>

\* chest_store::settle_choice's guard: an opened Epic whose choice is not made.
SettleTakes(c) == state[c] = "opened" /\ ~chosen[c]

\* The settle: chest c is resolved with its choice.
Settle(c) ==
    /\ Resolve(c)
    /\ chosen' = [chosen EXCEPT ![c] = TRUE]

\* chests.rs::settle_epic_choice_on, first step. Settled first: the guarded settle, which ends the
\* call when it took no row (AlreadySettled). With the defect: the prize, before the settle.
Choose(a, c) ==
    /\ pc[a] = "idle"
    /\ Free
    /\ epic[c]
    /\ IF SettleFirst
       THEN IF SettleTakes(c)
            THEN /\ Settle(c)
                 /\ pc' = [pc EXCEPT ![a] = "prize"]
                 /\ target' = [target EXCEPT ![a] = c]
                 /\ Take(a)
                 /\ UNCHANGED rewards
            ELSE /\ UNCHANGED <<state, chosen, everResolved, rewards, pc, target>>
                 /\ Release
       ELSE /\ rewards' = [rewards EXCEPT ![c] = @ + 1]
            /\ pc' = [pc EXCEPT ![a] = "settle"]
            /\ target' = [target EXCEPT ![a] = c]
            /\ Take(a)
            /\ UNCHANGED <<state, chosen, everResolved>>
    /\ UNCHANGED <<epic, todo>>

\* The choice's second step, settled first: the prize (a token stored for the chest, or a freeze
\* for the caller to grant).
Prize(a) ==
    /\ pc[a] = "prize"
    /\ Mine(a)
    /\ rewards' = [rewards EXCEPT ![target[a]] = @ + 1]
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ Release
    /\ UNCHANGED <<state, epic, chosen, everResolved, target, todo>>

\* The choice's second step with the defect: the guarded settle, after the prize was granted.
LateSettle(a) ==
    /\ pc[a] = "settle"
    /\ Mine(a)
    /\ IF SettleTakes(target[a])
       THEN Settle(target[a])
       ELSE UNCHANGED <<state, chosen, everResolved>>
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ Release
    /\ UNCHANGED <<epic, rewards, target, todo>>

\* chests.rs::sweep_stale_chests_on, its read: chest_store::unresolved_chests, of which the day
\* rules keep any non-empty subset.
SweepRead(a) ==
    /\ pc[a] = "idle"
    /\ Free
    /\ \E s \in SUBSET {c \in Chests : state[c] /= "resolved"} :
          /\ s /= {}
          /\ todo' = [todo EXCEPT ![a] = s]
    /\ pc' = [pc EXCEPT ![a] = "sweep"]
    /\ Take(a)
    /\ UNCHANGED <<state, epic, chosen, rewards, everResolved, target>>

\* One chest of the sweep's read: chest_store::sweep_resolve, guarded on an unresolved chest, and
\* only when it took the row, the payout or the fallback for the caller to grant.
SweepOne(a) ==
    /\ pc[a] = "sweep"
    /\ Mine(a)
    /\ \E c \in todo[a] :
          /\ IF state[c] /= "resolved"
             THEN /\ Resolve(c)
                  /\ rewards' = [rewards EXCEPT ![c] = @ + 1]
             ELSE UNCHANGED <<state, everResolved, rewards>>
          /\ todo' = [todo EXCEPT ![a] = @ \ {c}]
          /\ IF todo[a] = {c}
             THEN /\ pc' = [pc EXCEPT ![a] = "idle"]
                  /\ Release
             ELSE UNCHANGED <<pc, holder>>
    /\ UNCHANGED <<epic, chosen, target>>

Next ==
    \E a \in Actors :
        \/ \E c \in Chests : OpenChest(a, c)
        \/ ResolveOpened(a)
        \/ \E c \in Chests : Choose(a, c)
        \/ Prize(a)
        \/ LateSettle(a)
        \/ SweepRead(a)
        \/ SweepOne(a)

Spec == Init /\ [][Next]_vars

\* Each chest's reward is credited at most once, whatever the opens, choices and sweeps over it
\* and their order: its payout (open_chest_on, sweep_stale_chests_on), its prize
\* (settle_epic_choice_on) or an untapped Epic's fallback (sweep_stale_chests_on).
OneRewardPerChest == \A c \in Chests : rewards[c] <= 1

\* A resolved chest stays resolved: no open, choice or sweep moves it back (chest_store's guards
\* on mark_opened, settle_choice and sweep_resolve).
AResolvedChestStaysResolved == \A c \in Chests : everResolved[c] => state[c] = "resolved"

\* Reachability, beside the witnesses: an Epic's choice is settled; an untapped Epic is swept; a
\* chest of another rarity is resolved.
AnEpicIsChosen == \E c \in Chests : epic[c] /\ chosen[c]
AnUntappedEpicIsSwept == \E c \in Chests : epic[c] /\ ~chosen[c] /\ state[c] = "resolved"
AnotherRarityIsResolved == \E c \in Chests : ~epic[c] /\ state[c] = "resolved"

=============================================================================
