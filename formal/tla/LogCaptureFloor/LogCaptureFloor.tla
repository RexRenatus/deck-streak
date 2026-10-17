------------------------------ MODULE LogCaptureFloor ------------------------------
\* @phx covers tools/log-capture/capture.rs anchor=register_floor digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers tools/log-capture/capture.rs anchor=with_capture digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers tools/log-capture/capture.rs anchor=hold_capture digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers tools/log-capture/capture.rs anchor=refuse_nested_capture digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers tools/log-capture/capture.rs anchor=register_callsite digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers tools/log-capture/capture.rs anchor=max_level_hint digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #499, #461
\* @phx property NoCachedNeverWhileACaptureIsLive ramp=report
\* @phx property ALineUnderACaptureReachesIt ramp=report
\* @phx property NoInterestIsAskedBeforeTheFloor ramp=report
\* @phx witness witness/no-cached-never-while-a-capture-is-live.cfg kills=NoCachedNeverWhileACaptureIsLive
\* @phx witness witness/a-line-under-a-capture-reaches-it.cfg kills=ALineUnderACaptureReachesIt
\* @phx witness witness/no-interest-is-asked-before-the-floor.cfg kills=NoInterestIsAskedBeforeTheFloor

\* The test log capture's floor against one callsite's cached interest, over every interleaving
\* of two threads of one test binary: the bystanders, which hold no capture, and the capturer,
\* which captures Captures times in turn through tools/log-capture/capture.rs. The shared state is
\* the locked tracing-core's dispatcher registry, its has_just_one switch, the global level filter,
\* the global default and the callsite's cached interest and registration state.
\*
\* What is abstracted:
\* - the level filter is two values: closed ("off"), or open to the line's level ("on");
\* - a capture is a subscriber whose hint admits its line and whose answer is "sometimes" or
\*   "always" (CaptureAnswers);
\* - the capture's count HELD is outside the model: it decides nesting, which the refusal
\*   guards (capture.rs::refuse_nested_capture), and the capturer here never nests;
\* - a dispatcher's registration is one step under its write lock: the push, the switch's store,
\*   the rebuild of every listed callsite and the level's store (callsite.rs::register_dispatch);
\* - a Read registration is one step under its read lock: the choice, the fold and the store;
\* - memory is sequentially consistent: the relaxed loads of the level filter and of the cached
\*   interest are outside the model (SPEC-475 section 6);
\* - DirectRegister stands for a registration that skips the level gate and takes the
\*   registration path's steps.
\*
\* Switches, each TRUE in MCLogCaptureFloor.cfg, which models the code as it is: Floor (the helper
\* installs the floor before a capture and refuses a capture where the floor is not the default),
\* FloorHintsOff (the floor's level hint is OFF) and CensusHolds (nothing outside the helper
\* rebuilds the interest cache or registers the callsite directly).

EXTENDS Naturals

CONSTANTS Bystanders, Captures, Emits, Floor, FloorHintsOff, CensusHolds

Capturer == "capturer"
Threads == Bystanders \cup {Capturer}
Interests == {"none", "never", "sometimes", "always"}
Answers == {"none", "never", "sometimes", "always"}
CaptureAnswers == {"sometimes", "always"}
Steps == {"none", "choose", "ask", "store", "load"}
Places == {"idle", "floor", "global", "register", "swap", "held"}

VARIABLES interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
          capDefault, place, captures, step, ans, emitted, lost, reached,
          askedBeforeTheFloor, rebuilt

vars == <<interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
          capDefault, place, captures, step, ans, emitted, lost, reached,
          askedBeforeTheFloor, rebuilt>>

\* subscriber.rs::Interest::and: an equal pair keeps its value, an unequal one is "sometimes".
And(x, y) == IF x = y THEN x ELSE "sometimes"
\* callsite.rs::rebuild_callsite_interest on a locked rebuilder: the fold of every live
\* registrar's answer, the floor's "sometimes" and the capture's; with none live it is "never".
Fold(fl, cl, ca) == IF fl /\ cl THEN And("sometimes", ca)
                    ELSE IF fl THEN "sometimes"
                    ELSE IF cl THEN ca
                    ELSE "never"
\* capture.rs::max_level_hint, the floor's hint.
FloorHint == IF FloorHintsOff THEN "off" ELSE "on"
\* callsite.rs::Callsites::rebuild_interest: the largest hint of the live registrars.
LevelOf(fl, cl) == IF (fl /\ FloorHint = "on") \/ cl THEN "on" ELSE "off"
Live(fl, cl) == (IF fl THEN 1 ELSE 0) + (IF cl THEN 1 ELSE 0)
Holds(t) == t = Capturer /\ capDefault = "capture"
\* dispatcher.rs::get_default: the thread's scoped default, else the global default, which
\* reads as none before dispatcher.rs::set_global_default stores it.
DefaultOf(t) == IF Holds(t) THEN "capture" ELSE IF global THEN "floor" ELSE "none"
\* Each default's answer: the capture's, the floor's (capture.rs::register_callsite), and none's
\* "never" (subscriber.rs::NoSubscriber).
AnswerOf(d) == CASE d = "capture" -> capAns
                 [] d = "floor" -> "sometimes"
                 [] d = "none" -> "never"
\* Each default's level hint; none gives no hint, which a rebuild counts as TRACE.
HintOf(d) == CASE d = "capture" -> "on"
               [] d = "floor" -> FloorHint
               [] d = "none" -> "on"

TypeOK ==
    /\ interest \in Interests
    /\ reg \in {"unregistered", "registering", "registered"}
    /\ listed \in BOOLEAN
    /\ floorLive \in BOOLEAN
    /\ global \in BOOLEAN
    /\ capLive \in BOOLEAN
    /\ capAns \in CaptureAnswers
    /\ justOne \in BOOLEAN
    /\ level \in {"off", "on"}
    /\ capDefault \in {"none", "capture"}
    /\ place \in Places
    /\ captures \in 0..Captures
    /\ step \in [Threads -> Steps]
    /\ ans \in [Threads -> Answers]
    /\ emitted \in [Threads -> 0..Emits]
    /\ lost \in BOOLEAN
    /\ reached \in BOOLEAN
    /\ askedBeforeTheFloor \in BOOLEAN
    /\ rebuilt \in BOOLEAN

Init ==
    /\ interest = "none" /\ reg = "unregistered" /\ listed = FALSE
    /\ floorLive = FALSE /\ global = FALSE /\ capLive = FALSE /\ capAns = "always"
    /\ justOne = TRUE /\ level = "off" /\ capDefault = "none" /\ place = "idle"
    /\ captures = 0
    /\ step = [t \in Threads |-> "none"]
    /\ ans = [t \in Threads |-> "none"]
    /\ emitted = [t \in Threads |-> 0]
    /\ lost = FALSE /\ reached = FALSE /\ askedBeforeTheFloor = FALSE /\ rebuilt = FALSE

\* The line's fate: an interest of "never" drops it, any other reaches the thread's default
\* (callsite.rs::DefaultCallsite::interest, then the current default's enabled), which on the
\* capturer holding a capture is the capture.
Deliver(t, i) ==
    /\ lost' = (lost \/ (Holds(t) /\ i = "never"))
    /\ reached' = (reached \/ (Holds(t) /\ i # "never"))

\* A macro's emission: the level gate first, then the cached read
\* (callsite.rs::DefaultCallsite::interest), the registering arm that answers "sometimes" and
\* caches nothing, or the start of a first registration, its compare-and-swap and its push onto
\* the callsite list (callsite.rs::DefaultCallsite::register). The capturer emits between its
\* captures or while it holds one.
Emit(t) ==
    /\ step[t] = "none"
    /\ emitted[t] < Emits
    /\ (t = Capturer) => (place \in {"idle", "held"})
    /\ emitted' = [emitted EXCEPT ![t] = @ + 1]
    /\ IF level = "off"
         THEN (lost' = (lost \/ Holds(t)) /\ UNCHANGED <<reached, interest, reg, listed, step>>)
       ELSE IF interest # "none"
         THEN (Deliver(t, interest) /\ UNCHANGED <<interest, reg, listed, step>>)
       ELSE IF reg = "registering"
         THEN (Deliver(t, "sometimes") /\ UNCHANGED <<interest, reg, listed, step>>)
       ELSE (/\ reg' = "registering" /\ listed' = TRUE
             /\ step' = [step EXCEPT ![t] = "choose"]
             /\ UNCHANGED <<interest, lost, reached>>)
    /\ UNCHANGED <<floorLive, global, capLive, capAns, justOne, level, capDefault, place,
                   captures, ans, askedBeforeTheFloor, rebuilt>>

\* The rebuilder's choice (callsite.rs::Dispatchers::rebuilder): JustOne goes on, with no lock,
\* to the ask; Read holds the read lock through one fold of every live registrar's answer and its
\* store (callsite.rs::rebuild_callsite_interest, subscriber.rs::Interest::and), and the
\* registered mark (callsite.rs::DefaultCallsite::register).
Choose(t) ==
    /\ step[t] = "choose"
    /\ IF justOne
         THEN (step' = [step EXCEPT ![t] = "ask"] /\ UNCHANGED <<interest, reg>>)
         ELSE (/\ interest' = Fold(floorLive, capLive, capAns) /\ reg' = "registered"
               /\ step' = [step EXCEPT ![t] = "load"])
    /\ UNCHANGED <<listed, floorLive, global, capLive, capAns, justOne, level, capDefault,
                   place, captures, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* JustOne's lock-free ask of the reaching thread's own default
\* (callsite.rs::rebuild_callsite_interest through dispatcher.rs::get_default).
Ask(t) ==
    /\ step[t] = "ask"
    /\ ans' = [ans EXCEPT ![t] = AnswerOf(DefaultOf(t))]
    /\ askedBeforeTheFloor' = (askedBeforeTheFloor \/ ~global)
    /\ step' = [step EXCEPT ![t] = "store"]
    /\ UNCHANGED <<interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
                   capDefault, place, captures, emitted, lost, reached, rebuilt>>

\* The lock-free store of that answer (callsite.rs::rebuild_callsite_interest) and the
\* registered mark (callsite.rs::DefaultCallsite::register).
Store(t) ==
    /\ step[t] = "store"
    /\ interest' = ans[t] /\ reg' = "registered"
    /\ step' = [step EXCEPT ![t] = "load"]
    /\ UNCHANGED <<listed, floorLive, global, capLive, capAns, justOne, level, capDefault,
                   place, captures, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* The emission's load of the cached value that it then obeys
\* (callsite.rs::DefaultCallsite::register).
Load(t) ==
    /\ step[t] = "load"
    /\ Deliver(t, interest)
    /\ step' = [step EXCEPT ![t] = "none"]
    /\ UNCHANGED <<interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
                   capDefault, place, captures, ans, emitted, askedBeforeTheFloor, rebuilt>>

\* A capture begins (capture.rs::with_capture, capture.rs::hold_capture): with Floor,
\* capture.rs::register_floor runs its body once, behind its OnceLock, so only the first capture
\* installs the floor.
StartCapture ==
    /\ place = "idle" /\ captures < Captures /\ step[Capturer] = "none"
    /\ captures' = captures + 1
    /\ place' = IF Floor /\ ~floorLive THEN "floor" ELSE "register"
    /\ UNCHANGED <<interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
                   capDefault, step, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* capture.rs::register_floor's Dispatch::new(Floor), which registers the floor under the write
\* lock (callsite.rs::register_dispatch): the push, has_just_one's store, the rebuild of every
\* listed callsite (callsite.rs::rebuild_callsite_interest) and the level's store.
InstallFloor ==
    /\ place = "floor"
    /\ floorLive' = TRUE
    /\ justOne' = (Live(TRUE, capLive) <= 1)
    /\ interest' = IF listed THEN Fold(TRUE, capLive, capAns) ELSE interest
    /\ level' = LevelOf(TRUE, capLive)
    /\ place' = "global"
    /\ UNCHANGED <<reg, listed, global, capLive, capAns, capDefault, captures, step, ans,
                   emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* capture.rs::register_floor's dispatcher.rs::set_global_default: the floor becomes the global
\* default and is leaked, so its registrar stays live.
SetGlobal ==
    /\ place = "global"
    /\ global' = TRUE /\ place' = "register"
    /\ UNCHANGED <<interest, reg, listed, floorLive, capLive, capAns, justOne, level,
                   capDefault, captures, step, ans, emitted, lost, reached,
                   askedBeforeTheFloor, rebuilt>>

\* capture.rs::refuse_nested_capture, whose second arm refuses a thread whose default is not the
\* floor, then the capture's Dispatch::new inside tracing's subscriber::with_default or
\* subscriber::set_default: one write-locked registration (callsite.rs::register_dispatch) with
\* its rebuild (callsite.rs::rebuild_callsite_interest), its switch and its level.
RegisterCapture ==
    /\ place = "register"
    /\ Floor => global
    /\ capDefault = "none"
    /\ \E a \in CaptureAnswers :
         /\ capAns' = a
         /\ interest' = IF listed THEN Fold(floorLive, TRUE, a) ELSE interest
    /\ capLive' = TRUE
    /\ justOne' = (Live(floorLive, TRUE) <= 1)
    /\ level' = LevelOf(floorLive, TRUE)
    /\ place' = "swap"
    /\ UNCHANGED <<reg, listed, floorLive, global, capDefault, captures, step, ans, emitted,
                   lost, reached, askedBeforeTheFloor, rebuilt>>

\* The capture becomes the thread's scoped default (dispatcher.rs::with_default,
\* dispatcher.rs::set_default).
Swap ==
    /\ place = "swap"
    /\ capDefault' = "capture" /\ place' = "held"
    /\ UNCHANGED <<interest, reg, listed, floorLive, global, capLive, capAns, justOne, level,
                   captures, step, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* The capture's guard drops (capture.rs::CaptureGuard, dispatcher.rs::DefaultGuard): the
\* default returns and the capture's dispatcher is dropped, so its registrar no longer upgrades;
\* a drop recomputes neither has_just_one, the level nor the cache.
EndCapture ==
    /\ place = "held" /\ step[Capturer] = "none"
    /\ capDefault' = "none" /\ capLive' = FALSE /\ place' = "idle"
    /\ UNCHANGED <<interest, reg, listed, floorLive, global, capAns, justOne, level, captures,
                   step, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* The public callsite.rs::rebuild_interest_cache, a name the census refuses outside the helper:
\* on JustOne it asks the reaching thread's default (dispatcher.rs::get_default) for its hint and
\* its answer; on Read it folds every live registrar.
Rebuild(t) ==
    /\ ~CensusHolds /\ step[t] = "none" /\ ~rebuilt
    /\ rebuilt' = TRUE
    /\ IF justOne
         THEN (/\ level' = HintOf(DefaultOf(t))
               /\ interest' = IF listed THEN AnswerOf(DefaultOf(t)) ELSE interest
               /\ askedBeforeTheFloor' = (askedBeforeTheFloor \/ (listed /\ ~global)))
         ELSE (/\ level' = LevelOf(floorLive, capLive)
               /\ interest' = IF listed THEN Fold(floorLive, capLive, capAns) ELSE interest
               /\ UNCHANGED askedBeforeTheFloor)
    /\ UNCHANGED <<reg, listed, floorLive, global, capLive, capAns, justOne, capDefault, place,
                   captures, step, ans, emitted, lost, reached>>

\* A registration that skips the level gate (the public callsite.rs::register, a name the census
\* refuses outside the helper), taking the registration path's steps.
DirectRegister(t) ==
    /\ ~CensusHolds /\ step[t] = "none" /\ reg = "unregistered"
    /\ reg' = "registering" /\ listed' = TRUE
    /\ step' = [step EXCEPT ![t] = "choose"]
    /\ UNCHANGED <<interest, floorLive, global, capLive, capAns, justOne, level, capDefault,
                   place, captures, ans, emitted, lost, reached, askedBeforeTheFloor, rebuilt>>

\* The runs' ends: every emission made, every capture ended.
Finished ==
    /\ \A t \in Threads : step[t] = "none" /\ emitted[t] = Emits
    /\ place = "idle" /\ captures = Captures
    /\ UNCHANGED vars

Next ==
    \/ \E t \in Threads : Emit(t) \/ Choose(t) \/ Ask(t) \/ Store(t) \/ Load(t)
    \/ StartCapture \/ InstallFloor \/ SetGlobal \/ RegisterCapture \/ Swap \/ EndCapture
    \/ \E t \in Bystanders : Rebuild(t) \/ DirectRegister(t)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* #499: once the floor is installed, no callsite's cached interest is "never" while a capture
\* may be live; with Floor TRUE no capture registers before the floor (capture.rs::with_capture,
\* capture.rs::hold_capture).
NoCachedNeverWhileACaptureIsLive == capLive => interest # "never"

\* #499's consequence: a line emitted on a thread that holds a capture reaches that capture.
ALineUnderACaptureReachesIt == ~lost

\* capture.rs::Floor's doc: an answer computed once the floor is installed is never "never".
NoInterestIsAskedBeforeTheFloor == ~askedBeforeTheFloor
=============================================================================
