------------------------------ MODULE SecondRoute ------------------------------
\* @phx covers deploy/scripts/second-route.sh anchor=read_alert_path digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers deploy/scripts/second-route.sh anchor=tell_owner digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers deploy/scripts/second-route.sh anchor=check_in digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers deploy/scripts/second-route.sh anchor=main digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #285
\* @phx property ACheckInCoversOnlyToldFailures ramp=report
\* @phx property NoCheckInAfterAnUnreadableRead ramp=report
\* @phx property EveryFailureIsToldOrItsSilenceIsHeard ramp=report
\* @phx witness witness/a-key-recorded-before-its-report-is-delivered.cfg kills=EveryFailureIsToldOrItsSilenceIsHeard
\* @phx witness witness/a-failure-keyed-by-its-unit-name-alone.cfg kills=ACheckInCoversOnlyToldFailures
\* @phx witness witness/a-check-in-after-an-undelivered-report.cfg kills=ACheckInCoversOnlyToldFailures
\* @phx witness witness/an-unreadable-read-taken-as-nothing-failed.cfg kills=NoCheckInAfterAnUnreadableRead
\*
\* The second route (SPEC-396, ADR-410; #285). deck-streak-second-route.timer runs
\* deploy/scripts/second-route.sh every ten minutes. A run reads the alert path from the service
\* manager, reports the keys it has not yet reported to a receiver off the host, records the keys of
\* its read, and checks in. Meanwhile the service manager fails alert instances, replaces a failed
\* instance with a later one of the same name whose delivered page tells that failure, turns the
\* alert template absent, and answers unreadably. The reported keys persist across runs, so every
\* write to them is an action here (Plan under RecordFirst, Record otherwise).
\*
\* What the model abstracts, and why:
\* - A read is one step: the script lists the failed instances, then asks each one's invocation id,
\*   then the template's state. What the manager changes between those calls it changes between the
\* read and the act as well, which Fail, Replace and the two flips model after every step.
\* - A key is a failed instance's name and invocation id (read_alert_path's `failed <instance>
\*   <invocation id>`), the absent or masked template (`template <state>`, one key for any state),
\*   or `unreadable`. A report names every new key: the 3500 byte bound only splits it into whole
\*   lines and a count line (tell_owner), so the receiver still hears a report of all of them.
\* - Delivery of a request is a free choice per attempt (curl's retries are inside one attempt).
\* - The receiver counts consecutive runs without a delivered check-in, `miss`, saturating at the
\*   grace of two periods; at the grace it hears the check-ins stop, and that silence covers every
\*   failure that has happened by then (heardSilence).
\* - The episode marker (first undelivered run exits 1 at priority 3, later ones exit 0 at priority 4)
\*   changes a run's exit status and the priority of its own failure line and no request, key or
\*   state the receiver acts on, so it is not a variable here; the tests hold it (A7).
\* - `known` is the receiver's knowledge of a failure key: heard in a delivered report, or told by the
\*   delivered page of a later instance of the same name (Replace). A delivered report adds only its
\*   failure keys to it, because no property, guard or witness reads a template or unreadable key
\*   in `known`.
\* - Send empties `pend`, delivered or not, because nothing reads `pend` after Send until the next
\*   Plan writes it again (Finish empties it too), so two states that differ only there are one.
\* - Every configuration has two alert instance names and two invocations under each name.
\* - The timer starts one run at a time: the unit is a oneshot that ends before the timer is due.
\*
\* Mapping to the script:
\* - Start -> main's credential judgement (an empty or non-https address ends the run in its own
\*   failure).
\* - Read -> read_alert_path; Plan, Send, Record -> tell_owner; CheckIn -> check_in; Finish -> the
\*   end of main.
\* - Fail, Replace -> an alert instance failing and the next instance of its name, whose delivered
\*   page tells the earlier failure (deck-streak-alert@.service run by OnFailure=).
\* - FlipTemplate, FlipManager -> the template masked or absent, and systemctl failing or answering
\*   out of shape.
\* - HearStop -> the receiver's grace.

EXTENDS Integers

CONSTANTS RecordFirst, NameAlone, CheckInAfterUndelivered, UnreadableAsEmpty, MaxInv, Names

TKey == <<"t", "-", 0>>
UKey == <<"u", "-", 0>>
FailKeys == {<<"f", n, i>> : n \in Names, i \in 1..MaxInv}
AllKeys == FailKeys \cup {TKey, UKey}
\* How the script remembers a key: whole, or by its unit name alone under the defect.
Norm(k) == IF NameAlone THEN <<k[1], k[2], 0>> ELSE k
Stored == {Norm(k) : k \in AllKeys}
Places == {"idle", "read", "plan", "send", "record", "checkin", "checked", "done"}
Min2(x) == IF x > 2 THEN 2 ELSE x

VARIABLES nextInv, failed, tmplAbsent, mgrBroken, known, everFailed,
          pc, snap, pend, readBroken, snapUnread, reported, heardSilence, miss

vars == <<nextInv, failed, tmplAbsent, mgrBroken, known, everFailed,
          pc, snap, pend, readBroken, snapUnread, reported, heardSilence, miss>>

TypeOK ==
    /\ nextInv \in [Names -> 0..MaxInv]
    /\ failed \in [Names -> 0..MaxInv]
    /\ tmplAbsent \in BOOLEAN
    /\ mgrBroken \in BOOLEAN
    /\ known \subseteq AllKeys
    /\ everFailed \subseteq FailKeys
    /\ pc \in Places
    /\ snap \subseteq AllKeys
    /\ pend \subseteq AllKeys
    /\ readBroken \in BOOLEAN
    /\ snapUnread \in BOOLEAN
    /\ reported \subseteq Stored
    /\ heardSilence \subseteq FailKeys
    /\ miss \in 0..2

Init ==
    /\ nextInv = [n \in Names |-> 0]
    /\ failed = [n \in Names |-> 0]
    /\ tmplAbsent = FALSE
    /\ mgrBroken = FALSE
    /\ known = {}
    /\ everFailed = {}
    /\ pc = "idle"
    /\ snap = {}
    /\ pend = {}
    /\ readBroken = FALSE
    /\ snapUnread = FALSE
    /\ reported = {}
    /\ heardSilence = {}
    /\ miss = 0

RunVars == <<pc, snap, pend, readBroken, snapUnread, reported, miss>>
ManagerVars == <<nextInv, failed, tmplAbsent, mgrBroken, everFailed>>

\* The manager: an alert instance fails under a fresh invocation id.
Fail(n) ==
    /\ failed[n] = 0
    /\ nextInv[n] < MaxInv
    /\ nextInv' = [nextInv EXCEPT ![n] = nextInv[n] + 1]
    /\ failed' = [failed EXCEPT ![n] = nextInv[n] + 1]
    /\ everFailed' = everFailed \cup {<<"f", n, nextInv[n] + 1>>}
    /\ UNCHANGED <<tmplAbsent, mgrBroken, known, RunVars, heardSilence>>

\* A later instance of the same name runs and its delivered page tells the earlier failure.
Replace(n) ==
    /\ failed[n] # 0
    /\ known' = known \cup {<<"f", n, failed[n]>>}
    /\ failed' = [failed EXCEPT ![n] = 0]
    /\ UNCHANGED <<nextInv, tmplAbsent, mgrBroken, everFailed, RunVars, heardSilence>>

FlipTemplate ==
    /\ tmplAbsent' = ~tmplAbsent
    /\ UNCHANGED <<nextInv, failed, mgrBroken, known, everFailed, RunVars, heardSilence>>

FlipManager ==
    /\ mgrBroken' = ~mgrBroken
    /\ UNCHANGED <<nextInv, failed, tmplAbsent, known, everFailed, RunVars, heardSilence>>

\* main: the credentials are judged before anything is read or sent.
Start ==
    /\ pc = "idle"
    /\ \E ok \in BOOLEAN :
        IF ok
        THEN /\ pc' = "read"
             /\ UNCHANGED miss
        ELSE /\ pc' = "idle"
             /\ miss' = Min2(miss + 1)
    /\ UNCHANGED <<ManagerVars, known, snap, pend, readBroken, snapUnread, reported, heardSilence>>

\* read_alert_path: the failed instances with their invocation ids, and the template. A manager that
\* answers unreadably yields the key `unreadable`, and the defect reads it as nothing failed.
Read ==
    /\ pc = "read"
    /\ snap' = IF mgrBroken
                THEN (IF UnreadableAsEmpty THEN {} ELSE {UKey})
                ELSE {<<"f", n, failed[n]>> : n \in {m \in Names : failed[m] # 0}}
                     \cup (IF tmplAbsent THEN {TKey} ELSE {})
    /\ snapUnread' = (mgrBroken /\ ~UnreadableAsEmpty)
    /\ readBroken' = mgrBroken
    /\ pc' = "plan"
    /\ UNCHANGED <<ManagerVars, known, pend, reported, heardSilence, miss>>

\* tell_owner: the keys not yet reported. Under the defect the keys are recorded here, before the
\* report is delivered.
Plan ==
    /\ pc = "plan"
    /\ pend' = {k \in snap : Norm(k) \notin reported}
    /\ reported' = IF RecordFirst THEN {Norm(k) : k \in snap} ELSE reported
    /\ pc' = IF pend' = {} THEN "record" ELSE "send"
    /\ UNCHANGED <<ManagerVars, known, snap, readBroken, snapUnread, heardSilence, miss>>

\* tell_owner: one report request. Undelivered, the run ends in its own failure; the defect still
\* checks in.
Send ==
    /\ pc = "send"
    /\ \E ok \in BOOLEAN :
        IF ok
        THEN /\ known' = known \cup (pend \cap FailKeys)
             /\ pc' = "record"
             /\ UNCHANGED miss
        ELSE /\ pc' = IF CheckInAfterUndelivered THEN "checkin" ELSE "done"
             /\ miss' = Min2(miss + 1)
             /\ UNCHANGED known
    /\ pend' = {}
    /\ UNCHANGED <<ManagerVars, snap, readBroken, snapUnread, reported, heardSilence>>

\* tell_owner: only after the report is delivered are the keys of this read recorded, which drops a
\* key whose instance is gone.
Record ==
    /\ pc = "record"
    /\ reported' = {Norm(k) : k \in snap}
    /\ pc' = "checkin"
    /\ UNCHANGED <<ManagerVars, known, snap, pend, readBroken, snapUnread, heardSilence, miss>>

\* check_in: withheld after an unreadable read, else one request with no body.
CheckIn ==
    /\ pc = "checkin"
    /\ IF snapUnread
       THEN /\ pc' = "done"
            /\ miss' = Min2(miss + 1)
       ELSE \E ok \in BOOLEAN :
              IF ok
              THEN /\ pc' = "checked"
                   /\ miss' = 0
              ELSE /\ pc' = "done"
                   /\ miss' = Min2(miss + 1)
    /\ UNCHANGED <<ManagerVars, known, snap, pend, readBroken, snapUnread, reported, heardSilence>>

Finish ==
    /\ pc \in {"checked", "done"}
    /\ pc' = "idle"
    /\ snap' = {}
    /\ pend' = {}
    /\ readBroken' = FALSE
    /\ snapUnread' = FALSE
    /\ UNCHANGED <<ManagerVars, known, reported, heardSilence, miss>>

\* The receiver hears the check-ins stop past its grace.
HearStop ==
    /\ miss = 2
    /\ ~(everFailed \subseteq heardSilence)
    /\ heardSilence' = heardSilence \cup everFailed
    /\ UNCHANGED <<ManagerVars, known, RunVars>>

RunNext == Start \/ Read \/ Plan \/ Send \/ Record \/ CheckIn \/ Finish

Next ==
    \/ \E n \in Names : Fail(n) \/ Replace(n)
    \/ FlipTemplate
    \/ FlipManager
    \/ RunNext
    \/ HearStop

Spec == Init /\ [][Next]_vars

\* No check-in is sent while a failure key of its run's read is neither delivered nor told by a
\* later page under the same name ("the keys not in $STATE_DIRECTORY/reported are new",
\* second-route.sh::tell_owner; the check-in follows the delivered report, second-route.sh::main).
ACheckInCoversOnlyToldFailures ==
    pc = "checked" => \A k \in snap \cap FailKeys : k \in known

\* No check-in follows an unreadable read (second-route.sh::check_in).
NoCheckInAfterAnUnreadableRead == pc = "checked" => ~readBroken

\* Every failure is eventually told, by a delivered report or a later page of its name, or the
\* receiver hears the check-ins stop, given that the timer's runs and the receiver take their steps
\* when they stay enabled.
EveryFailureIsToldOrItsSilenceIsHeard ==
    (WF_vars(RunNext) /\ WF_vars(HearStop))
        => \A k \in FailKeys :
            (k \in everFailed) ~> (k \in known \cup heardSilence)

=============================================================================
