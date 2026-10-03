---------------------------- MODULE RelightOrder ----------------------------
\* @phx covers crates/coordination/src/relight.rs anchor=route_due_relights digest=sha256:c3ed617f7a2a14414ff87af6acc9acf08fb37cb1d9231b7bd75ed31702451aec
\* @phx covers crates/coordination/src/relight.rs anchor=announce_relight digest=sha256:9f527fa62497e4a68978e0da4fd845c774a1035cb92ec875744648252e794caf
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=sync_cycle digest=sha256:d32903aaea6eed5fa397ca99b5cfcea5ffbbe4540e1420a0503e8861c1b79add
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=run digest=sha256:7a383ba01872f4dde46ccb5220fa492de1f89498adf71774cae868e3e0b7acbb
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=runs_today_only_rules digest=sha256:f924a917b193bcfc4a5e0a9a39b45e5b8cb67f8be0b5cc1d6e765f0bc27f5e67
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=evaluate digest=sha256:bcafac597ac0e93e481c3c161efd5e28ed055657b74040ff7306c76066dc52f9
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=relight digest=sha256:19cb334d99817721cecf6432554bbd9cbcb501c80c5fdf2aa9d94dd425a16ed7
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=govern digest=sha256:37c4c03b2a7e959a0212e03b17a722b2cb56ae9f7e5286d4386ebcfcac157c70
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=pending digest=sha256:a40a08f18c1aec095c3787bf9422d0213fa264c9cab5fa1347105907aec9475c
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=routed digest=sha256:2b5a85a622b7f2b612570a5abc5c2a76663e2242839ca0eba636158800dd9455
\* @phx covers crates/coordination/src/recompute/streaks.rs anchor=RelightDue digest=sha256:0d175dbdba0ba57216894e424f90d61d679a279bf5b8415e6d06ea5867f7f549
\* @phx covers crates/streaks/src/store.rs anchor=put_relight_due digest=sha256:f264156702be79c3af3abb78dea24508426c19dded178fd9bce649259a6ef8db
\* @phx covers crates/streaks/src/store.rs anchor=relight_due digest=sha256:1dd411d6b5b949850a9d4f7846ffcbea51503faa0b41e05edcd7e0170df051f1
\* @phx covers crates/streaks/src/store.rs anchor=clear_relight_due digest=sha256:33d02cefd042e471b8466c700f70bb0b8f74ddba311a5edacc78ec2fd583b7ad
\* @phx covers crates/progression/src/ledger.rs anchor=grant_on digest=sha256:d8a78cf5539c65242930959fea3cb9873f22932169a7d3d10d838358b36aac7f
\* @phx covers crates/notifications/src/router.rs anchor=route digest=sha256:7bcf52fe22d886b3d71dfa0fa8e6dfb1d266a9a6b1662bada5482a2cd0c54dcb
\* @phx covers crates/notifications/src/router.rs anchor=decide digest=sha256:cce230bed3488bc390dc6b772974fdeba99379971eb059ab943a2aa5ddf83494
\* @phx covers crates/notifications/src/router.rs anchor=send_bot digest=sha256:de48b556abc2b5511a618398f0d54ecfa3354828708ed0eee2b4d9256e668f02
\* @phx covers crates/notifications/src/ledger.rs anchor=claim digest=sha256:0116ef4925de04614d09ac18952c0a0b0f7248fd65f5d4f6ca555448836a6ab7
\* @phx covers crates/notifications/src/ledger.rs anchor=release digest=sha256:e1c4b7e98c62f5632106dd14fc8ddfdf73e383f8d5b4b7b3f56f51f5dee861e5
\* @phx cites #446, #477
\* @phx property S1 ramp=report
\* @phx property S2 ramp=report
\* @phx property L1 ramp=report
\* @phx property ImportIsHistory ramp=report
\* @phx property NoOtherDayHeldBack ramp=report
\* @phx witness witness/router-with-no-once-ever-key.cfg kills=S1
\* @phx witness witness/due-list-keeps-a-day-answered-in-a-rolled-back-write.cfg kills=S2
\* @phx witness witness/outbox-row-written-before-the-grant-commits.cfg kills=S2
\* @phx witness witness/due-list-lost-at-a-crash.cfg kills=L1
\* @phx witness witness/outbox-row-cleared-when-taken.cfg kills=L1
\* @phx witness witness/a-failed-route-clears-the-day.cfg kills=L1
\* @phx witness witness/a-day-given-up-after-one-failed-route.cfg kills=L1
\* @phx witness witness/a-day-given-up-after-two-failed-routes.cfg kills=L1
\* @phx witness witness/a-day-given-up-after-three-failed-routes.cfg kills=L1
\* @phx witness witness/ledger-derived-due-list-celebrates-history.cfg kills=ImportIsHistory
\* @phx witness witness/a-failed-route-ends-the-take.cfg kills=NoOtherDayHeldBack
\* @phx witness witness/a-failed-route-returns-its-error.cfg kills=NoOtherDayHeldBack
(***************************************************************************)
(* The relight's XP grant and its celebration, across sync cycles.         *)
(* DeckStreak #446: SPEC-076 R18, R19 and R27. S1, S2 and L1 are the       *)
(* properties #477 names; the failed route (RouteFail, its arms, the day   *)
(* that keeps failing and NoOtherDayHeldBack) is #446's own.               *)
(*                                                                         *)
(* What is modelled, and the code each part abstracts:                     *)
(*  - The fold's per-day write (coordination recompute/mod.rs, Fold::run): *)
(*    one write per closed day it settles, oldest first, then one for the  *)
(*    current day. Each write commits, or rolls back at any step after the *)
(*    streak step (a later phase's error, or the commit's own). A fold     *)
(*    whose write rolls back returns an error and runs no later write.     *)
(*  - The relight (recompute/streaks.rs, StreaksStep::relight): with the   *)
(*    stored lapse open at the step's start and the day qualifying, the    *)
(*    grant is written on the fold's write (progression ledger.rs,         *)
(*    grant_on: INSERT ... ON CONFLICT DO NOTHING) and the day is answered *)
(*    as due.                                                              *)
(*  - The governor at a settle (StreaksStep::govern): a day that qualifies *)
(*    closes the lapse; any other day may open it, keep it or close it.    *)
(*  - The due record: where it lives (Store) and when a day enters it      *)
(*    (AnswerAt).                                                          *)
(*  - The cycle (sync_cycle.rs): a fold that returns an error routes       *)
(*    nothing; after a fold that commits, the cycle takes the due days and *)
(*    routes each.                                                         *)
(*  - The router (notifications router.rs, route): the celebration kind's  *)
(*    once-ever key, claimed at the first route; a later route of the same *)
(*    key is withheld.                                                     *)
(*  - A route that fails (relight.rs, route_due_relights, its Err arm):    *)
(*    announce_relight answers an error for one taken day. The loop logs   *)
(*    it and goes on to the next day. What the failure does to the day and *)
(*    to the rest of the take is RouteFailArm; the head's arm leaves the   *)
(*    day's row due and goes on. GiveUpAfter is a process that counts each *)
(*    day's failed routes in memory and drops the day at a fixed count;    *)
(*    the head keeps no such count (RelightDue holds no field).            *)
(*  - A day whose route keeps failing (KeepsFailing): its route fails at   *)
(*    every attempt and never succeeds, and its failures are not bounded.  *)
(*    NoOtherDayHeldBack states that it holds back no other day.           *)
(*  - A route loop that ends in an error (route_due_relights' `?`: the due *)
(*    read, RelightDue::pending, or a routed mark, RelightDue::routed), or *)
(*    a cycle that ends in an error after the fold's current commit and    *)
(*    before the route (Fold::run's revisit write; sync_cycle's level      *)
(*    read): sync_cycle logs it, and nothing more is routed this cycle.    *)
(*  - A crash or restart between any two steps: the open write rolls back  *)
(*    and every in-process value is lost; committed rows survive.          *)
(*  - The planned import (SPEC-140 R9), which writes history's relight     *)
(*    grants into the ledger and raises no occasion (SPEC-140 R5).         *)
(*                                                                         *)
(* Abstractions (each a choice, named):                                    *)
(*  - A day qualifies or not afresh at every evaluation: a later sync can  *)
(*    read fewer study reviews for the same day (an undo, a changed scope).*)
(*    A44's later sync, in which the day no longer qualifies, is one.      *)
(*  - The router's claim and its send are one step. A crash between them   *)
(*    is the router's own at-most-once choice (SPEC-041): a claimed key is *)
(*    never sent again. L1 is stated above the router: the relight's order *)
(*    must hand every committed grant to the router.                       *)
(*  - For the same reason, a failed route whose claim committed before the *)
(*    error (router.rs, send_bot, after route commits the claim) is the    *)
(*    router's decision; one whose first write rolled back, or that        *)
(*    announce_relight refused before routing, claims and sends nothing.   *)
(*  - The router's other withholds (quiet hours, celebrations off) are     *)
(*    decisions like a send, and what the router decides is SPEC-041's.    *)
(*  - Phases 1, 2 and 4 of a write are folded into its open and its        *)
(*    commit; a failure before the streak step leaves nothing to model.    *)
(*  - The fold's revisit write, after the current day's commit, runs no    *)
(*    relight and no governor (streaks.rs: the relight runs at a settle    *)
(*    and for the current day, the governor at a settle): when it commits  *)
(*    it changes no variable here, and when it fails it is RouteAbort at   *)
(*    the take.                                                            *)
(*  - A route failure and a route loop's error share one bound; the        *)
(*    failures of a day in KeepsFailing are outside it. A route loop's     *)
(*    error stays bounded because its steps (the due read, a routed mark)  *)
(*    fail with the database, never for one day: no day's own failure      *)
(*    reaches them.                                                        *)
(*  - The route loop takes the due days in any order. The code's order     *)
(*    (store.rs, relight_due: ORDER BY study_day) is one of them, and the  *)
(*    day that keeps failing in each witness is day 1, the first in it.    *)
(*  - A routed day's row is cleared in a step of its own (Clear), after    *)
(*    its route; the code clears it before the next day's route.           *)
(*  - One cycle runs at a time (the sync job's runner), with a router.     *)
(*  - The awards' offers (SPEC-073 part B; tla/AwardOnce models them):     *)
(*    sync_cycle hands Fold::run an AwardOffers over the cycle's router,   *)
(*    and Fold::run calls offer_owed before each settled day's write,      *)
(*    before the current day's write and after the revisit write. Each     *)
(*    offer routes a badge or record key in the router's own writes and    *)
(*    marks it in a write of its own; offer_owed logs every error and      *)
(*    never fails the fold. The base review count feeds the badges'        *)
(*    lifetime only. No variable here moves: a stutter, re-stamped.        *)
(*  - #311's re-read (ADR-313), re-read 2026-10-02: each owed day's        *)
(*    write in Fold::run now reads the settle cursor first, and a write    *)
(*    whose day is not the one owed is rolled back before any step runs.   *)
(*    With one cycle at a time the cursor it reads is the day before the   *)
(*    day in hand, or none on the first, so that arm never runs here: no   *)
(*    variable here moves, a stutter, re-stamped.                          *)
(*  - The landmarks' offers (SPEC-102 section 11, ADR-322), re-read        *)
(*    2026-10-03: sync_cycle reads the whole log's study days once and     *)
(*    hands Fold::run the awards' offers and then the landmarks' in turn.  *)
(*    The landmarks' offers route landmark keys and move their own cursor  *)
(*    in writes of their own, and touch no relight grant, due row or       *)
(*    route: no variable here moves, a stutter, re-stamped.                *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS
    Store,       \* "memory": an in-process list, taken by the cycle (#477's order)
                 \* "outbox": a row the grant's own write commits, cleared after the route
                 \* "slot":   one committed column, overwritten by each grant's write
                 \* "ledger": the committed grants the router has not yet decided
    AnswerAt,    \* "step": inside the write, before it commits (#477's order)
                 \* "commit": when the write commits, or after it
    ClearOnFail, \* TRUE: a fold that returns an error clears the in-process due list
    TakeClears,  \* TRUE: the take removes a day from the committed due record before its route
    OnceKey,     \* TRUE: the router keeps the celebration's once-ever key
    LastDay,     \* the study days are 1..LastDay
    MaxFail,     \* how many writes may roll back after the streak step
    MaxCrash,    \* how many crashes may fall between two steps
    ImportDays,  \* history's relight days that the import writes (before day 1)
    RouteFailArm, \* what a failed route does with its day and the rest of the take:
                  \* "stays":   the day leaves this cycle's take and its due row stays (the head)
                  \* "clears":  the day's due row is cleared as if it were routed
                  \* "ends":    the rest of this cycle's take is dropped with it, and the loop
                  \*            returns as if it had finished
                  \* "returns": the failure is the route loop's error: nothing more is routed
                  \*            this cycle, as at a route loop's error (RouteAbort)
    MaxRouteFail, \* how many routes may fail, and route loops end in an error
    KeepsFailing, \* the days whose route fails at every attempt, without a bound
    GiveUpAfter  \* 0: no count is kept (the head); k >= 1: a process that counts each day's
                 \* failed routes in memory drops the day's due row at its k-th failure

ASSUME Store \in {"memory", "outbox", "slot", "ledger"}
ASSUME AnswerAt \in {"step", "commit"}
ASSUME ClearOnFail \in BOOLEAN /\ TakeClears \in BOOLEAN /\ OnceKey \in BOOLEAN
ASSUME LastDay \in Nat /\ LastDay >= 1 /\ MaxFail \in Nat /\ MaxCrash \in Nat
ASSUME ImportDays \subseteq Nat /\ \A d \in ImportDays : d < 1
ASSUME RouteFailArm \in {"stays", "clears", "ends", "returns"} /\ MaxRouteFail \in Nat
ASSUME KeepsFailing \subseteq 1..LastDay /\ GiveUpAfter \in Nat

Days == 1..LastDay
AllDays == ImportDays \cup Days
Committed == {"outbox", "slot"}

VARIABLES
    today,     \* the clock's study day
    cursor,    \* the last settled day (committed with the settle's write)
    lapse,     \* the stored lapse is open (committed; governor_state.lapse_since)
    ledger,    \* the days whose relight grant is committed (xp_ledger, relight:<day>)
    imported,  \* the import has run
    outbox,    \* the committed due rows (Store "outbox" or "slot")
    due,       \* the in-process due list
    claimed,   \* the router's once-ever keys (notification_deliveries)
    sent,      \* celebrations sent per day, saturating at 2
    pc,        \* the cycle: "idle", "fold", "take", "route"
    ftoday,    \* the fold's today, fixed at its start
    w,         \* the open write: "none", "open", "stepped"
    wday,      \* the day the open write evaluates
    wkind,     \* how it evaluates it: "settle" or "current"
    wgrant,    \* the stepped write holds a relight grant
    wlapse,    \* the lapse the stepped write commits
    rq,        \* the days taken for this cycle's route
    cl,        \* routed days whose committed due row is still to clear
    fails,
    crashes,
    rfails,    \* routes that failed, and route loops that ended in an error
    rcount     \* each day's failed routes counted in process memory (GiveUpAfter only)

vars == <<today, cursor, lapse, ledger, imported, outbox, due, claimed, sent, pc, ftoday,
          w, wday, wkind, wgrant, wlapse, rq, cl, fails, crashes, rfails, rcount>>

TypeOK ==
    /\ today \in Days
    /\ cursor \in 0..LastDay
    /\ lapse \in BOOLEAN
    /\ ledger \subseteq AllDays
    /\ imported \in BOOLEAN
    /\ outbox \subseteq Days
    /\ due \subseteq Days
    /\ claimed \subseteq AllDays
    /\ sent \in [AllDays -> 0..2]
    /\ pc \in {"idle", "fold", "take", "route"}
    /\ ftoday \in Days
    /\ w \in {"none", "open", "stepped"}
    /\ wday \in Days
    /\ wkind \in {"settle", "current"}
    /\ wgrant \in BOOLEAN
    /\ wlapse \in BOOLEAN
    /\ rq \subseteq AllDays
    /\ cl \subseteq Days
    /\ fails \in 0..MaxFail
    /\ crashes \in 0..MaxCrash
    /\ rfails \in 0..MaxRouteFail
    /\ rcount \in [AllDays -> 0..GiveUpAfter]

Init ==
    /\ today = 1
    /\ cursor = 0
    /\ lapse = TRUE
    /\ ledger = {}
    /\ imported = FALSE
    /\ outbox = {}
    /\ due = {}
    /\ claimed = {}
    /\ sent = [d \in AllDays |-> 0]
    /\ pc = "idle"
    /\ ftoday = 1
    /\ w = "none"
    /\ wday = 1
    /\ wkind = "current"
    /\ wgrant = FALSE
    /\ wlapse = FALSE
    /\ rq = {}
    /\ cl = {}
    /\ fails = 0
    /\ crashes = 0
    /\ rfails = 0
    /\ rcount = [d \in AllDays |-> 0]

\* The cycle at rest: no fold, no open write, nothing taken. The write's fields return to one value
\* so that a state at rest is one state.
AtRest ==
    /\ pc' = "idle"
    /\ ftoday' = 1
    /\ w' = "none"
    /\ wday' = 1
    /\ wkind' = "current"
    /\ wgrant' = FALSE
    /\ wlapse' = FALSE
    /\ rq' = {}
    /\ cl' = {}

-----------------------------------------------------------------------------
\* The environment: the clock, the import, a write that rolls back, a crash, a route that fails
\* and a route loop that ends in an error.

Advance ==
    /\ today < LastDay
    /\ today' = today + 1
    /\ UNCHANGED <<cursor, lapse, ledger, imported, outbox, due, claimed, sent, pc, ftoday,
                   w, wday, wkind, wgrant, wlapse, rq, cl, fails, crashes, rfails, rcount>>

\* SPEC-140 R9: history's relight grants enter the ledger; R5: no occasion is raised.
Import ==
    /\ ~imported
    /\ imported' = TRUE
    /\ ledger' = ledger \cup ImportDays
    /\ UNCHANGED <<today, cursor, lapse, outbox, due, claimed, sent, pc, ftoday,
                   w, wday, wkind, wgrant, wlapse, rq, cl, fails, crashes, rfails, rcount>>

\* A write rolls back after its streak step: the fold returns an error, and the cycle routes nothing.
Fail ==
    /\ pc = "fold"
    /\ w = "stepped"
    /\ fails < MaxFail
    /\ fails' = fails + 1
    /\ due' = IF ClearOnFail THEN {} ELSE due
    /\ AtRest
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, claimed, sent, crashes,
                   rfails, rcount>>

\* A crash or restart: the open write rolls back and every in-process value is lost.
Crash ==
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ due' = {}
    /\ rcount' = [d \in AllDays |-> 0]
    /\ AtRest
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, claimed, sent, fails, rfails>>

\* A process that counts each day's failed routes in memory drops the day's due row at its k-th
\* failure (GiveUpAfter = k); the head keeps no count (GiveUpAfter = 0).
GivesUp(d) == GiveUpAfter > 0 /\ rcount[d] + 1 >= GiveUpAfter

\* A route that fails (relight.rs, route_due_relights' Err arm): announce_relight answers an error
\* for the taken day d. `c` is whether the router's claim committed before the error (send_bot's
\* half of route); otherwise the first write rolled back, or announce_relight refused before
\* routing, and nothing is claimed or sent. The head's arm ("stays") logs it and goes on to the
\* next taken day, so the day leaves this cycle's take and keeps its committed due row, and a
\* later cycle's take routes it again. A day in KeepsFailing fails without a bound.
RouteFail(d) ==
    /\ pc = "route"
    /\ d \in rq
    /\ rfails < MaxRouteFail \/ d \in KeepsFailing
    /\ rfails' = IF d \in KeepsFailing THEN rfails ELSE rfails + 1
    /\ rcount' = IF GiveUpAfter > 0
                    THEN [rcount EXCEPT ![d] = IF @ < GiveUpAfter THEN @ + 1 ELSE @]
                    ELSE rcount
    /\ \E c \in BOOLEAN :
         IF c /\ ~(OnceKey /\ d \in claimed)
           THEN /\ claimed' = claimed \cup {d}
                /\ sent' = [sent EXCEPT ![d] = IF @ < 2 THEN @ + 1 ELSE 2]
           ELSE UNCHANGED <<claimed, sent>>
    /\ IF RouteFailArm = "returns"
         THEN AtRest
         ELSE /\ rq' = IF RouteFailArm = "ends" THEN {} ELSE rq \ {d}
              /\ cl' = IF (RouteFailArm = "clears" \/ GivesUp(d))
                          /\ Store \in Committed /\ ~TakeClears
                          THEN cl \cup {d} ELSE cl
              /\ UNCHANGED <<pc, ftoday, w, wday, wkind, wgrant, wlapse>>
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, fails, crashes>>

\* The cycle ends in an error after the fold's current commit: at the take (the revisit write,
\* the level read, the due read) or during the route (a routed mark's write). sync_cycle logs it;
\* nothing more is routed this cycle, and no committed row changes.
RouteAbort ==
    /\ pc \in {"take", "route"}
    /\ rfails < MaxRouteFail
    /\ rfails' = rfails + 1
    /\ AtRest
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, claimed, sent,
                   fails, crashes, rcount>>

-----------------------------------------------------------------------------
\* The cycle.

StartFold ==
    /\ pc = "idle"
    /\ pc' = "fold"
    /\ ftoday' = today
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, claimed, sent,
                   w, wday, wkind, wgrant, wlapse, rq, cl, fails, crashes, rfails, rcount>>

\* The fold settles each closed day after the cursor, oldest first, then evaluates its today.
Open ==
    /\ pc = "fold"
    /\ w = "none"
    /\ w' = "open"
    /\ wday' = IF cursor + 1 < ftoday THEN cursor + 1 ELSE ftoday
    /\ wkind' = IF cursor + 1 < ftoday THEN "settle" ELSE "current"
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, claimed, sent, pc,
                   ftoday, wgrant, wlapse, rq, cl, fails, crashes, rfails, rcount>>

\* Phase 3 on the open write. The relight runs at a settle and at the current day; the governor
\* runs at a settle only. `lapse` is the stored value the step reads at its start.
Step ==
    /\ pc = "fold"
    /\ w = "open"
    /\ w' = "stepped"
    /\ \E q \in BOOLEAN :
         LET relit == lapse /\ q
         IN  /\ wgrant' = relit
             /\ wlapse' \in IF wkind = "settle"
                              THEN (IF q THEN {FALSE} ELSE BOOLEAN)
                              ELSE {lapse}
             \* A day answered inside the write, before it commits.
             /\ due' = IF AnswerAt = "step" /\ Store \in {"memory", "ledger"} /\ relit
                          THEN due \cup {wday} ELSE due
             /\ outbox' = IF AnswerAt = "step" /\ Store = "outbox" /\ relit
                             THEN outbox \cup {wday}
                             ELSE IF AnswerAt = "step" /\ Store = "slot" /\ relit
                                     THEN {wday} ELSE outbox
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, claimed, sent, pc, ftoday,
                   wday, wkind, rq, cl, fails, crashes, rfails, rcount>>

\* The write commits: the grant, the lapse, the cursor and (answered at commit) the due record
\* together. The current day's commit ends the fold.
Commit ==
    /\ pc = "fold"
    /\ w = "stepped"
    /\ ledger' = IF wgrant THEN ledger \cup {wday} ELSE ledger
    /\ lapse' = wlapse
    /\ cursor' = IF wkind = "settle" THEN wday ELSE cursor
    /\ outbox' = IF AnswerAt = "commit" /\ Store = "outbox" /\ wgrant
                    THEN outbox \cup {wday}
                    ELSE IF AnswerAt = "commit" /\ Store = "slot" /\ wgrant
                            THEN {wday} ELSE outbox
    /\ due' = IF AnswerAt = "commit" /\ Store = "memory" /\ wgrant THEN due \cup {wday} ELSE due
    /\ w' = "none"
    /\ wgrant' = FALSE
    /\ wlapse' = FALSE
    /\ pc' = IF wkind = "current" THEN "take" ELSE "fold"
    /\ UNCHANGED <<today, imported, claimed, sent, ftoday, wday, wkind, rq, cl, fails, crashes,
                   rfails, rcount>>

\* After the fold's commit, the cycle takes the due days.
Take ==
    /\ pc = "take"
    /\ pc' = "route"
    /\ CASE Store = "memory" ->
              /\ rq' = due
              /\ due' = {}
              /\ UNCHANGED outbox
         [] Store \in Committed ->
              /\ rq' = outbox
              /\ outbox' = IF TakeClears THEN {} ELSE outbox
              /\ UNCHANGED due
         [] Store = "ledger" ->
              /\ rq' = (ledger \ claimed) \cup due
              /\ due' = {}
              /\ UNCHANGED outbox
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, claimed, sent, ftoday,
                   w, wday, wkind, wgrant, wlapse, cl, fails, crashes, rfails, rcount>>

\* The router decides one taken day: the first route of a key claims it and sends; with the
\* once-ever key, a later route of a claimed key is withheld. A day in KeepsFailing has no route
\* that succeeds.
Route(d) ==
    /\ pc = "route"
    /\ d \in rq
    /\ d \notin KeepsFailing
    /\ rq' = rq \ {d}
    /\ IF OnceKey /\ d \in claimed
         THEN UNCHANGED <<claimed, sent>>
         ELSE /\ claimed' = claimed \cup {d}
              /\ sent' = [sent EXCEPT ![d] = IF @ < 2 THEN @ + 1 ELSE 2]
    /\ cl' = IF Store \in Committed /\ ~TakeClears THEN cl \cup {d} ELSE cl
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, pc, ftoday,
                   w, wday, wkind, wgrant, wlapse, fails, crashes, rfails, rcount>>

\* After the router's decision, the committed due row is cleared, in a write of its own.
Clear(d) ==
    /\ pc = "route"
    /\ d \in cl
    /\ cl' = cl \ {d}
    /\ outbox' = outbox \ {d}
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, due, claimed, sent, pc, ftoday,
                   w, wday, wkind, wgrant, wlapse, rq, fails, crashes, rfails, rcount>>

EndCycle ==
    /\ pc = "route"
    /\ rq = {}
    /\ cl = {}
    /\ AtRest
    /\ UNCHANGED <<today, cursor, lapse, ledger, imported, outbox, due, claimed, sent,
                   fails, crashes, rfails, rcount>>

Cycle ==
    \/ StartFold
    \/ Open
    \/ Step
    \/ Commit
    \/ Take
    \/ \E d \in AllDays : Route(d)
    \/ \E d \in Days : Clear(d)
    \/ EndCycle

Next == Cycle \/ Advance \/ Import \/ Fail \/ Crash \/ (\E d \in AllDays : RouteFail(d))
        \/ RouteAbort

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* The properties of #477.

\* S1: at most one celebration per relight day, across every order of cycles, failed folds,
\* crashes and restarts.
S1 == \A d \in AllDays : sent[d] <= 1

\* S2: a celebration for a day is sent only when that day's relight grant is committed. The
\* ledger never loses a row, so holding in every state is holding at every send.
S2 == \A d \in AllDays : sent[d] >= 1 => d \in ledger

\* L1: under fair recompute, every committed relight grant is followed by its celebration, also
\* when a crash falls between the commit and the route (SPEC-076 R27). Fairness rides in the
\* property, so Spec stays unchanged.
L1 == WF_vars(Cycle) => \A d \in Days : (d \in ledger) ~> (sent[d] >= 1)

\* SPEC-140 R5, beside #477: an imported relight grant is history, and no celebration is raised
\* for it.
ImportIsHistory == \A d \in ImportDays : sent[d] = 0

-----------------------------------------------------------------------------
\* #446's own property of the failed route.

\* The cycle's own steps and the routes of every day that does not keep failing. A day in
\* KeepsFailing has no route that succeeds, so no fairness here can celebrate it.
OthersCycle ==
    \/ StartFold
    \/ Open
    \/ Step
    \/ Commit
    \/ Take
    \/ \E d \in AllDays \ KeepsFailing : Route(d)
    \/ \E d \in Days : Clear(d)
    \/ EndCycle

\* The route loop's call for a day that keeps failing returns, as an error: announce_relight is
\* awaited, and a call that never returns is a hang, not a failed route.
FailingReturns == \E d \in KeepsFailing : RouteFail(d)

\* NoOtherDayHeldBack (SPEC-076 R27, section 22): a day whose route fails at every attempt holds
\* back no other day's celebration. Under fair recompute, with the failing days' routes failing
\* for ever, every other committed relight grant is followed by its celebration. Fairness is on
\* the other days' routes and the cycle's own steps, never on a failing day's route succeeding.
NoOtherDayHeldBack ==
    (WF_vars(OthersCycle) /\ WF_vars(FailingReturns))
        => \A e \in Days \ KeepsFailing : (e \in ledger) ~> (sent[e] >= 1)

=============================================================================
