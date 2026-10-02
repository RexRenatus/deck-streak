---------------------------- MODULE CaptureOnce ----------------------------
\* @phx covers crates/vault/src/inbox.rs anchor=capture digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/vault/src/capture_store.rs anchor=claim digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/vault/src/atomic.rs anchor=refuse_journal digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #154, #56
\* @phx property AtMostOneStub ramp=report
\* @phx property StubImpliesAttachment ramp=report
\* @phx property SecondWriterWritesNothing ramp=report
\* @phx property NoJournalWrite ramp=report
\* @phx witness witness/a-stub-written-before-its-attachment.cfg kills=StubImpliesAttachment
\* @phx witness witness/a-stem-with-no-unique-claim.cfg kills=AtMostOneStub
\* @phx witness witness/files-written-before-the-claim.cfg kills=SecondWriterWritesNothing
\* @phx witness witness/a-writer-without-the-journal-refusal.cfg kills=NoJournalWrite
\*
\* A capture of one stem (SPEC-118 R3, ADR-118), written by two writers: a Telegram resend and a
\* Mini App retry are each a second writer of a stem the first may already have recorded. Each
\* writer streams its attachment into its own temporary file, then, in ONE `BEGIN IMMEDIATE`
\* transaction, inserts the `inbox_captures` row, renames the attachment into place, writes the
\* stub last, and commits. A writer may crash between any two steps: the transaction rolls back,
\* the lock is released, and the writer may start again, as a retry does.
\*
\* What the model abstracts, and why:
\* - The stem is one stem. Two stems never share a row, a stub or an attachment name.
\* - A writer's attachment name is its own or the other's (`names`, chosen at Init): a resend of
\*   the same document carries the same name, and a renamed one does not.
\* - The stub's own atomic write (its temporary file, sync and rename) is one step: SPEC-042 R2's
\*   writer already lands it whole or not at all.
\* - A crashed writer's temporary file is an ignored name no reader reads; it is not tracked.
\* - The layout either keeps the inbox apart from every journal folder or names the inbox inside
\*   one (`layout`, chosen at Init); the refusal must hold for both.
\* - The curator's filing (SPEC-116) and the erase are not modelled: neither writes a capture.
\*
\* The switches are the fixed design when TRUE and "attach-first": `Order` is the write order
\* inside the transaction, `Claim` the stem's unique row, `ClaimFirst` that the files are written
\* only after the claim, and `JournalRefusal` the atomic writer's refusal of a journal path.

EXTENDS Naturals, FiniteSets

CONSTANTS Order, Claim, ClaimFirst, JournalRefusal, MaxCrash

None == 0
Writers == {1, 2}
Steps == {"attach", "stub"}
Places == {"idle", "streamed", "begun", "claimed", "saved", "dup", "refused"}

VARIABLES layout, names, pc, temp, lock, rows, attached, stubBy, written, answer, journal,
          crashes

vars == <<layout, names, pc, temp, lock, rows, attached, stubBy, written, answer, journal,
          crashes>>

TypeOK ==
    /\ layout \in {"apart", "nested"}
    /\ names \in [Writers -> Writers]
    /\ pc \in [Writers -> Places]
    /\ temp \in [Writers -> BOOLEAN]
    /\ lock \in Writers \cup {None}
    /\ rows \subseteq Writers
    /\ attached \subseteq Writers
    /\ stubBy \in Writers \cup {None}
    /\ written \in [Writers -> SUBSET Steps]
    /\ answer \in [Writers -> Writers \cup {None}]
    /\ journal \in BOOLEAN
    /\ crashes \in 0..MaxCrash

Init ==
    /\ layout \in {"apart", "nested"}
    /\ names \in {[w \in Writers |-> 1], [w \in Writers |-> w]}
    /\ pc = [w \in Writers |-> "idle"]
    /\ temp = [w \in Writers |-> FALSE]
    /\ lock = None
    /\ rows = {}
    /\ attached = {}
    /\ stubBy = None
    /\ written = [w \in Writers |-> {}]
    /\ answer = [w \in Writers |-> None]
    /\ journal = FALSE
    /\ crashes = 0

\* `atomic.rs::refuse_journal`: a write whose target lies under a journal folder is refused before
\* any file is created, so a refused capture writes nothing.
Refused == layout = "nested" /\ JournalRefusal

\* A file the writer creates lands under a journal folder exactly when the layout nests the inbox.
Touch == journal' = (journal \/ layout = "nested")

\* The attachment's bytes stream into the writer's own temporary file beside its target.
Stream(w) ==
    /\ pc[w] = "idle"
    /\ written' = [written EXCEPT ![w] = {}]
    /\ answer' = [answer EXCEPT ![w] = None]
    /\ IF Refused
          THEN /\ pc' = [pc EXCEPT ![w] = "refused"]
               /\ UNCHANGED <<temp, journal>>
          ELSE /\ pc' = [pc EXCEPT ![w] = "streamed"]
               /\ temp' = [temp EXCEPT ![w] = TRUE]
               /\ Touch
    /\ UNCHANGED <<layout, names, lock, rows, attached, stubBy, crashes>>

\* The writer may write its files: after its claim in the chosen design, before it otherwise.
MayWrite(w) == IF ClaimFirst THEN pc[w] = "claimed" ELSE pc[w] = "streamed"

\* The attachment's rename into place. In the chosen order it comes first.
Rename(w) ==
    /\ MayWrite(w)
    /\ temp[w]
    /\ "attach" \notin written[w]
    /\ Order = "attach-first" \/ "stub" \in written[w]
    /\ temp' = [temp EXCEPT ![w] = FALSE]
    /\ attached' = attached \cup {names[w]}
    /\ written' = [written EXCEPT ![w] = @ \cup {"attach"}]
    /\ Touch
    /\ UNCHANGED <<layout, names, pc, lock, rows, stubBy, answer, crashes>>

\* The stub, naming the writer's attachment. In the chosen order it is written last.
WriteStub(w) ==
    /\ MayWrite(w)
    /\ "stub" \notin written[w]
    /\ Order = "stub-first" \/ "attach" \in written[w]
    /\ stubBy' = w
    /\ written' = [written EXCEPT ![w] = @ \cup {"stub"}]
    /\ Touch
    /\ UNCHANGED <<layout, names, pc, temp, lock, rows, attached, answer, crashes>>

\* `BEGIN IMMEDIATE`: the one write lock on the ledger.
Begin(w) ==
    /\ pc[w] = "streamed"
    /\ lock = None
    /\ ClaimFirst \/ written[w] = Steps
    /\ lock' = w
    /\ pc' = [pc EXCEPT ![w] = "begun"]
    /\ UNCHANGED <<layout, names, temp, rows, attached, stubBy, written, answer, journal,
                   crashes>>

\* `capture_store.rs::claim`: the row's insert. A stem already recorded refuses: the transaction
\* rolls back, the writer's temporary file is removed, and it answers the recorded name.
Insert(w) ==
    /\ pc[w] = "begun"
    /\ IF Claim /\ rows # {}
          THEN /\ pc' = [pc EXCEPT ![w] = "dup"]
               /\ lock' = None
               /\ temp' = [temp EXCEPT ![w] = FALSE]
               /\ answer' = [answer EXCEPT ![w] = names[CHOOSE r \in rows : TRUE]]
          ELSE /\ pc' = [pc EXCEPT ![w] = "claimed"]
               /\ UNCHANGED <<lock, temp, answer>>
    /\ UNCHANGED <<layout, names, rows, attached, stubBy, written, journal, crashes>>

\* The commit, after both files: the row is recorded and the writer answers its own name.
Commit(w) ==
    /\ pc[w] = "claimed"
    /\ written[w] = Steps
    /\ rows' = rows \cup {w}
    /\ lock' = None
    /\ pc' = [pc EXCEPT ![w] = "saved"]
    /\ answer' = [answer EXCEPT ![w] = names[w]]
    /\ UNCHANGED <<layout, names, temp, attached, stubBy, written, journal, crashes>>

\* A crash between any two steps: the open transaction rolls back and its lock is released; the
\* files already renamed or written stay; the writer may start again.
Crash(w) ==
    /\ pc[w] \in {"streamed", "begun", "claimed"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![w] = "idle"]
    /\ temp' = [temp EXCEPT ![w] = FALSE]
    /\ lock' = IF lock = w THEN None ELSE lock
    /\ UNCHANGED <<layout, names, rows, attached, stubBy, written, answer, journal>>

\* Every writer has answered or been refused: the runs end here.
Quiescent == \A w \in Writers : pc[w] \in {"saved", "dup", "refused"}

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ \E w \in Writers :
          Stream(w) \/ Rename(w) \/ WriteStub(w) \/ Begin(w) \/ Insert(w) \/ Commit(w)
          \/ Crash(w)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* At most one stub per stem: the stem holds at most one recorded capture, and the stub on disk is
\* that capture's, so no later writer replaces it ("a capture is written once on the host", R3).
AtMostOneStub == Cardinality(rows) <= 1 /\ (rows # {} => stubBy \in rows)

\* A stub on disk implies its attachment on disk: the curator never sees a stub before its file
\* ("the attachment lands before its stub", A2; `inbox.rs::capture`).
StubImpliesAttachment == stubBy # None => names[stubBy] \in attached

\* A second writer of a recorded stem writes nothing and answers the existing name
\* ("a capture sent twice is written once", A4; `capture_store.rs::claim`).
SecondWriterWritesNothing ==
    \A w \in Writers :
        pc[w] = "dup" =>
            /\ written[w] = {}
            /\ ~temp[w]
            /\ answer[w] \in {names[r] : r \in rows}

\* Nothing is written under a journal folder ("no code path writes the journal", #56, A5).
NoJournalWrite == ~journal
=============================================================================
