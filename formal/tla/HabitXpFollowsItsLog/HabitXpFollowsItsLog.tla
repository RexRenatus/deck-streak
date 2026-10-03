------------------------- MODULE HabitXpFollowsItsLog -------------------------
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=log_minutes digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=undo_newest digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=undo_entry digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/recompute/habits.rs anchor=evaluate digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/habits/src/minutes.rs anchor=week_start digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/progression/src/settle.rs anchor=settle digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/level_up.rs anchor=announce_level_up digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #93
\* @phx property ReadXpFollowsTheLog ramp=report
\* @phx property GoalBonusFollowsItsWeek ramp=report
\* @phx property ALevelUpIsCelebratedOnce ramp=report
\* @phx witness witness/a-habit-step-that-reads-the-log-before-its-write.cfg kills=ReadXpFollowsTheLog
\* @phx witness witness/an-undo-whose-settle-is-a-write-of-its-own.cfg kills=ReadXpFollowsTheLog
\* @phx witness witness/an-undo-that-re-derives-the-current-week.cfg kills=GoalBonusFollowsItsWeek
\* @phx witness witness/a-bonus-settled-on-the-entrys-own-day.cfg kills=GoalBonusFollowsItsWeek
\* @phx witness witness/a-level-up-announced-without-its-once-ever-key.cfg kills=ALevelUpIsCelebratedOnce

(***************************************************************************)
\* The minutes log of book reading and its XP (#93, SPEC-078 R3 to R5 and R18; ADR-078). Two actors
\* write the log and its settled XP, and two raise a level-up, over one database while the study day
\* turns over:
\* - the owner's use cases (coordination habits/minutes.rs::log_minutes, undo_newest, undo_entry),
\*   reached from the bot today and from any later adapter;
\* - the fold's habit step (recompute/habits.rs::evaluate), run by the scheduled and the owner's
\*   cycles, inside each evaluated day's write; the sync cycle also announces the level it reached.
\*
\* One course. log is the minutes log, newest last, each entry a study day and its minutes. read[d]
\* is the settled read:<code> on day d, goal[d] the settled readgoal:<code>, and readC, goalC whether
\* each row is held closed. settle.rs::settle is Settled: a recompute keeps the larger amount when
\* the row is held closed or the day is over, and replaces it otherwise; the owner's correction
\* replaces it whatever it was. An absent row reads as 0, open: settle treats both alike, and a zero
\* the use case skips for want of a row changes no amount.
\*
\* LogMinutes and Undo are each ONE action: habits/minutes.rs runs the log write and both settles in
\* one db.write (BEGIN IMMEDIATE), the read:<code> of the entry's day and the readgoal:<code> of its
\* week's first study day (minutes.rs::week_start, here WeekStart), with the owner's correction as the
\* cause and closed when the day is before today. undo_entry with an id that is not the newest
\* writes nothing (a stutter); with the newest id it is Undo. FDay is ONE action: the habit step
\* reads the day's and the week's minutes and settles in the day's own write, with the recompute as
\* the cause; it settles readgoal:<code> only when the day is its week's first. The fold evaluates
\* the days with reviews and today, so FDay may take any day up to today, each once a run. The use
\* case reads the level before and after its write and announces after commit; the cycle reads it
\* before its fold and after it (level_up.rs::announce_level_up, which routes the one key level:N
\* for the level reached, and the router's once-ever dedupe).
\*
\* The switches each place one rejected design: Undo1Write = FALSE is an undo whose settle is a
\* write of its own after the log write (ADR-078's rejected two writes); StepReadsEarly a habit step
\* that reads the log in one write and settles in a later one; UndoWeekCurrent an undo that
\* re-derives the current week rather than its entry's; BonusOnEntryDay a weekly bonus settled on the
\* entry's own day; Dedupe = FALSE a router with no once-ever key. Crash ends an actor between two
\* of its steps, and what it committed stays.
\*
\* MintReadsTheFinalBase covers settle.rs::settle too: SPEC-078 changes only its first line, the
\* registry test, from the nine names to is_derived, which refuses before any write.
\*
\* Two abstractions, re-read against the covered spans when they were first stamped:
\* - Undo announces nothing here, and the code's undo_newest and undo_entry skip the announcement:
\*   an undo only lowers or keeps the held amounts, so the level after it is never above the level
\*   before it, and announce_level_up routes nothing when after <= before. The skipped call is a
\*   stutter.
\* - log_minutes reads `before` just before its write and `after` just after its commit
\*   (minutes.rs::level_before, celebrate); the model reads both inside LogMinutes. A fold commit
\*   that lands between the read and the write, or between the commit and the read, can make the
\*   use case see a crossing the cycle also sees; both route the one key level:N, and the router's
\*   once-ever dedupe absorbs the second, which is what ALevelUpIsCelebratedOnce states.
(***************************************************************************)
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS Undo1Write, StepReadsEarly, UndoWeekCurrent, BonusOnEntryDay, Dedupe,
          NDays, WeekLen, MaxMinutes, Goal, Per, Cap, Bonus, LevelStep, MaxEntries, Runs

Days == 1..NDays
Minutes == 1..MaxMinutes
Entries == [day : Days, min : Minutes]
Logs == UNION {[1..n -> Entries] : n \in 0..MaxEntries}
Levels == 0..((NDays * (Cap + Bonus)) \div LevelStep)
MaxRoutes == 2 * MaxEntries + Runs
MaxSeen == MaxEntries * MaxMinutes

Max(a, b) == IF a >= b THEN a ELSE b

\* habits/src/minutes.rs::week_start: the first study day of d's week
WeekStart(d) == d - ((d - 1) % WeekLen)

RECURSIVE MinOn(_, _)
MinOn(lg, d) ==
    IF lg = <<>> THEN 0
    ELSE (IF Head(lg).day = d THEN Head(lg).min ELSE 0) + MinOn(Tail(lg), d)

RECURSIVE MinInWeek(_, _)
MinInWeek(lg, ws) ==
    IF lg = <<>> THEN 0
    ELSE (IF WeekStart(Head(lg).day) = ws THEN Head(lg).min ELSE 0) + MinInWeek(Tail(lg), ws)

\* SPEC-078 R3: min(cap, per x minutes) a day; the bonus when the week's minutes reach the goal
ReadXp(m) == IF Per * m > Cap THEN Cap ELSE Per * m
GoalXp(m) == IF m >= Goal THEN Bonus ELSE 0

RECURSIVE SumOver(_, _)
SumOver(f, S) ==
    IF S = {} THEN 0
    ELSE LET d == CHOOSE x \in S : TRUE IN f[d] + SumOver(f, S \ {d})

Total(r, g) == SumOver(r, Days) + SumOver(g, Days)
Level(x) == x \div LevelStep

\* settle.rs::settle: the amount and the closed flag the row holds after one settle
Settled(held, heldClosed, reqClosed, amount, cause) ==
    IF cause = "recompute" /\ (heldClosed \/ reqClosed) THEN Max(held, amount) ELSE amount
SettledClosed(heldClosed, reqClosed, cause) ==
    IF cause = "recompute" /\ (heldClosed \/ reqClosed) THEN TRUE ELSE reqClosed

\* the day a habit write settles its week's bonus on, and the week an undo re-derives
GoalDay(d) == IF BonusOnEntryDay THEN d ELSE WeekStart(d)

VARIABLES today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent,
          fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated

vars == <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent,
          fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>

TypeOK ==
    /\ today \in Days
    /\ log \in Logs
    /\ read \in [Days -> 0..Cap]
    /\ readC \in [Days -> BOOLEAN]
    /\ goal \in [Days -> 0..Bonus]
    /\ goalC \in [Days -> BOOLEAN]
    /\ opc \in {"idle", "settle", "announce"}
    /\ oDay \in Days
    /\ oBefore \in Levels
    /\ oAfter \in Levels
    /\ spent \in 0..MaxEntries
    /\ fpc \in {"idle", "day", "write", "announce"}
    /\ fDone \subseteq Days
    /\ fDay \in Days
    /\ fSeenD \in 0..MaxSeen
    /\ fSeenW \in 0..MaxSeen
    /\ fBefore \in Levels
    /\ runs \in 0..Runs
    /\ claimed \subseteq Levels
    /\ celebrated \in [Levels -> 0..MaxRoutes]

Init ==
    /\ today = 1
    /\ log = <<>>
    /\ read = [d \in Days |-> 0]
    /\ readC = [d \in Days |-> FALSE]
    /\ goal = [d \in Days |-> 0]
    /\ goalC = [d \in Days |-> FALSE]
    /\ opc = "idle"
    /\ oDay = 1
    /\ oBefore = 0
    /\ oAfter = 0
    /\ spent = 0
    /\ fpc = "idle"
    /\ fDone = {}
    /\ fDay = 1
    /\ fSeenD = 0
    /\ fSeenW = 0
    /\ fBefore = 0
    /\ runs = 0
    /\ claimed = {}
    /\ celebrated = [l \in Levels |-> 0]

\* the router: one celebration for the level reached, once ever when it dedupes
Route(before, after) ==
    IF after > before /\ ~(Dedupe /\ after \in claimed)
        THEN /\ claimed' = claimed \cup {after}
             /\ celebrated' = [celebrated EXCEPT ![after] = @ + 1]
        ELSE UNCHANGED <<claimed, celebrated>>

\* the settles of one habit write: read:<code> on day d, readgoal:<code> for the week of w, over lg
OwnerSettles(d, w, lg) ==
    LET gd == GoalDay(w)
        r2 == [read EXCEPT ![d] =
                  Settled(read[d], readC[d], d < today, ReadXp(MinOn(lg, d)), "owner")]
        g2 == [goal EXCEPT ![gd] =
                  Settled(goal[gd], goalC[gd], gd < today, GoalXp(MinInWeek(lg, WeekStart(w))), "owner")]
    IN /\ read' = r2
       /\ readC' = [readC EXCEPT ![d] = SettledClosed(readC[d], d < today, "owner")]
       /\ goal' = g2
       /\ goalC' = [goalC EXCEPT ![gd] = SettledClosed(goalC[gd], gd < today, "owner")]
       /\ oAfter' = Level(Total(r2, g2))

UndoWeek(d) == IF UndoWeekCurrent THEN today ELSE d

\* minutes.rs::log_minutes: the entry and its settles in one write
LogMinutes(m) ==
    /\ opc = "idle"
    /\ spent < MaxEntries
    /\ LET lg == Append(log, [day |-> today, min |-> m])
       IN /\ log' = lg
          /\ OwnerSettles(today, today, lg)
    /\ oBefore' = Level(Total(read, goal))
    /\ spent' = spent + 1
    /\ opc' = "announce"
    /\ UNCHANGED <<today, oDay, fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>

\* minutes.rs::undo_newest, and undo_entry for the newest id: the newest entry removed and its day
\* and week re-derived, in one write as built
Undo ==
    /\ opc = "idle"
    /\ log # <<>>
    /\ LET e == log[Len(log)]
           lg == SubSeq(log, 1, Len(log) - 1)
       IN /\ log' = lg
          /\ IF Undo1Write
                THEN /\ OwnerSettles(e.day, UndoWeek(e.day), lg)
                     /\ opc' = "announce"
                     /\ UNCHANGED oDay
                ELSE /\ UNCHANGED <<read, readC, goal, goalC, oAfter>>
                     /\ oDay' = e.day
                     /\ opc' = "settle"
    /\ oBefore' = Level(Total(read, goal))
    /\ UNCHANGED <<today, spent, fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>

\* the rejected design's second write: the undo's settles after its log write committed
UndoSettle ==
    /\ opc = "settle"
    /\ OwnerSettles(oDay, UndoWeek(oDay), log)
    /\ opc' = "announce"
    /\ UNCHANGED <<today, log, oDay, oBefore, spent, fpc, fDone, fDay, fSeenD, fSeenW, fBefore,
                   runs, claimed, celebrated>>

\* the owner's step-local values, which no step reads once it is idle again
OwnerIdle ==
    /\ opc' = "idle"
    /\ oDay' = 1
    /\ oBefore' = 0
    /\ oAfter' = 0

\* after commit: announce_level_up(router, before, after, today)
OwnerAnnounce ==
    /\ opc = "announce"
    /\ Route(oBefore, oAfter)
    /\ OwnerIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, spent, fpc, fDone, fDay, fSeenD, fSeenW,
                   fBefore, runs>>

OwnerCrash ==
    /\ opc \in {"settle", "announce"}
    /\ OwnerIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, spent, fpc, fDone, fDay, fSeenD, fSeenW,
                   fBefore, runs, claimed, celebrated>>

\* a sync cycle starts: it reads the level before its fold
FStart ==
    /\ fpc = "idle"
    /\ runs < Runs
    /\ fpc' = "day"
    /\ runs' = runs + 1
    /\ fDone' = {}
    /\ fBefore' = Level(Total(read, goal))
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fDay,
                   fSeenD, fSeenW, claimed, celebrated>>

\* recompute/habits.rs::evaluate's settles of day d, from minutes md on the day and mw in its week
FoldWrite(d, md, mw) ==
    /\ read' = [read EXCEPT ![d] = Settled(read[d], readC[d], d < today, ReadXp(md), "recompute")]
    /\ readC' = [readC EXCEPT ![d] = SettledClosed(readC[d], d < today, "recompute")]
    /\ IF d = WeekStart(d)
          THEN /\ goal' = [goal EXCEPT ![d] =
                              Settled(goal[d], goalC[d], d < today, GoalXp(mw), "recompute")]
               /\ goalC' = [goalC EXCEPT ![d] = SettledClosed(goalC[d], d < today, "recompute")]
          ELSE UNCHANGED <<goal, goalC>>

\* the habit step on one evaluated day, inside that day's write
FDay(d) ==
    /\ fpc = "day"
    /\ d \in (1..today) \ fDone
    /\ IF StepReadsEarly
          THEN /\ fDay' = d
               /\ fSeenD' = MinOn(log, d)
               /\ fSeenW' = MinInWeek(log, WeekStart(d))
               /\ fpc' = "write"
               /\ UNCHANGED <<read, readC, goal, goalC, fDone>>
          ELSE /\ FoldWrite(d, MinOn(log, d), MinInWeek(log, WeekStart(d)))
               /\ fDone' = fDone \cup {d}
               /\ UNCHANGED <<fpc, fDay, fSeenD, fSeenW>>
    /\ UNCHANGED <<today, log, opc, oDay, oBefore, oAfter, spent, fBefore, runs, claimed, celebrated>>

\* the rejected design's later write, from what the step read before it
FWrite ==
    /\ fpc = "write"
    /\ FoldWrite(fDay, fSeenD, fSeenW)
    /\ fDone' = fDone \cup {fDay}
    /\ fpc' = "day"
    /\ UNCHANGED <<today, log, opc, oDay, oBefore, oAfter, spent, fDay, fSeenD, fSeenW, fBefore,
                   runs, claimed, celebrated>>

\* the fold has evaluated the days it owed
FEnd ==
    /\ fpc = "day"
    /\ fpc' = "announce"
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fDone,
                   fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>

\* the cycle's run-local values, which no step reads once it is idle again
FoldIdle ==
    /\ fpc' = "idle"
    /\ fDone' = {}
    /\ fDay' = 1
    /\ fSeenD' = 0
    /\ fSeenW' = 0
    /\ fBefore' = 0

\* the cycle reads the level after its fold and announces it
FAnnounce ==
    /\ fpc = "announce"
    /\ Route(fBefore, Level(Total(read, goal)))
    /\ FoldIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, runs>>

FCrash ==
    /\ fpc \in {"day", "write", "announce"}
    /\ FoldIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, runs,
                   claimed, celebrated>>

\* the study day turns over
Rollover ==
    /\ today < NDays
    /\ today' = today + 1
    /\ UNCHANGED <<log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fpc, fDone,
                   fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>

\* the last day: both actors idle, every run and every entry spent, and every entry undone
Finished ==
    /\ today = NDays
    /\ opc = "idle"
    /\ fpc = "idle"
    /\ runs = Runs
    /\ spent = MaxEntries
    /\ log = <<>>
    /\ UNCHANGED vars

Next ==
    \/ \E m \in Minutes : LogMinutes(m)
    \/ Undo
    \/ UndoSettle
    \/ OwnerAnnounce
    \/ OwnerCrash
    \/ FStart
    \/ \E d \in Days : FDay(d)
    \/ FWrite
    \/ FEnd
    \/ FAnnounce
    \/ FCrash
    \/ Rollover
    \/ Finished

Spec == Init /\ [][Next]_vars

\* no actor holds a log write whose settles are still to come
NoWriteInFlight == opc # "settle" /\ fpc # "write"

\* SPEC-078 R3 to R5: every day's read:<code> is the rule over the log's minutes on that day
ReadXpFollowsTheLog ==
    NoWriteInFlight => \A d \in Days : read[d] = ReadXp(MinOn(log, d))

\* SPEC-078 R3 and R4: each week's first study day holds the bonus its week earns, and no other day
\* holds one
GoalBonusFollowsItsWeek ==
    NoWriteInFlight =>
        \A d \in Days : goal[d] = IF d = WeekStart(d) THEN GoalXp(MinInWeek(log, d)) ELSE 0

\* SPEC-078 R18: a level is celebrated at most once, whichever actor crosses it
ALevelUpIsCelebratedOnce == \A l \in Levels : celebrated[l] <= 1
================================================================================
