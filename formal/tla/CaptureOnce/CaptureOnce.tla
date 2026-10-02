---------------------------- MODULE CaptureOnce ----------------------------
\* @phx covers crates/vault/src/inbox.rs anchor=capture digest=sha256:70b88eacbae9b72793a8bf9c00063f814c364a20816b3bd772789e1ecef36ab6
\* @phx covers crates/vault/src/inbox.rs anchor=attachment_name digest=sha256:c294956d83218d98cb45d6696b6efaba8b6a9047d609256e310998210e3a7d03
\* @phx covers crates/vault/src/capture_store.rs anchor=claim digest=sha256:8fdefc66709482e026039a94da6aa39ec425b5fc8d57f94812716b383c4fd7f8
\* @phx covers crates/vault/src/atomic.rs anchor=refuse_journal digest=sha256:e1c58aa78212bd07fc762d50f219012d2cde41d0103ec82e259ed888ed398f06
\* @phx cites #154, #56
\* @phx property AtMostOneStub ramp=report
\* @phx property StubImpliesAttachment ramp=report
\* @phx property SecondWriterWritesNothing ramp=report
\* @phx property NoJournalWrite ramp=report
\* @phx property StubNeverNamesTheAttachment ramp=report
\* @phx property OneCapturePerKey ramp=report
\* @phx witness witness/a-stub-written-before-its-attachment.cfg kills=StubImpliesAttachment
\* @phx witness witness/a-stem-with-no-unique-claim.cfg kills=AtMostOneStub
\* @phx witness witness/files-written-before-the-claim.cfg kills=SecondWriterWritesNothing
\* @phx witness witness/a-writer-without-the-journal-refusal.cfg kills=NoJournalWrite
\* @phx witness witness/an-attachment-named-as-its-stub.cfg kills=StubNeverNamesTheAttachment
\* @phx witness witness/a-stub-that-replaces-its-attachment.cfg kills=StubImpliesAttachment
\* @phx witness witness/a-miniapp-key-with-no-unique-claim.cfg kills=OneCapturePerKey
\*
\* A capture of one unique (SPEC-118 R3, ADR-118), written by two writers: a Telegram resend and a
\* Mini App retry are each a second writer of a capture the first may already have recorded. Each
\* writer streams its attachment into its own temporary file, then, in ONE `BEGIN IMMEDIATE`
\* transaction, inserts the `inbox_captures` row, renames the attachment into place, writes the
\* stub last, and commits. A writer may crash between any two steps: the transaction rolls back,
\* the lock is released, and the writer may start again, as a retry does.
\*
\* What the model abstracts, and why:
\* - Both writers carry one unique. The first writes under the stem of the first UTC day; the
\*   second writes under that stem or, sent after UTC midnight, under a later day's stem (`day`,
\*   chosen at Init). Two stems never share a stub or an attachment name, so every file is a pair of
\*   a stem and a name; they share only the ledger's lock and its unique keys.
\* - Both writers come from one source (`source`, chosen at Init): a Telegram capture's kind is a
\*   photo, voice note or document and a Mini App capture's is text or journal, so the two never
\*   share a stem, and the Mini App's key index holds no Telegram row (ADR-118's capture-key
\*   amendment). A Mini App capture writes no attachment; the model gives it one, which only adds
\*   behaviours.
\* - A writer's attachment name is its own or the other's (`names`, chosen at Init): a resend of
\*   the same document carries the same name, and a renamed one does not.
\* - The stub's own atomic write (its temporary file, sync and rename) is one step: SPEC-042 R2's
\*   writer already lands it whole or not at all.
\* - A crashed writer's temporary file is an ignored name no reader reads; it is not tracked.
\* - A writer's attachment carries the extension `md` or another (`ext`, chosen at Init). The stub's
\*   own file is `<stem>.md`, so an attachment named `<stem>.md` would be the stub's file: whichever
\*   is written second replaces the other. File names are numbers, the stub's `StubFile` and the
\*   attachments' `names`, so TLC never compares a number with a string.
\* - The layout either keeps the inbox apart from every journal folder or names the inbox inside
\*   one (`layout`, chosen at Init); the refusal must hold for both.
\* - The curator's filing (SPEC-116) and the erase are not modelled: neither writes a capture.
\*
\* The switches are the fixed design when TRUE and "attach-first": `Order` is the write order
\* inside the transaction, `Claim` the stem's unique row, `ClaimFirst` that the files are written
\* only after the claim, `JournalRefusal` the atomic writer's refusal of a journal path,
\* `DistinctNames` that an attachment whose extension is `md` is named apart from its stub
\* (`inbox.rs::attachment_name`, ADR-118's amendment), and `KeyClaim` the partial unique index on a
\* Mini App capture's key (`capture_store.rs::claim`, ADR-118's capture-key amendment).
\*
\* The action-to-code map, by `file::item` (re-read against each covered item at its stamp):
\* - Stream -> `inbox.rs::Inbox::attachment`, which opens `atomic.rs::stream`; the stream calls
\*   `refuse_journal(fs.journal(), target)?` before it creates its temporary file.
\* - Begin -> `inbox.rs::capture`, `let mut transaction = db.write().await?;` (BEGIN IMMEDIATE).
\* - Insert, the claim -> `capture_store.rs::claim`: `INSERT INTO inbox_captures ... ON CONFLICT
\*   DO NOTHING` with no conflict target; `rows_affected() == 1` answers `Claim::Claimed`;
\*   otherwise the stem arm `FROM inbox_captures WHERE stem = ?1` (`ByStem`), and, only when
\*   `row.source == Source::MiniApp`, the key arm
\*   `FROM inbox_captures WHERE capture_key = ?1 AND source = 'miniapp'` (`ByKey`, ruling (d)),
\*   each answering `Claim::Recorded { name }`.
\* - Insert's refusal, the rollback on Recorded -> `inbox.rs::capture`:
\*   `if let Claim::Recorded { name } = capture_store::claim(...)` then
\*   `transaction.rollback()`, `file.stream.discard()` and `Captured::AlreadyCaptured { name }`.
\* - Rename -> `inbox.rs::capture`, `file.stream.land()?`, after the claim (`ClaimFirst`) and
\*   before the stub (`Order`).
\* - AttachFile -> `inbox.rs::attachment_name`: `if extension.eq_ignore_ascii_case(".md")` names
\*   it `format!("{stem}.attachment{extension}")` (`DistinctNames`, ruling (c)(4)).
\* - WriteStub -> `inbox.rs::capture`,
\*   `atomic::write(fs, &inbox.folder.join(stub_name(&stem)), stub.as_bytes())?`, last.
\* - Commit -> `inbox.rs::capture`, `transaction.commit()`, then `Captured::Saved { name }`.
\* - Refused -> `atomic.rs::refuse_journal`, called by `atomic.rs::write` and `atomic.rs::stream`
\*   before any file is created (`Refused`). A Mini App capture streams nothing, so in the code its
\*   refusal comes at the stub's write inside the transaction, whose drop rolls the claim back:
\*   the model's Begin, Insert and Crash from "claimed", with no file written.
\* - Crash -> any `?` in `inbox.rs::capture` after `db.write()`: the dropped transaction rolls back
\*   and the dropped stream removes its temporary file.

EXTENDS Naturals, FiniteSets

CONSTANTS Order, Claim, ClaimFirst, JournalRefusal, DistinctNames, KeyClaim, MaxCrash

None == 0
Writers == {1, 2}
\* The stem's UTC day: 1 is the first capture's, 2 a later one.
Stems == {1, 2}
Steps == {"attach", "stub"}
Places == {"idle", "streamed", "begun", "claimed", "saved", "dup", "refused"}
\* The stub's file, `<stem>.md`: a name apart from the attachments' names 1 and 2.
StubFile == 3
Names == Writers \cup {StubFile}
\* A file is a stem and a name in that stem.
Files == Stems \X Names
\* No answer yet: a pair, so TLC compares an answer with files only.
NoFile == <<0, 0>>

VARIABLES layout, names, ext, day, source, pc, temp, lock, rows, attached, stubBy, written,
          answer, journal, crashes

vars == <<layout, names, ext, day, source, pc, temp, lock, rows, attached, stubBy, written,
          answer, journal, crashes>>

TypeOK ==
    /\ layout \in {"apart", "nested"}
    /\ names \in [Writers -> Writers]
    /\ ext \in [Writers -> {"md", "other"}]
    /\ day \in [Writers -> Stems]
    /\ source \in {"telegram", "miniapp"}
    /\ pc \in [Writers -> Places]
    /\ temp \in [Writers -> BOOLEAN]
    /\ lock \in Writers \cup {None}
    /\ rows \subseteq Writers
    /\ attached \subseteq Files
    /\ stubBy \in [Stems -> Writers \cup {None}]
    /\ written \in [Writers -> SUBSET Steps]
    /\ answer \in [Writers -> Files \cup {NoFile}]
    /\ journal \in BOOLEAN
    /\ crashes \in 0..MaxCrash

Init ==
    /\ layout \in {"apart", "nested"}
    /\ names \in {[w \in Writers |-> 1], [w \in Writers |-> w]}
    /\ ext \in [Writers -> {"md", "other"}]
    /\ day \in {[w \in Writers |-> 1], [w \in Writers |-> w]}
    /\ source \in {"telegram", "miniapp"}
    /\ pc = [w \in Writers |-> "idle"]
    /\ temp = [w \in Writers |-> FALSE]
    /\ lock = None
    /\ rows = {}
    /\ attached = {}
    /\ stubBy = [s \in Stems |-> None]
    /\ written = [w \in Writers |-> {}]
    /\ answer = [w \in Writers |-> NoFile]
    /\ journal = FALSE
    /\ crashes = 0

\* `inbox.rs::attachment_name`: the file a writer's attachment lands in. An attachment whose
\* extension is `md` takes a name with a second dot, which no stem holds, so it is never the stub's.
AttachFile(w) == <<day[w], IF ext[w] = "md" /\ ~DistinctNames THEN StubFile ELSE names[w]>>

\* The stub's file of a stem.
StubOf(s) == <<s, StubFile>>

\* `atomic.rs::refuse_journal`: a write whose target lies under a journal folder is refused before
\* any file is created, so a refused capture writes nothing.
Refused == layout = "nested" /\ JournalRefusal

\* A file the writer creates lands under a journal folder exactly when the layout nests the inbox.
Touch == journal' = (journal \/ layout = "nested")

\* The attachment's bytes stream into the writer's own temporary file beside its target.
Stream(w) ==
    /\ pc[w] = "idle"
    /\ written' = [written EXCEPT ![w] = {}]
    /\ answer' = [answer EXCEPT ![w] = NoFile]
    /\ IF Refused
          THEN /\ pc' = [pc EXCEPT ![w] = "refused"]
               /\ UNCHANGED <<temp, journal>>
          ELSE /\ pc' = [pc EXCEPT ![w] = "streamed"]
               /\ temp' = [temp EXCEPT ![w] = TRUE]
               /\ Touch
    /\ UNCHANGED <<layout, names, ext, day, source, lock, rows, attached, stubBy, crashes>>

\* The writer may write its files: after its claim in the chosen design, before it otherwise.
MayWrite(w) == IF ClaimFirst THEN pc[w] = "claimed" ELSE pc[w] = "streamed"

\* The attachment's rename into place. In the chosen order it comes first. A rename onto the stub's
\* file replaces the stub.
Rename(w) ==
    /\ MayWrite(w)
    /\ temp[w]
    /\ "attach" \notin written[w]
    /\ Order = "attach-first" \/ "stub" \in written[w]
    /\ temp' = [temp EXCEPT ![w] = FALSE]
    /\ attached' = attached \cup {AttachFile(w)}
    /\ stubBy' = IF AttachFile(w) = StubOf(day[w]) THEN [stubBy EXCEPT ![day[w]] = None]
                                                     ELSE stubBy
    /\ written' = [written EXCEPT ![w] = @ \cup {"attach"}]
    /\ Touch
    /\ UNCHANGED <<layout, names, ext, day, source, pc, lock, rows, answer, crashes>>

\* The stub, naming the writer's attachment. In the chosen order it is written last. Its atomic
\* write replaces whatever its file held, an attachment of that name included.
WriteStub(w) ==
    /\ MayWrite(w)
    /\ "stub" \notin written[w]
    /\ Order = "stub-first" \/ "attach" \in written[w]
    /\ stubBy' = [stubBy EXCEPT ![day[w]] = w]
    /\ attached' = attached \ {StubOf(day[w])}
    /\ written' = [written EXCEPT ![w] = @ \cup {"stub"}]
    /\ Touch
    /\ UNCHANGED <<layout, names, ext, day, source, pc, temp, lock, rows, answer, crashes>>

\* `BEGIN IMMEDIATE`: the one write lock on the ledger.
Begin(w) ==
    /\ pc[w] = "streamed"
    /\ lock = None
    /\ ClaimFirst \/ written[w] = Steps
    /\ lock' = w
    /\ pc' = [pc EXCEPT ![w] = "begun"]
    /\ UNCHANGED <<layout, names, ext, day, source, temp, rows, attached, stubBy, written, answer,
                   journal, crashes>>

\* The recorded rows a writer's insert meets: the row of its own stem (the primary key), and, for a
\* Mini App capture, any row of its key (the partial unique index); every writer here carries the
\* one key.
ByStem(w) == IF Claim THEN {r \in rows : day[r] = day[w]} ELSE {}
ByKey(w) == IF KeyClaim /\ source = "miniapp" THEN rows ELSE {}
Recorded(w) == ByStem(w) \cup ByKey(w)

\* `capture_store.rs::claim`: the row's insert, `ON CONFLICT DO NOTHING` with no conflict target, so
\* either key refuses. A stem, or a Mini App key, already recorded refuses: the transaction rolls
\* back, the writer's temporary file is removed, and it answers the recorded name.
Insert(w) ==
    /\ pc[w] = "begun"
    /\ IF Recorded(w) # {}
          THEN /\ pc' = [pc EXCEPT ![w] = "dup"]
               /\ lock' = None
               /\ temp' = [temp EXCEPT ![w] = FALSE]
               /\ answer' = [answer EXCEPT ![w] = AttachFile(CHOOSE r \in Recorded(w) : TRUE)]
          ELSE /\ pc' = [pc EXCEPT ![w] = "claimed"]
               /\ UNCHANGED <<lock, temp, answer>>
    /\ UNCHANGED <<layout, names, ext, day, source, rows, attached, stubBy, written, journal,
                   crashes>>

\* The commit, after both files: the row is recorded and the writer answers its own name.
Commit(w) ==
    /\ pc[w] = "claimed"
    /\ written[w] = Steps
    /\ rows' = rows \cup {w}
    /\ lock' = None
    /\ pc' = [pc EXCEPT ![w] = "saved"]
    /\ answer' = [answer EXCEPT ![w] = AttachFile(w)]
    /\ UNCHANGED <<layout, names, ext, day, source, temp, attached, stubBy, written, journal,
                   crashes>>

\* A crash between any two steps: the open transaction rolls back and its lock is released; the
\* files already renamed or written stay; the writer may start again.
Crash(w) ==
    /\ pc[w] \in {"streamed", "begun", "claimed"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![w] = "idle"]
    /\ temp' = [temp EXCEPT ![w] = FALSE]
    /\ lock' = IF lock = w THEN None ELSE lock
    /\ UNCHANGED <<layout, names, ext, day, source, rows, attached, stubBy, written, answer,
                   journal>>

\* Every writer has answered or been refused: the runs end here.
Quiescent == \A w \in Writers : pc[w] \in {"saved", "dup", "refused"}

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ \E w \in Writers :
          Stream(w) \/ Rename(w) \/ WriteStub(w) \/ Begin(w) \/ Insert(w) \/ Commit(w)
          \/ Crash(w)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* At most one stub per stem: each stem holds at most one recorded capture, and the stub on disk is
\* that capture's, so no later writer replaces it ("a capture is written once on the host", R3).
AtMostOneStub ==
    \A s \in Stems :
        LET here == {r \in rows : day[r] = s}
        IN  Cardinality(here) <= 1 /\ (here # {} => stubBy[s] \in here)

\* A stub on disk implies its attachment on disk: the curator never sees a stub before its file
\* ("the attachment lands before its stub", A2; `inbox.rs::capture`).
StubImpliesAttachment == \A s \in Stems : stubBy[s] # None => AttachFile(stubBy[s]) \in attached

\* A second writer of a recorded stem writes nothing and answers the existing name
\* ("a capture sent twice is written once", A4; `capture_store.rs::claim`).
SecondWriterWritesNothing ==
    \A w \in Writers :
        pc[w] = "dup" =>
            /\ written[w] = {}
            /\ ~temp[w]
            /\ answer[w] \in {AttachFile(r) : r \in rows}

\* Nothing is written under a journal folder ("no code path writes the journal", #56, A5).
NoJournalWrite == ~journal

\* No attachment ever lands in the stub's file, so the stub never replaces it and never names
\* itself ("an attachment never takes its stub's name", A23; `inbox.rs::attachment_name`).
StubNeverNamesTheAttachment == \A s \in Stems : StubOf(s) \notin attached

\* At most one capture per Mini App key: a retry, on the first day or a later one, never records a
\* second capture, so the inbox records one stub for the key ("a Mini App retry on a later UTC day
\* answers the first name", A24; `capture_store.rs::claim`). A stub a crash left before its commit
\* has no row; it is SPEC-118 section 6's risk, outside the curator's snapshot, and a Telegram
\* resend on a later day is a new capture.
OneCapturePerKey == source = "miniapp" => Cardinality(rows) <= 1
=============================================================================
