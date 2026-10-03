------------------------- MODULE HabitXpFollowsItsLog -------------------------
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=log_minutes digest=sha256:2fe935f11a32dfd0d3d1ea7f9c5c5887ed3d8e24386e0836a2aa66a64e5ef7fb
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=undo_newest digest=sha256:0fed056c4122026b7d0ad27958adf607a614fff28d230cbd0b171f66a0a30796
\* @phx covers crates/coordination/src/habits/minutes.rs anchor=undo_entry digest=sha256:03a6924131cbf448b1345682ee01eaf800bc14ed049bce84f3bb838e1900b432
\* @phx covers crates/coordination/src/recompute/habits.rs anchor=evaluate digest=sha256:5df571be96ed9c281260841fd0662ab7f93621d47146761987714bbd01fde192
\* @phx covers crates/habits/src/minutes.rs anchor=week_start digest=sha256:580735cd07aed4417ba20cad976d40ff325b4860fbbeacc4f2d02669e9356e4a
\* @phx covers crates/progression/src/settle.rs anchor=settle digest=sha256:d5473de04ebe9964bea0c7755b0ac7116ea5d9a45e62a8e342a9a5ff1771eeb7
\* @phx covers crates/coordination/src/level_up.rs anchor=announce_level_up digest=sha256:acbb8923e76fbd4ab77949a119c1ad3a87de6f41737e65b89df689875e851dd0
\* @phx covers crates/coordination/src/habits/writing.rs anchor=confirm digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/habits/writing.rs anchor=clear digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/habits/writing.rs anchor=toggle digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/recompute/writing.rs anchor=evaluate digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/habits/src/writing.rs anchor=all_confirmed_days digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #93, #94
\* @phx property ReadXpFollowsTheLog ramp=report
\* @phx property GoalBonusFollowsItsWeek ramp=report
\* @phx property ALevelUpIsCelebratedOnce ramp=report
\* @phx property WritingXpFollowsItsConfirmations ramp=report
\* @phx property AChipTogglesOnlyItsOwnDay ramp=report
\* @phx witness witness/a-habit-step-that-reads-the-log-before-its-write.cfg kills=ReadXpFollowsTheLog
\* @phx witness witness/an-undo-whose-settle-is-a-write-of-its-own.cfg kills=ReadXpFollowsTheLog
\* @phx witness witness/an-undo-that-re-derives-the-current-week.cfg kills=GoalBonusFollowsItsWeek
\* @phx witness witness/a-bonus-settled-on-the-entrys-own-day.cfg kills=GoalBonusFollowsItsWeek
\* @phx witness witness/a-level-up-announced-without-its-once-ever-key.cfg kills=ALevelUpIsCelebratedOnce
\* @phx witness witness/a-toggle-whose-settle-is-a-write-of-its-own.cfg kills=WritingXpFollowsItsConfirmations
\* @phx witness witness/a-writing-bonus-paid-with-no-writing-course.cfg kills=WritingXpFollowsItsConfirmations
\* @phx witness witness/a-chip-tap-that-toggles-the-day-it-is-tapped-on.cfg kills=AChipTogglesOnlyItsOwnDay

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
\*
\* The writing confirmations and their XP (#94, SPEC-078 R7 and R17 as amended; ADR-078's part 078b
\* amendment). WCourses are the configured writing courses; conf[d] is the set confirmed on study
\* day d (writing_log's rows), wx[c][d] the settled write:<code> of course c on day d and wa[d] the
\* settled write:all. The owner's toggle (coordination habits/writing.rs::confirm, clear, toggle)
\* writes the confirmation and the day's two settles in ONE write, cause the owner's correction, and
\* announces after commit through the same owner state as LogMinutes; Toggle flips a course, which
\* is confirm on an unconfirmed course and clear on a confirmed one. The fold's writing step
\* (recompute/writing.rs::evaluate) settles every writing course and write:all of the evaluated day
\* from its confirmations inside that day's write, so it rides FDay's write (and FWrite's in the
\* rejected early-read design). A chip carries the day it was drawn for (hb:w:<code>:<epoch day>):
\* Draw is the bot drawing today's chips, Tap a press on one, and the toggle refuses a chip whose
\* day is not today and answers with today's chips.
\* - habits/src/writing.rs::all_confirmed_days returns no day over an empty writing set, so write:all
\*   is WAll: the bonus only when WCourses is not empty and every course in it is confirmed.
\* - The write: rows carry no closed flag here. A row is held closed only by a settle on a day
\*   already over, and today never goes back, so held-closed implies d < today, and Settled's
\*   heldClosed \/ reqClosed reads d < today; SettledW is Settled with that substituted.
\* - The writing step also re-settles a write:<code> row of a code that is no longer a writing
\*   course; WCourses is fixed for a run here, so that row never exists (a stutter).
\* - A toggle reaches only today: /write, /unwrite and a chip whose day is today. A past day's
\*   confirmations are therefore fixed once the day is over, which is what lets the recompute keep
\*   the larger amount there.
\* - HabitTotal adds the write: rows to the level's total, so a level the writing XP reaches is
\*   announced through the same Route; MCHabitWriting.cfg checks ALevelUpIsCelebratedOnce over it.
\* Writing switches: Toggle1Write = FALSE is a toggle whose settles are a write of their own after
\* the confirmation's (ADR-078's rejected two writes); ChipCarriesDay = FALSE a chip with no day,
\* which toggles today whenever it is pressed; SubsetRule the predecessor's habits.py writing_day_xp,
\* whose set(codes) <= set(done) pays the bonus over an empty writing set.
(***************************************************************************)
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS Undo1Write, StepReadsEarly, UndoWeekCurrent, BonusOnEntryDay, Dedupe,
          NDays, WeekLen, MaxMinutes, Goal, Per, Cap, Bonus, LevelStep, MaxEntries, Runs
CONSTANTS Toggle1Write, ChipCarriesDay, SubsetRule, NWriting, WPer, WBonus, MaxTaps

Days == 1..NDays
Minutes == 1..MaxMinutes
Entries == [day : Days, min : Minutes]
Logs == UNION {[1..n -> Entries] : n \in 0..MaxEntries}
Levels == 0..((NDays * (Cap + Bonus)) \div LevelStep)
MaxRoutes == 2 * MaxEntries + Runs
MaxSeen == MaxEntries * MaxMinutes
WCourses == 1..NWriting

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

\* the habit XP a level reads: the reading rows and every write: row (#94)
WSum(x, a) == SumOver([c \in WCourses |-> SumOver(x[c], Days)], WCourses) + SumOver(a, Days)
HabitTotal(r, g, x, a) == Total(r, g) + WSum(x, a)

\* habits/src/writing.rs: write:<code> pays WPer on a day the course is confirmed; write:all pays
\* WBonus on a day all_confirmed_days returns, never over an empty writing set as built
WPay(c, C) == IF c \in C THEN WPer ELSE 0
WAll(C) == IF (SubsetRule \/ WCourses # {}) /\ WCourses \subseteq C THEN WBonus ELSE 0

\* settle.rs::settle: the amount and the closed flag the row holds after one settle
Settled(held, heldClosed, reqClosed, amount, cause) ==
    IF cause = "recompute" /\ (heldClosed \/ reqClosed) THEN Max(held, amount) ELSE amount
SettledClosed(heldClosed, reqClosed, cause) ==
    IF cause = "recompute" /\ (heldClosed \/ reqClosed) THEN TRUE ELSE reqClosed

\* the day a habit write settles its week's bonus on, and the week an undo re-derives
GoalDay(d) == IF BonusOnEntryDay THEN d ELSE WeekStart(d)

VARIABLES today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent,
          fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated
VARIABLES conf, wx, wa, wpc, wDay, drawn, tapped, nTaps

vars == <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent,
          fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated,
          conf, wx, wa, wpc, wDay, drawn, tapped, nTaps>>

\* the writing variables (#94), unchanged by every reading action
wvars == <<conf, wx, wa, wpc, wDay, drawn, tapped, nTaps>>

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
    /\ conf \in [Days -> SUBSET WCourses]
    /\ wx \in [WCourses -> [Days -> 0..WPer]]
    /\ wa \in [Days -> 0..WBonus]
    /\ wpc \in {"idle", "settle"}
    /\ wDay \in Days
    /\ drawn \subseteq Days
    /\ tapped \subseteq [drawn : Days, acted : 0..NDays]
    /\ nTaps \in 0..MaxTaps

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
    /\ conf = [d \in Days |-> {}]
    /\ wx = [c \in WCourses |-> [d \in Days |-> 0]]
    /\ wa = [d \in Days |-> 0]
    /\ wpc = "idle"
    /\ wDay = 1
    /\ drawn = {}
    /\ tapped = {}
    /\ nTaps = 0

\* the router: one celebration for the level reached, once ever when it dedupes
Route(before, after) ==
    IF after > before /\ ~(Dedupe /\ after \in claimed)
        THEN /\ claimed' = claimed \cup {after}
             /\ celebrated' = [celebrated EXCEPT ![after] = @ + 1]
        ELSE UNCHANGED <<claimed, celebrated>>

\* settle.rs::settle over a write: row: Settled with heldClosed \/ reqClosed read as d < today
SettledW(held, d, amount, cause) ==
    IF cause = "recompute" /\ d < today THEN Max(held, amount) ELSE amount

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
       /\ oAfter' = Level(HabitTotal(r2, g2, wx, wa))

UndoWeek(d) == IF UndoWeekCurrent THEN today ELSE d

\* minutes.rs::log_minutes: the entry and its settles in one write
LogMinutes(m) ==
    /\ opc = "idle"
    /\ wpc = "idle"
    /\ spent < MaxEntries
    /\ LET lg == Append(log, [day |-> today, min |-> m])
       IN /\ log' = lg
          /\ OwnerSettles(today, today, lg)
    /\ oBefore' = Level(HabitTotal(read, goal, wx, wa))
    /\ spent' = spent + 1
    /\ opc' = "announce"
    /\ UNCHANGED <<today, oDay, fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED wvars

\* minutes.rs::undo_newest, and undo_entry for the newest id: the newest entry removed and its day
\* and week re-derived, in one write as built
Undo ==
    /\ opc = "idle"
    /\ wpc = "idle"
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
    /\ oBefore' = Level(HabitTotal(read, goal, wx, wa))
    /\ UNCHANGED <<today, spent, fpc, fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED wvars

\* the rejected design's second write: the undo's settles after its log write committed
UndoSettle ==
    /\ opc = "settle"
    /\ OwnerSettles(oDay, UndoWeek(oDay), log)
    /\ opc' = "announce"
    /\ UNCHANGED <<today, log, oDay, oBefore, spent, fpc, fDone, fDay, fSeenD, fSeenW, fBefore,
                   runs, claimed, celebrated>>
    /\ UNCHANGED wvars

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
    /\ UNCHANGED wvars

OwnerCrash ==
    /\ opc \in {"settle", "announce"}
    /\ OwnerIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, spent, fpc, fDone, fDay, fSeenD, fSeenW,
                   fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED wvars

\* a sync cycle starts: it reads the level before its fold
FStart ==
    /\ fpc = "idle"
    /\ runs < Runs
    /\ fpc' = "day"
    /\ runs' = runs + 1
    /\ fDone' = {}
    /\ fBefore' = Level(HabitTotal(read, goal, wx, wa))
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fDay,
                   fSeenD, fSeenW, claimed, celebrated>>
    /\ UNCHANGED wvars

\* recompute/habits.rs::evaluate's settles of day d, from minutes md on the day and mw in its week
FoldWrite(d, md, mw) ==
    /\ read' = [read EXCEPT ![d] = Settled(read[d], readC[d], d < today, ReadXp(md), "recompute")]
    /\ readC' = [readC EXCEPT ![d] = SettledClosed(readC[d], d < today, "recompute")]
    /\ IF d = WeekStart(d)
          THEN /\ goal' = [goal EXCEPT ![d] =
                              Settled(goal[d], goalC[d], d < today, GoalXp(mw), "recompute")]
               /\ goalC' = [goalC EXCEPT ![d] = SettledClosed(goalC[d], d < today, "recompute")]
          ELSE UNCHANGED <<goal, goalC>>

\* recompute/writing.rs::evaluate's settles of day d: every writing course and write:all, from the
\* day's confirmations, inside the same day's write
WritingFoldWrite(d) ==
    /\ wx' = [c \in WCourses |-> [wx[c] EXCEPT ![d] = SettledW(wx[c][d], d, WPay(c, conf[d]), "recompute")]]
    /\ wa' = [wa EXCEPT ![d] = SettledW(wa[d], d, WAll(conf[d]), "recompute")]

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
               /\ UNCHANGED <<wx, wa>>
          ELSE /\ FoldWrite(d, MinOn(log, d), MinInWeek(log, WeekStart(d)))
               /\ WritingFoldWrite(d)
               /\ fDone' = fDone \cup {d}
               /\ UNCHANGED <<fpc, fDay, fSeenD, fSeenW>>
    /\ UNCHANGED <<today, log, opc, oDay, oBefore, oAfter, spent, fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED <<conf, wpc, wDay, drawn, tapped, nTaps>>

\* the rejected design's later write, from what the step read before it
FWrite ==
    /\ fpc = "write"
    /\ FoldWrite(fDay, fSeenD, fSeenW)
    /\ WritingFoldWrite(fDay)
    /\ fDone' = fDone \cup {fDay}
    /\ fpc' = "day"
    /\ UNCHANGED <<today, log, opc, oDay, oBefore, oAfter, spent, fDay, fSeenD, fSeenW, fBefore,
                   runs, claimed, celebrated>>
    /\ UNCHANGED <<conf, wpc, wDay, drawn, tapped, nTaps>>

\* the fold has evaluated the days it owed
FEnd ==
    /\ fpc = "day"
    /\ fpc' = "announce"
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fDone,
                   fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED wvars

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
    /\ Route(fBefore, Level(HabitTotal(read, goal, wx, wa)))
    /\ FoldIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, runs>>
    /\ UNCHANGED wvars

FCrash ==
    /\ fpc \in {"day", "write", "announce"}
    /\ FoldIdle
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, runs,
                   claimed, celebrated>>
    /\ UNCHANGED wvars

\* the study day turns over
Rollover ==
    /\ today < NDays
    /\ today' = today + 1
    /\ UNCHANGED <<log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fpc, fDone,
                   fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated>>
    /\ UNCHANGED wvars

\* habits/writing.rs::toggle, and confirm or clear on the course's current state: the confirmation
\* and the day's write:<code> and write:all settles in one write, cause the owner's correction (the
\* owner's settle replaces the amount); the level is read before the write and after its commit
ToggleW(c) ==
    LET c2 == IF c \in conf[today] THEN conf[today] \ {c} ELSE conf[today] \cup {c}
    IN /\ conf' = [conf EXCEPT ![today] = c2]
       /\ oBefore' = Level(HabitTotal(read, goal, wx, wa))
       /\ IF Toggle1Write
             THEN LET x2 == [wx EXCEPT ![c][today] = WPay(c, c2)]
                      a2 == [wa EXCEPT ![today] = WAll(c2)]
                  IN /\ wx' = x2
                     /\ wa' = a2
                     /\ oAfter' = Level(HabitTotal(read, goal, x2, a2))
                     /\ opc' = "announce"
                     /\ UNCHANGED <<wpc, wDay>>
             ELSE /\ wpc' = "settle"
                  /\ wDay' = today
                  /\ UNCHANGED <<wx, wa, oAfter, opc>>

\* /write and /unwrite on today's study day
Toggle(c) ==
    /\ opc = "idle"
    /\ wpc = "idle"
    /\ ToggleW(c)
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, oDay, spent, fpc, fDone, fDay, fSeenD,
                   fSeenW, fBefore, runs, claimed, celebrated, drawn, tapped, nTaps>>

\* the rejected design's second write: the toggled day's settles after its confirmation committed
ToggleSettle ==
    /\ wpc = "settle"
    /\ LET x2 == [c \in WCourses |-> [wx[c] EXCEPT ![wDay] = WPay(c, conf[wDay])]]
           a2 == [wa EXCEPT ![wDay] = WAll(conf[wDay])]
       IN /\ wx' = x2
          /\ wa' = a2
          /\ oAfter' = Level(HabitTotal(read, goal, x2, a2))
    /\ opc' = "announce"
    /\ wpc' = "idle"
    /\ wDay' = 1
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, oDay, oBefore, spent, fpc, fDone, fDay,
                   fSeenD, fSeenW, fBefore, runs, claimed, celebrated, conf, drawn, tapped, nTaps>>

\* the owner ends between the rejected design's two writes; the confirmation committed stays
ToggleCrash ==
    /\ wpc = "settle"
    /\ wpc' = "idle"
    /\ wDay' = 1
    /\ oBefore' = 0
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oAfter, spent, fpc, fDone, fDay,
                   fSeenD, fSeenW, fBefore, runs, claimed, celebrated, conf, wx, wa, drawn, tapped,
                   nTaps>>

\* the bot draws today's chips (habits_commands.rs's hb:w:<code>:<epoch day>)
Draw ==
    /\ WCourses # {}
    /\ opc = "idle"
    /\ drawn' = drawn \cup {today}
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, opc, oDay, oBefore, oAfter, spent, fpc,
                   fDone, fDay, fSeenD, fSeenW, fBefore, runs, claimed, celebrated, conf, wx, wa,
                   wpc, wDay, tapped, nTaps>>

\* a press on a chip drawn for day dd: as built the toggle refuses a chip day that is not today
\* (habits/writing.rs's `if chip_day != today`) and answers with today's chips; tapped records the
\* day the chip was drawn for and the day it acted on, 0 for none
Tap(c, dd) ==
    /\ opc = "idle"
    /\ wpc = "idle"
    /\ dd \in drawn
    /\ nTaps < MaxTaps
    /\ nTaps' = nTaps + 1
    /\ IF ~ChipCarriesDay \/ dd = today
          THEN /\ ToggleW(c)
               /\ tapped' = tapped \cup {[drawn |-> dd, acted |-> today]}
               /\ UNCHANGED drawn
          ELSE /\ tapped' = tapped \cup {[drawn |-> dd, acted |-> 0]}
               /\ drawn' = drawn \cup {today}
               /\ UNCHANGED <<conf, wx, wa, wpc, wDay, opc, oBefore, oAfter>>
    /\ UNCHANGED <<today, log, read, readC, goal, goalC, oDay, spent, fpc, fDone, fDay, fSeenD,
                   fSeenW, fBefore, runs, claimed, celebrated>>

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
    \/ \E c \in WCourses : Toggle(c)
    \/ ToggleSettle
    \/ ToggleCrash
    \/ Draw
    \/ \E c \in WCourses, dd \in Days : Tap(c, dd)
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

\* SPEC-078 R7: with no toggle in flight, each day's write:<code> pays WPer exactly when the course
\* is confirmed, and write:all pays WBonus exactly when the writing set is not empty and every course
\* in it is confirmed (habits/src/writing.rs::all_confirmed_days returns no day over an empty set)
WritingXpFollowsItsConfirmations ==
    (NoWriteInFlight /\ wpc # "settle") =>
        \A d \in Days :
            /\ \A c \in WCourses : wx[c][d] = IF c \in conf[d] THEN WPer ELSE 0
            /\ wa[d] = IF WCourses # {} /\ WCourses \subseteq conf[d] THEN WBonus ELSE 0

\* SPEC-078 R6 (a tap flips today's confirmation; the chip carries its day): every chip press
\* toggled the day the chip was drawn for, or no day at all
AChipTogglesOnlyItsOwnDay == \A t \in tapped : t.acted \in {t.drawn, 0}
================================================================================
