--------------------------- MODULE StagedNoJournal ---------------------------
\* @phx covers crates/vault/src/staged.rs anchor=apply digest=sha256:20c4e6e0312424f077075812bbcc9342fc7e5ea37473650e6eca3b03f061b10b
\* @phx covers crates/vault/src/staged.rs anchor=apply_op digest=sha256:6e443a7efa33dd6ea092207d4f0cf36d6391588687ae07c2e6da4051c5e16716
\* @phx covers crates/vault/src/staged.rs anchor=create_folders digest=sha256:f2d0f7eb93e88aa692393e2f05d125a6678d2c113b2ba900d64ecb1d2a3b0bef
\* @phx covers crates/vault/src/atomic.rs anchor=refuse_journal_resolved digest=sha256:1d1214fc9260d159643a718d039d2eba2ceb21d832eb393aaf2ab7dd59b82abc
\* @phx covers crates/vault/src/atomic.rs anchor=resolve digest=sha256:cbb47bd45964e2822b1ea2ed69a8e78ab57174e08fca9a3ea3d89f9921c99675
\* @phx covers crates/vault/src/fs.rs anchor=refuse digest=sha256:8f907ad56dd617cec00d34617784515cf47740dd8371d5453bc443c763175d15
\* @phx cites #56
\* @phx property NoJournalWrite ramp=report
\* @phx witness witness/an-apply-that-trusts-the-check-alone.cfg kills=NoJournalWrite
\* @phx witness witness/a-guard-that-compares-names.cfg kills=NoJournalWrite
\* @phx witness witness/a-guard-over-byte-writes-only.cfg kills=NoJournalWrite
\* @phx witness witness/targets-resolved-once-before-the-first-write.cfg kills=NoJournalWrite
\*
\* A staged duty run (SPEC-042 R4), applied by the three-verb executor while the owner's devices keep
\* writing the vault. The executor checks the run, asks the gate, checks it again, and then applies
\* its operations one by one: a folder it creates, a capture it renames into place, the bytes of a
\* note it writes. Between any two of those steps another writer may replace a duty's folder with a
\* link into a journal folder, or put the folder back. Nothing a run applies may land under a
\* journal folder, as the file system resolves the path at the moment the operation runs (#56;
\* SPEC-118 R5's rule, carried to the staged executor).
\*
\* What the model abstracts, and why:
\* - Three folders stand for the vault: `A` and `B`, which a duty may write, and `J`, a journal
\*   folder. One journal folder stands for the layout's list: the guard refuses a path under any of
\*   them, so each is judged alone.
\* - `link` maps each folder name to the folder it resolves to. A duty folder resolves to itself or,
\*   once another writer made it a link, to `J`; `J` resolves to itself. A layout whose journal
\*   folder's own name is a link is CaptureOnce's case, judged there, and is not repeated here.
\* - A run is `NumOps` operations, each one kind (`dir`, `rename` or `write`) over the folder it
\*   writes and, for a rename, the folder its source leaves. A create or an update is the model's
\*   `dir` for the folders it needs and its `write`; a move is its `dir` and its `rename`. Every run
\*   of every kind over every folder is chosen at Init, so the runs a check refuses are judged too.
\* - The check and the re-check are one step each, and each refuses a run that names a journal
\*   folder (`staged.rs::allowed_folders`) or whose folder resolves into one at that moment
\*   (`staged.rs::placement`). The gate passes or refuses; its classes are not modelled.
\* - The guard's path check and the operation it guards are modelled as one atomic step.
\* - Another writer's changes are bounded by `MaxEnv` in all, a link made or a folder put back.
\*
\* The switches are the fixed design when `ApplyGuard` is TRUE, `GuardKinds` is every kind and
\* `ResolveAt` is "op": `ApplyGuard` is the guard at apply time, `GuardKinds` the operations it
\* guards, and `ResolveAt` when the guard resolves the paths, at each operation ("op") or once, at
\* the first write ("once"). `GuardHit` compares the resolved folders; the witness that compares
\* names binds it to `NameHit`.
\*
\* The action-to-code map, by `file::item`:
\* - Check, Recheck -> `staged.rs::apply`, `self.check(&run, run_dir)?` before and after the gate.
\* - Gate -> `staged.rs::apply`, `self.gate.judge(run_dir)`.
\* - Apply -> `staged.rs::apply_op` and `staged.rs::create_folders`: the folder's `create_dir`, the
\*   move's `rename` and the note's `atomic::write`, each through the journal guard built at that
\*   operation over the run's own journal folders (`fs.rs::JournalGuard`, whose `refuse` calls
\*   `atomic.rs::refuse_journal_resolved`, which compares the path as written and as
\*   `atomic.rs::resolve` resolves it).
\* - Stop -> `staged.rs::apply`: a refused operation stops the run, and the operations before it stay
\*   applied (ADR-316).
\* - Relink, Restore -> the owner's devices, outside the executor.

EXTENDS Naturals, FiniteSets

CONSTANTS ApplyGuard, GuardKinds, ResolveAt, NumOps, MaxEnv

Folders == {"A", "B", "J"}
Journal == {"J"}
Duty == Folders \ Journal
Kinds == {"dir", "rename", "write"}
Places == {"check", "gate", "recheck", "apply", "done", "stopped", "discarded"}
Ends == {"done", "stopped", "discarded"}

\* An operation: its kind, the folder it writes, and the folder its source leaves (a rename's only;
\* the folder it writes otherwise).
Ops ==
    {[kind |-> k, to |-> t, from |-> t] : k \in {"dir", "write"}, t \in Folders}
    \cup {[kind |-> "rename", to |-> t, from |-> s] : t \in Folders, s \in Folders}

\* Every folder an operation changes: a rename changes the folder it leaves too.
Touches(op) == {op.to, op.from}

VARIABLES link, env, run, pc, i, touched, seen

vars == <<link, env, run, pc, i, touched, seen>>

TypeOK ==
    /\ link \in [Folders -> Folders]
    /\ link["J"] = "J"
    /\ env \in 0..MaxEnv
    /\ run \in [1..NumOps -> Ops]
    /\ pc \in Places
    /\ i \in 1..NumOps
    /\ touched \subseteq Folders
    /\ seen \in [Folders -> Folders]

Links == {l \in [Folders -> Folders] : l["J"] = "J" /\ \A f \in Duty : l[f] \in {f, "J"}}

\* The executor's own check, as `staged.rs::check` runs it before and after the gate: a run that
\* names a journal folder, or whose folder resolves into one now, is refused.
CheckRefuses ==
    \E n \in 1..NumOps : \E f \in Touches(run[n]) : f \in Journal \/ link[f] \in Journal

\* The folders the guard resolves the operation's paths through: the file system as it stands at
\* this operation, or as it stood at the run's first write.
Seen == IF ResolveAt = "once" /\ i > 1 THEN seen ELSE link

\* The guard's comparison: a path under a journal folder as written or as resolved
\* (`atomic.rs::refuse_journal_resolved`).
GuardHit == \E f \in Touches(run[i]) : f \in Journal \/ Seen[f] \in Journal

\* The comparison of names alone, `atomic.rs::refuse_journal` without the resolution: a duty
\* folder made a link into a journal folder passes it.
NameHit == \E f \in Touches(run[i]) : f \in Journal

Refused == ApplyGuard /\ run[i].kind \in GuardKinds /\ GuardHit

Init ==
    /\ link \in Links
    /\ env = 0
    /\ run \in [1..NumOps -> Ops]
    /\ pc = "check"
    /\ i = 1
    /\ touched = {}
    /\ seen = link

Check ==
    /\ pc = "check"
    /\ pc' = IF CheckRefuses THEN "discarded" ELSE "gate"
    /\ UNCHANGED <<link, env, run, i, touched, seen>>

Gate ==
    /\ pc = "gate"
    /\ pc' \in {"recheck", "discarded"}
    /\ UNCHANGED <<link, env, run, i, touched, seen>>

Recheck ==
    /\ pc = "recheck"
    /\ pc' = IF CheckRefuses THEN "discarded" ELSE "apply"
    /\ UNCHANGED <<link, env, run, i, touched, seen>>

\* One operation: refused by the guard, which stops the run with nothing written, or applied, which
\* changes each folder it touches as the file system resolves it now.
Apply ==
    /\ pc = "apply"
    /\ seen' = IF i = 1 THEN link ELSE seen
    /\ IF Refused
          THEN /\ pc' = "stopped"
               /\ UNCHANGED <<touched, i>>
          ELSE /\ touched' = touched \cup {link[f] : f \in Touches(run[i])}
               /\ pc' = IF i = NumOps THEN "done" ELSE "apply"
               /\ i' = IF i = NumOps THEN i ELSE i + 1
    /\ UNCHANGED <<link, env, run>>

\* Another writer makes a duty folder a link into the journal folder.
Relink(f) ==
    /\ env < MaxEnv
    /\ link[f] = f
    /\ link' = [link EXCEPT ![f] = "J"]
    /\ env' = env + 1
    /\ UNCHANGED <<run, pc, i, touched, seen>>

\* Another writer puts the folder back.
Restore(f) ==
    /\ env < MaxEnv
    /\ link[f] # f
    /\ link' = [link EXCEPT ![f] = f]
    /\ env' = env + 1
    /\ UNCHANGED <<run, pc, i, touched, seen>>

Finished == pc \in Ends /\ UNCHANGED vars

Next ==
    \/ Check \/ Gate \/ Recheck \/ Apply
    \/ \E f \in Duty : Relink(f) \/ Restore(f)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* Nothing a run applies lands under a journal folder, as the file system resolves each path at the
\* moment its operation runs ("every create_dir, rename and atomic write a run applies goes through
\* the journal guard", ADR-316; `staged.rs::apply_op`).
NoJournalWrite == touched \cap Journal = {}

=============================================================================
