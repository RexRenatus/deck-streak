------------------------------ MODULE TokenWindow ------------------------------
\* @phx covers crates/quests/src/tokens.rs anchor=activate_token_on digest=sha256:0218568f0e771a6a4f21bc2eb81abc648477d842ea3a01b8e608d1269ac4a0a4
\* @phx covers crates/quests/src/tokens.rs anchor=grant_token digest=sha256:bc0428e21e65fbe87078f0c51c3d6fdc58d12c1d2427ff96ff8d86b49844d11f
\* @phx covers crates/kernel/src/db.rs anchor=write digest=sha256:c3d700eda268a6f46c7eea0aabcd2f62d8fc0aeffbe03d1cd1e8437a88bdb916
\* @phx cites #102, #103
\* @phx property AtMostOneOpenWindow ramp=report
\* @phx property OldestHeldFirst ramp=report
\* @phx witness witness/an-activation-split-across-two-writes.cfg kills=AtMostOneOpenWindow
\* @phx witness witness/an-activation-with-no-open-window-check.cfg kills=AtMostOneOpenWindow
\* @phx witness witness/the-newest-held-token-first.cfg kills=OldestHeldFirst
(***************************************************************************)
\* One double-XP window at a time, the oldest token first (#102, #103): N callers activate tokens
\* through tokens.rs::activate_token_on while Epic choices store new ones through
\* tokens.rs::grant_token and the clock runs. The xp_tokens table is migration 008101's.
\*
\* An activation is ONE write of its caller, and the model takes it in steps:
\* - db.rs::write opens every write with BEGIN IMMEDIATE, which takes SQLite's write lock at once,
\*   so no other write runs until the holder ends; a call's first step takes the lock and its last
\*   gives it back;
\* - Check: the read of an open window at the caller's instant ("A token whose window is still
\*   open refuses the activation", tokens.rs::activate_token_on): a window is open while the
\*   instant is before its end; an open one ends the call (AlreadyActive);
\* - Pick: the read of the oldest held token, by id; none held ends the call (NoneHeld);
\* - Write: the activation, guarded on the token still being held, opens its window from the
\*   caller's instant for Window ticks; a token another write activated first changes nothing.
\*
\* What is abstracted:
\* - a token's id is its place in the order grant_token stored it: an id is never reused;
\* - the window is Window ticks of the model's clock, two hours in the code; one clock is read
\*   by every caller, at its call's first step (the code's `now` is the caller's argument);
\* - consumption (tokens.rs::settle_token_bonuses_on) marks only a token whose window ended
\*   before the instant, which neither read counts, so it is left out;
\* - a rollback keeps nothing of its call and is the call never made, so it is left out.
\*
\* Three defect switches, each FALSE in a witness: CheckOpenWindow (the read of an open window),
\* OldestFirst (the held token picked is the oldest, not the newest) and OneWrite (a call's steps
\* run in one write of its caller).
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS CheckOpenWindow, OldestFirst, OneWrite, NActors, NTokens, Window, MaxTime

Actors == 1..NActors
Tokens == 1..NTokens
Instants == 1..MaxTime
NoActor == 0
Never == 0

VARIABLES
    granted,  \* the tokens stored: ids 1..granted
    start,    \* each token's activation instant, Never while it is held
    ends,     \* each token's window end, Never while it is held
    now,      \* the clock
    holder,   \* the caller holding the write lock, or NoActor
    pc,       \* each caller's place: "idle", "pick" or "write"
    at,       \* each caller's instant, read at its call's first step
    pick      \* each caller's picked token

vars == <<granted, start, ends, now, holder, pc, at, pick>>

TypeOK ==
    /\ granted \in 0..NTokens
    /\ start \in [Tokens -> {Never} \cup Instants]
    /\ ends \in [Tokens -> {Never} \cup 1..(MaxTime + Window)]
    /\ now \in Instants
    /\ holder \in Actors \cup {NoActor}
    /\ pc \in [Actors -> {"idle", "pick", "write"}]
    /\ at \in [Actors -> Instants]
    /\ pick \in [Actors -> Tokens]

Init ==
    /\ granted = 0
    /\ start = [t \in Tokens |-> Never]
    /\ ends = [t \in Tokens |-> Never]
    /\ now = 1
    /\ holder = NoActor
    /\ pc = [a \in Actors |-> "idle"]
    /\ at = [a \in Actors |-> 1]
    /\ pick = [a \in Actors |-> 1]

\* A call's first step may start: under one write, no other caller holds the lock.
Free == ~OneWrite \/ holder = NoActor

\* A later step of caller a: under one write, a holds the lock.
Mine(a) == ~OneWrite \/ holder = a

\* A first step that continues the call: under one write, a takes the lock.
Take(a) == holder' = IF OneWrite THEN a ELSE NoActor

\* The stored tokens.
Stored == 1..granted

\* A token's window is open at instant i: it was activated and i is before its end.
OpenAt(t, i) == start[t] /= Never /\ i < ends[t]

\* The clock runs, whoever holds the lock.
Tick ==
    /\ now < MaxTime
    /\ now' = now + 1
    /\ UNCHANGED <<granted, start, ends, holder, pc, at, pick>>

\* tokens.rs::grant_token: an Epic's choice stores the next token, in a write of its own caller.
Grant ==
    /\ granted < NTokens
    /\ Free
    /\ granted' = granted + 1
    /\ UNCHANGED <<start, ends, now, holder, pc, at, pick>>

\* tokens.rs::activate_token_on, first step: the read of an open window at the caller's instant.
Check(a) ==
    /\ pc[a] = "idle"
    /\ Free
    /\ IF CheckOpenWindow /\ \E t \in Stored : OpenAt(t, now)
       THEN /\ holder' = NoActor
            /\ UNCHANGED <<pc, at>>
       ELSE /\ pc' = [pc EXCEPT ![a] = "pick"]
            /\ at' = [at EXCEPT ![a] = now]
            /\ Take(a)
    /\ UNCHANGED <<granted, start, ends, now, pick>>

\* The read of the oldest held token, by id (or with the defect, the newest).
Pick(a) ==
    /\ pc[a] = "pick"
    /\ Mine(a)
    /\ LET held == {t \in Stored : start[t] = Never}
       IN IF held = {}
          THEN /\ pc' = [pc EXCEPT ![a] = "idle"]
               /\ holder' = NoActor
               /\ UNCHANGED pick
          ELSE /\ pick' = [pick EXCEPT ![a] =
                              CHOOSE t \in held :
                                  \A u \in held : IF OldestFirst THEN t <= u ELSE t >= u]
               /\ pc' = [pc EXCEPT ![a] = "write"]
               /\ UNCHANGED holder
    /\ UNCHANGED <<granted, start, ends, now, at>>

\* The activation, guarded on the picked token still being held: its window opens from the
\* caller's instant.
Activate(a) ==
    /\ pc[a] = "write"
    /\ Mine(a)
    /\ LET t == pick[a]
       IN IF start[t] = Never
          THEN /\ start' = [start EXCEPT ![t] = at[a]]
               /\ ends' = [ends EXCEPT ![t] = at[a] + Window]
          ELSE UNCHANGED <<start, ends>>
    /\ pc' = [pc EXCEPT ![a] = "idle"]
    /\ holder' = NoActor
    /\ UNCHANGED <<granted, now, at, pick>>

Next ==
    \/ Tick
    \/ Grant
    \/ \E a \in Actors : Check(a) \/ Pick(a) \/ Activate(a)

Spec == Init /\ [][Next]_vars

\* At most one token's window is open at any instant: "a token whose window is still open
\* refuses the activation" (tokens.rs::activate_token_on), and the read and the write it guards
\* are one write of the caller (the tokens.rs module doc).
AtMostOneOpenWindow == Cardinality({t \in Stored : OpenAt(t, now)}) <= 1

\* The tokens are activated oldest first: a token is never activated while an older one is held
\* ("the oldest token never activated has its window opened", tokens.rs::activate_token_on).
OldestHeldFirst ==
    \A t, u \in Stored : (t < u /\ start[u] /= Never) => start[t] /= Never

\* Reachability, beside the witnesses: a token is activated; a window ends and a later token is
\* activated after it; a call finds a window open.
ATokenIsActivated == \E t \in Stored : start[t] /= Never
ALaterWindowFollows == \E t, u \in Stored : t < u /\ start[u] /= Never /\ ends[t] <= start[u]
AWindowIsOpen == \E t \in Stored : OpenAt(t, now)

=============================================================================
