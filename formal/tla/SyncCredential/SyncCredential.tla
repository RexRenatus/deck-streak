---------------------------- MODULE SyncCredential ----------------------------
\* @phx covers crates/engine-core/src/credential.rs anchor=on_obtained digest=sha256:380742ef98a28ccdf4358d7637ffcf7fd97b59521cbce3a96a7da27c8c9ac07e
\* @phx covers crates/engine-core/src/credential.rs anchor=may_send digest=sha256:bcdc87dd7031d3e43f4c2ee240d65c3cb74a8b2239162d08f4f85c2f9f23d207
\* @phx covers crates/engine-core/src/credential.rs anchor=classify digest=sha256:d40d0054d5cc7f95b96316cb3d3a0dd493f78bfcd813bd0741c80a695f99ca9b
\* @phx covers crates/engine-core/src/credential.rs anchor=on_outcome digest=sha256:7589ecced4b13ca2a28f6af26e59bf09a5bc247bd9e5d622cce0c7c2da5dbcac
\* @phx covers crates/engine-core/src/credential.rs anchor=on_removed digest=sha256:0ddadfdcda9101e86f8878c9a4dee4bdccfc75bb5a63943bd9f1d2eaf201bfcd
\* @phx covers crates/api/src/sync_seal_routes.rs anchor=release digest=sha256:5f8b94533c69ab7823140033b49b58c9ec4c43348e5809f033f3189ff99afa4b
\* @phx cites #654, #631, #618
\* @phx property NoKeyAtRestAfterSignOut ramp=report
\* @phx property NoSendAfterSignOut ramp=report
\* @phx property ARefusalDropsOnlyItsOwnKey ramp=report
\* @phx property AFailureKeepsTheKey ramp=report
\* @phx property UnsealOnlyInALiveSession ramp=report
\* @phx witness witness/a-login-kept-after-a-sign-out.cfg kills=NoKeyAtRestAfterSignOut
\* @phx witness witness/a-send-from-memory-after-a-sign-out.cfg kills=NoSendAfterSignOut
\* @phx witness witness/a-refusal-that-drops-a-newer-key.cfg kills=ARefusalDropsOnlyItsOwnKey
\* @phx witness witness/a-network-failure-that-drops-the-key.cfg kills=AFailureKeepsTheKey
\* @phx witness witness/an-unseal-with-no-session.cfg kills=UnsealOnlyInALiveSession
\*
\* The web client's sync key (SPEC-363, ADR-374). The browser keeps the sync host key as one sealed
\* record in the origin's indexed database, beside a stored generation. The record opens only under
\* a sealing key the service releases to a live owner session, and only in a dedicated Worker. One
\* rule in the core decides when a login is kept, when a key may be sent, which answer is a refusal,
\* when a refusal drops the key, and what a removal does to the generation
\* (`crates/engine-core/src/credential.rs`, #654). Its worth is the order of the steps, so the model
\* comes first: a login in flight racing a sign-out, a refusal of an older key racing a newer login,
\* and a removal racing a Worker that holds the key open (#631, #618).
\*
\* The actors: the owner (a login, a sign-out), two Workers of one origin (an open key, a send, a
\* restart), the sync server (an accepted key, a refusal, a network failure) and the service (a
\* session that opens and ends, the release of the sealing key, a rotation of the seal secret).
\*
\* What the model abstracts, and why (ADR-374 D12):
\* - The cipher, the nonce, the seal id and the HMAC are out of the model: a record opens exactly
\*   when the seal secret's version it was sealed under is the current one. Each of them is held by
\*   a unit test and a mutation row instead.
\* - The record is sealed under the seal secret's version current when the login lands. A rotation
\*   between the release and the landing reaches the same state as a rotation just after it, which
\*   `Rotate` already takes.
\* - The sync server's refusal is free to happen to any send at any time, a refusal after a new
\*   password or a new hash included: the model gives the server no key of its own, so every order
\*   of a refusal against the client's steps is explored.
\* - A sign-out's broadcast to the origin's other Workers is not relied on: no Worker's memory is
\*   cleared by `SignOut`. Each Worker learns of a forget only by re-reading the generation before
\*   a send, which is the rule the model checks.
\* - The web session's own bounds (idle and absolute) are `SessionEnds`; the page's logout request
\*   is `SessionEnds` too, after the forget or never (offline), so `SignOut` touches no session.
\* - `NONE` marks an empty slot: no key open, no login in flight, no send in flight. Generations
\*   are 0 to `MaxGen`, and `MaxGen` stands for the largest generation the code can name.
\*
\* The switches are the chosen design when all five are TRUE: `CheckLoginGeneration` keeps a
\* landed login only at the generation it started at, `CheckSendGeneration` admits a send only from
\* a Worker holding the current generation while a record is stored, `CheckRefusalGeneration` drops
\* the record on a refusal only when the refusal is of the current generation, `DropOnlyOnRefusal`
\* keeps the record on a network failure, and `ReleaseNeedsSession` releases the sealing key only
\* to a live owner session.
\*
\* The action-to-code map, by `file::item`:
\* - StartLogin -> the Worker asks for the release first (`sync_seal_routes.rs::release`, an owner
\*   session or 401) and reads the generation it starts at.
\* - LandLogin -> `credential.rs::on_obtained`: `Store` at the next generation, or `Discard`.
\* - LoginFails -> the engine's login call fails; nothing is stored.
\* - SignOut -> the forget: the record deleted and the generation `credential.rs::on_removed`
\*   names, in one transaction.
\* - Unseal -> `sync_seal_routes.rs::release` for the record's seal id, and the record opened in the
\*   Worker.
\* - DropStale -> a record that does not open under its released key is deleted, and the
\*   generation raised by `credential.rs::on_removed` (ADR-374 D11).
\* - StartSend -> `credential.rs::may_send`; refused, the Worker sends nothing and clears its
\*   memory.
\* - Accepted, Refused, Failed -> `credential.rs::classify` of the sync's answer, then
\*   `credential.rs::on_outcome` and, on a drop, `credential.rs::on_removed`.
\* - Rotate -> the seal secret's rotation on the service.
\* - SessionOpens, SessionEnds -> an owner session opened, and ended by its bounds, a logout or a
\*   passkey's removal.
\* - Restart -> a Worker's memory, login and send cleared, as a closed tab or a reloaded page.
\*
\* The re-read against the code, at the stamp. Each covered item, beside the action it maps to:
\* - `credential.rs::on_obtained` stores at `current.next()` only when `started == current`, and
\*   discards otherwise, a missing next generation included: `LandLogin`'s guard
\*   `login[w] = gen /\ gen < MaxGen`.
\* - `credential.rs::may_send` is `sealed && held == current`: `StartSend`'s guard. A stored record
\*   is at the current generation in every state, since `gen` moves only with `sealed`
\*   (`LandLogin`, `SignOut`, `DropStale`, `Refused` and `Failed`), so the code's flag and the
\*   model's record agree.
\* - `credential.rs::classify` reads no error as accepted, the decoded kind `SYNC_AUTH_ERROR`
\*   alone as refused, and every other answer, an undecodable one included, as failed: the split
\*   of `Accepted`, `Refused` and `Failed`.
\* - `credential.rs::on_outcome` drops only a refusal with `sent == current`: `Refused`'s guard
\*   `inflight[w] = gen`. `Failed` drops nothing.
\* - `credential.rs::on_removed` is `current.next()`, `None` at the maximum: `Removed`, which keeps
\*   `MaxGen` where the code answers `None`, so no login is kept again.
\* - `sync_seal_routes.rs::release` answers 404 with no seal secret, refuses a cross-site request,
\*   answers 401 without an owner session and 429 past its bound, and 400 for a bad seal id, all
\*   before it derives a key: `Unseal`'s `session` guard. A release that is off never unseals, which
\*   is a subset of the model's behaviours.

EXTENDS Integers

CONSTANTS CheckLoginGeneration, CheckSendGeneration, CheckRefusalGeneration,
          DropOnlyOnRefusal, ReleaseNeedsSession, NWorkers, MaxGen, MaxKeys

Workers == 1..NWorkers
Gens == 0..MaxGen
Keys == 1..MaxKeys
NONE == -1
Slot == Gens \cup {NONE}
Records == [g : 1..MaxGen, kv : Keys]
NoRecord == [g |-> NONE, kv |-> 0]

\* `credential.rs::on_removed`: the next generation, and at the largest one none: the record is
\* deleted and the generation stays where no login can be kept again, so the store fails closed.
Removed(g) == IF g < MaxGen THEN g + 1 ELSE g

VARIABLES gen, sealed, keyver, mem, login, inflight, session,
          signedOut, noLoginKept, sentAfterOut, droppedNewer, failDropped, unsealedNoSession

store == <<gen, sealed, keyver>>
workers == <<mem, login, inflight>>
history == <<signedOut, noLoginKept, sentAfterOut, droppedNewer, failDropped, unsealedNoSession>>
vars == <<store, workers, session, history>>

TypeOK ==
    /\ gen \in Gens
    /\ sealed \in Records \cup {NoRecord}
    /\ keyver \in Keys
    /\ mem \in [Workers -> Slot]
    /\ login \in [Workers -> Slot]
    /\ inflight \in [Workers -> Slot]
    /\ session \in BOOLEAN
    /\ signedOut \in BOOLEAN
    /\ noLoginKept \in BOOLEAN
    /\ sentAfterOut \in BOOLEAN
    /\ droppedNewer \in BOOLEAN
    /\ failDropped \in BOOLEAN
    /\ unsealedNoSession \in BOOLEAN

\* A browser that has never obtained a key: no record, generation 0, no Worker holding anything,
\* and a web session that is live or not.
Init ==
    /\ gen = 0
    /\ sealed = NoRecord
    /\ keyver = 1
    /\ mem = [w \in Workers |-> NONE]
    /\ login = [w \in Workers |-> NONE]
    /\ inflight = [w \in Workers |-> NONE]
    /\ session \in BOOLEAN
    /\ signedOut = FALSE
    /\ noLoginKept = FALSE
    /\ sentAfterOut = FALSE
    /\ droppedNewer = FALSE
    /\ failDropped = FALSE
    /\ unsealedNoSession = FALSE

\* The owner starts a login in Worker `w`: the release is asked for first, so a login needs a live
\* owner session, and the login remembers the generation it started at.
StartLogin(w) ==
    /\ session
    /\ login[w] = NONE
    /\ login' = [login EXCEPT ![w] = gen]
    /\ signedOut' = FALSE
    /\ UNCHANGED <<store, mem, inflight, session>>
    /\ UNCHANGED <<noLoginKept, sentAfterOut, droppedNewer, failDropped, unsealedNoSession>>

\* The login lands: `credential.rs::on_obtained` keeps it only when it started at the current
\* generation and a next generation exists; a kept login is sealed at that next generation and the
\* Worker holds it open.
LandLogin(w) ==
    /\ login[w] /= NONE
    /\ login' = [login EXCEPT ![w] = NONE]
    /\ IF (~CheckLoginGeneration \/ login[w] = gen) /\ gen < MaxGen
          THEN /\ gen' = gen + 1
               /\ sealed' = [g |-> gen + 1, kv |-> keyver]
               /\ mem' = [mem EXCEPT ![w] = gen + 1]
               /\ noLoginKept' = FALSE
          ELSE UNCHANGED <<gen, sealed, mem, noLoginKept>>
    /\ UNCHANGED <<keyver, inflight, session>>
    /\ UNCHANGED <<signedOut, sentAfterOut, droppedNewer, failDropped, unsealedNoSession>>

\* The engine's login call fails; nothing is stored.
LoginFails(w) ==
    /\ login[w] /= NONE
    /\ login' = [login EXCEPT ![w] = NONE]
    /\ UNCHANGED <<store, mem, inflight, session, history>>

\* The owner signs out: the forget deletes the record and raises the generation in one
\* transaction, online or not.
SignOut ==
    /\ gen' = Removed(gen)
    /\ sealed' = NoRecord
    /\ signedOut' = TRUE
    /\ noLoginKept' = TRUE
    /\ UNCHANGED <<keyver, workers, session>>
    /\ UNCHANGED <<sentAfterOut, droppedNewer, failDropped, unsealedNoSession>>

\* Worker `w` opens the record: the service releases the sealing key to a live owner session, and
\* the record opens under it when it was sealed under the current seal secret.
Unseal(w) ==
    /\ ~ReleaseNeedsSession \/ session
    /\ sealed /= NoRecord
    /\ sealed.kv = keyver
    /\ mem[w] = NONE
    /\ mem' = [mem EXCEPT ![w] = sealed.g]
    /\ unsealedNoSession' = (unsealedNoSession \/ ~session)
    /\ UNCHANGED <<store, login, inflight, session>>
    /\ UNCHANGED <<signedOut, noLoginKept, sentAfterOut, droppedNewer, failDropped>>

\* A record that does not open under its released key, sealed under a seal secret since rotated,
\* is deleted and the generation raised (ADR-374 D11).
DropStale(w) ==
    /\ ~ReleaseNeedsSession \/ session
    /\ sealed /= NoRecord
    /\ sealed.kv /= keyver
    /\ mem[w] = NONE
    /\ sealed' = NoRecord
    /\ gen' = Removed(gen)
    /\ UNCHANGED <<keyver, workers, session, history>>

\* Worker `w` asks to send the key it holds: `credential.rs::may_send` admits it only while a record
\* is stored at the generation the Worker holds; refused, the Worker sends nothing and clears its
\* memory.
StartSend(w) ==
    /\ mem[w] /= NONE
    /\ inflight[w] = NONE
    /\ IF ~CheckSendGeneration \/ (sealed /= NoRecord /\ mem[w] = gen)
          THEN /\ inflight' = [inflight EXCEPT ![w] = mem[w]]
               /\ sentAfterOut' = (sentAfterOut \/ noLoginKept)
               /\ UNCHANGED mem
          ELSE /\ mem' = [mem EXCEPT ![w] = NONE]
               /\ UNCHANGED <<inflight, sentAfterOut>>
    /\ UNCHANGED <<store, login, session>>
    /\ UNCHANGED <<signedOut, noLoginKept, droppedNewer, failDropped, unsealedNoSession>>

\* The sync server accepts the key: the record is kept.
Accepted(w) ==
    /\ inflight[w] /= NONE
    /\ inflight' = [inflight EXCEPT ![w] = NONE]
    /\ UNCHANGED <<store, mem, login, session, history>>

\* The sync server refuses the key (`SYNC_AUTH_ERROR`): `credential.rs::on_outcome` drops the
\* record only when the refused send was of the current generation, and the Worker forgets the key
\* it sent.
Refused(w) ==
    /\ inflight[w] /= NONE
    /\ inflight' = [inflight EXCEPT ![w] = NONE]
    /\ IF ~CheckRefusalGeneration \/ inflight[w] = gen
          THEN /\ sealed' = NoRecord
               /\ gen' = Removed(gen)
               /\ droppedNewer' = (droppedNewer \/ (sealed /= NoRecord /\ sealed.g > inflight[w]))
          ELSE UNCHANGED <<sealed, gen, droppedNewer>>
    /\ mem' = [mem EXCEPT ![w] = IF mem[w] = inflight[w] THEN NONE ELSE mem[w]]
    /\ UNCHANGED <<keyver, login, session>>
    /\ UNCHANGED <<signedOut, noLoginKept, sentAfterOut, failDropped, unsealedNoSession>>

\* The send fails on the network, or with any answer that is not the server's refusal:
\* `credential.rs::classify` reads it as a failure, which keeps the record.
Failed(w) ==
    /\ inflight[w] /= NONE
    /\ inflight' = [inflight EXCEPT ![w] = NONE]
    /\ IF DropOnlyOnRefusal
          THEN UNCHANGED <<sealed, gen, failDropped>>
          ELSE /\ sealed' = NoRecord
               /\ gen' = Removed(gen)
               /\ failDropped' = (failDropped \/ sealed /= NoRecord)
    /\ UNCHANGED <<keyver, mem, login, session>>
    /\ UNCHANGED <<signedOut, noLoginKept, sentAfterOut, droppedNewer, unsealedNoSession>>

\* The service rotates the seal secret: every record sealed before it stops opening.
Rotate ==
    /\ keyver < MaxKeys
    /\ keyver' = keyver + 1
    /\ UNCHANGED <<gen, sealed, workers, session, history>>

\* An owner session opens, by Telegram or a passkey.
SessionOpens ==
    /\ ~session
    /\ session' = TRUE
    /\ UNCHANGED <<store, workers, history>>

\* The owner session ends: its bounds, a logout, or a passkey's removal. The record stays sealed.
SessionEnds ==
    /\ session
    /\ session' = FALSE
    /\ UNCHANGED <<store, workers, history>>

\* Worker `w` restarts: its open key, its login in flight and its send in flight are gone.
Restart(w) ==
    /\ mem[w] /= NONE \/ login[w] /= NONE \/ inflight[w] /= NONE
    /\ mem' = [mem EXCEPT ![w] = NONE]
    /\ login' = [login EXCEPT ![w] = NONE]
    /\ inflight' = [inflight EXCEPT ![w] = NONE]
    /\ UNCHANGED <<store, session, history>>

Next ==
    \/ \E w \in Workers :
          \/ StartLogin(w)
          \/ LandLogin(w)
          \/ LoginFails(w)
          \/ Unseal(w)
          \/ DropStale(w)
          \/ StartSend(w)
          \/ Accepted(w)
          \/ Refused(w)
          \/ Failed(w)
          \/ Restart(w)
    \/ SignOut
    \/ Rotate
    \/ SessionOpens
    \/ SessionEnds

Spec == Init /\ [][Next]_vars

\* After a sign-out, until the owner starts a new login, no sealed record is stored ("`Store` only
\* when started equals current and a next generation exists, else `Discard`",
\* `credential.rs::on_obtained`; `credential.rs::on_removed`).
NoKeyAtRestAfterSignOut == signedOut => sealed = NoRecord

\* No Worker starts a send after a sign-out until a later login is kept ("a send only when a sealed
\* record is stored and its generation is the one the sender holds", `credential.rs::may_send`).
NoSendAfterSignOut == ~sentAfterOut

\* A refusal never removes a record of a later generation than the one it refused ("drop only on a
\* refusal of the current generation", `credential.rs::on_outcome`; `credential.rs::classify`).
ARefusalDropsOnlyItsOwnKey == ~droppedNewer

\* A network failure never removes the record ("every other answer is a failure that keeps the
\* key", `credential.rs::classify`; `credential.rs::on_outcome`).
AFailureKeepsTheKey == ~failDropped

\* A Worker opens the record only while an owner session is live ("the key is released only to an
\* owner session", `sync_seal_routes.rs::release`).
UnsealOnlyInALiveSession == ~unsealedNoSession
=============================================================================
