------------------------ MODULE MinimumClientHandshake ------------------------
\* @phx covers crates/engine-core/src/handshake.rs anchor=decide digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=handshake digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=run digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=full_sync digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=private digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ffi/src/engine.rs anchor=run digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #671
\* @phx property EverySyncFollowsAnAdmittingRead ramp=report
\* @phx witness witness/EverySyncWaitsForARead.cfg kills=EverySyncFollowsAnAdmittingRead
\* @phx witness witness/EverySyncWaitsForAnAdmittingRead.cfg kills=EverySyncFollowsAnAdmittingRead
\* @phx witness witness/APrivateEngineWaitsForItsParentsRead.cfg kills=EverySyncFollowsAnAdmittingRead
\*
\* The minimum-client handshake (SPEC-374, ADR-385). The sync service states the oldest client
\* level it accepts, and may raise it at any time. A client reads the statement before a sync login
\* and hands it to the core's dispatcher, which keeps one outcome and refuses every sync call until
\* the latest statement admits the client's level. A one-way sync's fetch runs on a private engine
\* the dispatcher starts, which must obey the same outcome as the write that asked for it (#671).
\*
\* What the model abstracts, and why:
\* - `minimum` is the service's stated minimum; Raise moves it up by one, to a bound. The service
\*   never lowers it in this model: a lowered minimum only admits more, so it adds no unsafe step.
\* - `outcome` is the dispatcher's one cell, and `read` the minimum of the statement that set it,
\*   or 0 when no statement was read. Read and ReadFails each replace both, at once, as `handshake`
\*   replaces the cell under its lock. An undecodable statement refuses exactly as an unread one,
\*   so ReadFails stands for both.
\* - Each engine is the parent dispatcher or a private engine. `pc` is whether a private engine
\*   has started; the parent always has. A private engine shares the parent's cell, unless the
\*   `PrivateStartsAdmitted` defect gives it a cell of its own that begins admitted and that no
\*   read ever sets.
\* - `syncs` is the history of every sync call that reached the engine: which engine, and the
\*   minimum of the read its outcome came from (0 for none). A call the core refused is not in it.
\* - The sync login, the normal sync and the one-way sync are one Sync action: the core's rule is
\*   the same check before the engine on each.
\*
\* The switches are the chosen design when `Gate` is TRUE and both others FALSE: `Gate` is the
\* check before the engine (`dispatch.rs::run`, `dispatch.rs::full_sync`), `Flip` turns the
\* comparison in `handshake.rs::decide` over, and `PrivateStartsAdmitted` gives a private engine
\* its own admitted cell instead of the parent's (`dispatch.rs::private`).
\*
\* The action-to-code map, by `file::item`:
\* - Raise -> the service's minimum, raised by a later release.
\* - Read -> `engine.rs::run` reads the statement, `dispatch.rs::handshake` keeps its outcome as
\*   `handshake.rs::decide` decides it.
\* - ReadFails -> the same, when nothing answered or the body does not decode.
\* - StartPrivate(e) -> `dispatch.rs::private`.
\* - Sync(e) -> `dispatch.rs::run` for the sync login and the normal sync, and
\*   `dispatch.rs::full_sync` for the one-way sync and a private engine's fetch.

EXTENDS Integers, FiniteSets

CONSTANTS Level, MaxMinimum, NPrivate, Gate, Flip, PrivateStartsAdmitted

ASSUME Level \in Nat \ {0} /\ MaxMinimum \in Nat /\ MaxMinimum > Level /\ NPrivate \in Nat
ASSUME Gate \in BOOLEAN /\ Flip \in BOOLEAN /\ PrivateStartsAdmitted \in BOOLEAN

Parent == 0
Engines == 0..NPrivate
Privates == 1..NPrivate
Outcomes == {"admitted", "below", "unread"}

VARIABLES minimum, outcome, read, pc, own, syncs

vars == <<minimum, outcome, read, pc, own, syncs>>

TypeOK ==
    /\ minimum \in 1..MaxMinimum
    /\ outcome \in Outcomes
    /\ read \in 0..MaxMinimum
    /\ pc \in [Engines -> {"absent", "started"}]
    /\ own \in [Engines -> BOOLEAN]
    /\ syncs \subseteq [engine : Engines, read : 0..MaxMinimum]

Init ==
    /\ minimum = 1
    /\ outcome = "unread"
    /\ read = 0
    /\ pc = [e \in Engines |-> IF e = Parent THEN "started" ELSE "absent"]
    /\ own = [e \in Engines |-> FALSE]
    /\ syncs = {}

\* The outcome `handshake.rs::decide` gives a statement naming `m`.
Decided(m) ==
    IF (m <= Level) # Flip THEN "admitted" ELSE "below"

\* The outcome engine `e` reads, and the read it came from.
View(e) == IF own[e] THEN "admitted" ELSE outcome
ReadOf(e) == IF own[e] THEN 0 ELSE read

Raise ==
    /\ minimum < MaxMinimum
    /\ minimum' = minimum + 1
    /\ UNCHANGED <<outcome, read, pc, own, syncs>>

Read ==
    /\ outcome' = Decided(minimum)
    /\ read' = minimum
    /\ UNCHANGED <<minimum, pc, own, syncs>>

ReadFails ==
    /\ outcome' = "unread"
    /\ read' = 0
    /\ UNCHANGED <<minimum, pc, own, syncs>>

StartPrivate(e) ==
    /\ pc[e] = "absent"
    /\ pc' = [pc EXCEPT ![e] = "started"]
    /\ own' = [own EXCEPT ![e] = PrivateStartsAdmitted]
    /\ UNCHANGED <<minimum, outcome, read, syncs>>

Sync(e) ==
    /\ pc[e] = "started"
    /\ Gate => View(e) = "admitted"
    /\ syncs' = syncs \cup {[engine |-> e, read |-> ReadOf(e)]}
    /\ UNCHANGED <<minimum, outcome, read, pc, own>>

Next ==
    \/ Raise
    \/ Read
    \/ ReadFails
    \/ \E e \in Privates : StartPrivate(e)
    \/ \E e \in Engines : Sync(e)

Spec == Init /\ [][Next]_vars

\* Every sync call that reached the engine acted on a read, and that read admitted the client's
\* level.
EverySyncFollowsAnAdmittingRead ==
    \A s \in syncs : s.read \in 1..Level

=============================================================================
