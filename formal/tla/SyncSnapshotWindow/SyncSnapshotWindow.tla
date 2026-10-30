------------------------- MODULE SyncSnapshotWindow -------------------------
\* @phx covers deploy/scripts/backup.py anchor=window digest=sha256:b9e9aa5dead439dfd5ba5318a7a083b7cd9c74ad87a0f3f0f0cb690af6db8431
\* @phx covers deploy/scripts/backup.py anchor=copy_stopped digest=sha256:c4a13976b4ee1a275a4deee7961f1a1ea7dc100dc7401ceb42ab1c5299cf0147
\* @phx covers deploy/scripts/backup.py anchor=refuse_a_holder digest=sha256:6917f2b2a52e57ac1e48006e8dc87a6e255f788c316115d9a4b9c9de892a5cd0
\* @phx cites #617, #647
\* @phx property CopyOnlyWhileStopped ramp=report
\* @phx property PairFromOneGeneration ramp=report
\* @phx property ASyncInTheWindowIsRefusedWhole ramp=report
\* @phx property TheServerIsUpAfterEveryWindow ramp=report
\* @phx witness witness/a-copy-not-ordered-after-the-stop.cfg kills=CopyOnlyWhileStopped
\* @phx witness witness/a-generation-published-before-it-is-whole.cfg kills=PairFromOneGeneration
\* @phx witness witness/a-session-committed-step-by-step.cfg kills=ASyncInTheWindowIsRefusedWhole
\* @phx witness witness/a-write-that-lands-during-the-copy.cfg kills=ASyncInTheWindowIsRefusedWhole
\* @phx witness witness/a-failed-copy-that-starts-nothing.cfg kills=TheServerIsUpAfterEveryWindow
\* @phx witness witness/a-window-gated-by-a-condition.cfg kills=TheServerIsUpAfterEveryWindow
\*
\* The sync server's stopped-server window (ADR-347 D12, SPEC-337 R5). The server holds each user's
\* media index in SQLite's exclusive locking mode for its whole life, so the snapshot copies the store
\* with the server stopped. The daily backup's run pulls in deck-streak-sync-snapshot.service, a
\* oneshot whose `Conflicts=` stops the server, ordered (`After=`) so the stop completes before the
\* copy starts; PID 1 starts the server again when the window ends, on success and on failure alike
\* (`OnSuccess=`, `OnFailure=`). The copy, `backup.py --sync-window`, takes each database by the
\* online backup, refused at once while another process holds it, into one generation that is
\* published by a rename only when whole. Meanwhile the owner's clients sync: a session begins only
\* while the server is up, applies and finishes while it is up or draining after its interrupt, and
\* the server's exit cuts it (#617, #647).
\*
\* What the model abstracts, and why:
\* - One user's store stands for each: the window copies every user's folder the same way, and a
\*   user's sessions write that user's folder only. `ver` is the store's generation: every committed
\*   write of either database moves it, so two copies taken at one `ver` are of one generation.
\* - One session in flight at a time, `MaxSessions` in all: a client syncs one session at a time.
\*   A session's writes are two steps, its changes and its finish; under `AtomicSession` both commit
\*   at the finish (the engine's collection sync runs in one transaction, begun at its start and
\*   committed at its finish, at the engine pin's server_start and server_finish), so a cut session
\*   commits nothing.
\* - The collection is never refused as held: it was measured readable by a second process after a
\*   full upload and a full download, so the model takes the worst case. The media index is held for
\*   as long as a server process runs (measured).
\* - The server's stop is two steps: the interrupt (`up` to `stopping`, the stop job's start) and the
\*   exit (`stopping` to `down`), bounded by the unit's stop timeout. A start of the server by hand
\*   while the window runs is outside the model: `Conflicts=` then stops the window first, and an
\*   unpublished generation publishes nothing.
\* - `MaxWindows` daily runs and `MaxFail` failed copies (a planted failure, a refusal or the start
\*   timeout) bound the runs. A later run's trigger replaces a start PID 1 still holds for the server
\*   (the window's `Conflicts=` turns the queued start into a stop).
\*
\* The switches are the fixed design when `OrderAfterStop`, `RefuseHolder`, `PublishWhole`,
\* `StartOnFailure` and `AtomicSession` are TRUE and `GatedByCondition` is FALSE: `OrderAfterStop` is
\* the window's `After=` the server, `RefuseHolder` the progress callback that ends a copy on a busy
\* or locked step, `PublishWhole` the rename of a whole generation, `StartOnFailure` the server named
\* in `OnFailure=` as well as `OnSuccess=`, `GatedByCondition` a Condition on the window (checked after
\* the stop; a skipped start fires neither `OnSuccess=` nor `OnFailure=`), and `AtomicSession` the
\* engine's one transaction per session. No property rests on `RefuseHolder` alone: with the order
\* in place every copy runs after the exit, and without it the collection, never refused, is read
\* while the server runs. It is the second guard, and the model keeps it so that `Fail` carries the
\* refusal the code makes.
\*
\* The action-to-code map, by `file::item`:
\* - Trigger -> the backup's run pulling in the window (`WantedBy=deck-streak-backup.service`), whose
\*   `Conflicts=` enqueues the server's stop.
\* - Begin, Skip -> PID 1 running the window's start job once the stop is done (`After=`); Skip is a
\*   failed Condition, which the fixed design does not carry.
\* - CopyColl, CopyMedia -> `backup.py::window`, each database by `backup.py::copy_stopped`, whose
\*   progress callback is `backup.py::refuse_a_holder`; the publish is `window`'s `os.replace` of the
\*   whole generation.
\* - Fail -> `window` returning 1 on a failed step, or the start timeout ending it.
\* - StartRuns -> the start job `OnSuccess=` or `OnFailure=` enqueued at the window's end.
\* - Exit -> the server's exit after its interrupt (`KillSignal=SIGINT`), which cuts any session.
\* - SessBegin, SessApply, SessFinish -> a client's sync session against the running server.

EXTENDS Integers

CONSTANTS OrderAfterStop, RefuseHolder, PublishWhole, StartOnFailure, GatedByCondition,
          AtomicSession, MaxWindows, MaxSessions, MaxFail

MaxVer == 2 * MaxSessions
NoCopy == -1
Running == {"up", "stopping"}
Copying == {"coll", "media"}
WinPlaces == {"idle", "waiting", "coll", "media", "ok", "fail", "skipped"}
WinEnds == {"idle", "ok", "fail", "skipped"}

VARIABLES srv, win, pending, windows, fails, sess, begun, ver, cp, pub, half, wroteInCopy

vars == <<srv, win, pending, windows, fails, sess, begun, ver, cp, pub, half, wroteInCopy>>

Copied == {NoCopy} \cup (0..MaxVer)

TypeOK ==
    /\ srv \in {"up", "stopping", "down"}
    /\ win \in WinPlaces
    /\ pending \in BOOLEAN
    /\ windows \in 0..MaxWindows
    /\ fails \in 0..MaxFail
    /\ sess \in {"none", "open", "part"}
    /\ begun \in 0..MaxSessions
    /\ ver \in 0..MaxVer
    /\ cp \in [c : Copied, m : Copied, cu : BOOLEAN, mu : BOOLEAN]
    /\ pub \in [c : 0..MaxVer, m : 0..MaxVer, cu : BOOLEAN, mu : BOOLEAN]
    /\ half \in BOOLEAN
    /\ wroteInCopy \in BOOLEAN

NoneCopied == [c |-> NoCopy, m |-> NoCopy, cu |-> FALSE, mu |-> FALSE]

\* The generation the store held before the first window, taken while the server was stopped.
Init ==
    /\ srv = "up"
    /\ win = "idle"
    /\ pending = FALSE
    /\ windows = 0
    /\ fails = 0
    /\ sess = "none"
    /\ begun = 0
    /\ ver = 0
    /\ cp = NoneCopied
    /\ pub = [c |-> 0, m |-> 0, cu |-> FALSE, mu |-> FALSE]
    /\ half = FALSE
    /\ wroteInCopy = FALSE

\* The backup's run starts the window: the server's stop is enqueued and any start PID 1 still holds
\* for the server is replaced by it.
Trigger ==
    /\ win \in WinEnds
    /\ windows < MaxWindows
    /\ win' = "waiting"
    /\ windows' = windows + 1
    /\ pending' = FALSE
    /\ srv' = IF srv = "up" THEN "stopping" ELSE srv
    /\ cp' = NoneCopied
    /\ UNCHANGED <<fails, sess, begun, ver, pub, half, wroteInCopy>>

\* The window's start job runs: after the server's stop is done when it is ordered after it.
Ready == win = "waiting" /\ (OrderAfterStop => srv = "down")

Begin ==
    /\ Ready
    /\ win' = "coll"
    /\ UNCHANGED <<srv, pending, windows, fails, sess, begun, ver, cp, pub, half, wroteInCopy>>

\* A failed Condition: the start is skipped, and a skipped start fires neither `OnSuccess=` nor
\* `OnFailure=` ("man systemd.unit", Conditions and Asserts).
Skip ==
    /\ GatedByCondition
    /\ Ready
    /\ win' = "skipped"
    /\ UNCHANGED <<srv, pending, windows, fails, sess, begun, ver, cp, pub, half, wroteInCopy>>

\* The collection, by the online backup: never refused, whatever runs (the worst case).
CopyColl ==
    /\ win = "coll"
    /\ win' = "media"
    /\ cp' = [cp EXCEPT !.c = ver, !.cu = srv \in Running]
    /\ pub' = IF PublishWhole THEN pub ELSE [pub EXCEPT !.c = ver, !.cu = srv \in Running]
    /\ UNCHANGED <<srv, pending, windows, fails, sess, begun, ver, half, wroteInCopy>>

\* The media index, refused while a server process holds it; the whole generation is published.
CopyMedia ==
    /\ win = "media"
    /\ ~(RefuseHolder /\ srv \in Running)
    /\ win' = "ok"
    /\ cp' = [cp EXCEPT !.m = ver, !.mu = srv \in Running]
    /\ pub' = IF PublishWhole THEN cp' ELSE [pub EXCEPT !.m = ver, !.mu = srv \in Running]
    /\ pending' = TRUE
    /\ UNCHANGED <<srv, windows, fails, sess, begun, ver, half, wroteInCopy>>

\* A copy ends in failure: a refusal of a held database, or any failed step, bounded in all except
\* the refusal, which the holder decides.
Refused == win = "media" /\ RefuseHolder /\ srv \in Running

Fail ==
    /\ win \in Copying
    /\ Refused \/ fails < MaxFail
    /\ win' = "fail"
    /\ fails' = IF Refused THEN fails ELSE fails + 1
    /\ pending' = StartOnFailure
    /\ UNCHANGED <<srv, windows, sess, begun, ver, cp, pub, half, wroteInCopy>>

WindowMoves == Begin \/ Skip \/ CopyColl \/ CopyMedia \/ Fail

\* PID 1 runs the start it holds for the server once the server is down.
StartRuns ==
    /\ pending
    /\ srv = "down"
    /\ srv' = "up"
    /\ pending' = FALSE
    /\ UNCHANGED <<win, windows, fails, sess, begun, ver, cp, pub, half, wroteInCopy>>

\* The server exits after its interrupt and cuts a session in flight: a session committed step by
\* step leaves its changes without its finish.
Exit ==
    /\ srv = "stopping"
    /\ srv' = "down"
    /\ sess' = "none"
    /\ half' = (half \/ (sess = "part" /\ ~AtomicSession))
    /\ UNCHANGED <<win, pending, windows, fails, begun, ver, cp, pub, wroteInCopy>>

SessBegin ==
    /\ sess = "none"
    /\ srv = "up"
    /\ begun < MaxSessions
    /\ sess' = "open"
    /\ begun' = begun + 1
    /\ UNCHANGED <<srv, win, pending, windows, fails, ver, cp, pub, half, wroteInCopy>>

\* The session's changes: committed now only when a session is not one transaction.
SessApply ==
    /\ sess = "open"
    /\ srv \in Running
    /\ sess' = "part"
    /\ ver' = IF AtomicSession THEN ver ELSE ver + 1
    /\ wroteInCopy' = (wroteInCopy \/ (~AtomicSession /\ win \in Copying))
    /\ UNCHANGED <<srv, win, pending, windows, fails, begun, cp, pub, half>>

SessFinish ==
    /\ sess = "part"
    /\ srv \in Running
    /\ sess' = "none"
    /\ ver' = ver + 1
    /\ wroteInCopy' = (wroteInCopy \/ win \in Copying)
    /\ UNCHANGED <<srv, win, pending, windows, fails, begun, cp, pub, half>>

\* The runs end once no window runs, no start is held, the server is not stopping and no session is
\* in flight.
Finished ==
    /\ win \in WinEnds
    /\ ~pending
    /\ srv # "stopping"
    /\ sess = "none"
    /\ UNCHANGED vars

Next ==
    \/ Trigger \/ WindowMoves \/ StartRuns \/ Exit
    \/ SessBegin \/ SessApply \/ SessFinish
    \/ Finished

Spec == Init /\ [][Next]_vars

\* The published generation holds no database read while a server process ran ("the copy runs with
\* the server stopped, and only the copy does", deck-streak-sync-snapshot.service; `backup.py::window`).
CopyOnlyWhileStopped == ~pub.cu /\ ~pub.mu

\* The published collection and media index are of one generation: no write lands between the two
\* copies, and nothing is published until both are taken ("one generation that is published by a
\* rename only when whole", `backup.py`'s docstring; `backup.py::window`).
PairFromOneGeneration == pub.c = pub.m

\* A client's sync in the window is refused whole: no write lands while the window copies, and the
\* server's exit leaves no session's changes without its finish.
ASyncInTheWindowIsRefusedWhole == ~wroteInCopy /\ ~half

\* After every window, the server is up again, a failed copy included, given that the window, the
\* server's exit and PID 1's start each take their step when it stays enabled ("PID 1 starts the
\* server again when it ends, on success and on failure alike", deck-streak-sync-snapshot.service).
TheServerIsUpAfterEveryWindow ==
    (WF_vars(WindowMoves) /\ WF_vars(Exit) /\ WF_vars(StartRuns))
        => ((win \in {"ok", "fail", "skipped"}) ~> (srv = "up"))

=============================================================================
