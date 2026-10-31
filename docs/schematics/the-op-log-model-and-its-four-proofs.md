# The op log model and its four proofs

- **Kind:** data flow and component.
- **Read at:** `d3b09d5e`, before the sync core exists.
- **Decided by:** ADR-522 (SPEC-504).

## The flow the model checks

A device makes an operation, keeps it durably in its queue, and pushes the queue's head. The judge reads the
device's token, then the (origin, seq) key, and either refuses, answers Duplicate, or hands the operation to the
writer, which appends it at the next log position and records the loser of a concurrent edit. The device drops the
operation from its queue only when it reads its answer, and pulls the log by cursor into its copy, folding the card
and each field's register as the operations arrive.

```mermaid
flowchart LR
    subgraph DEV["a device (durable: queue, cursor, copy, clock)"]
        MAKE["Make<br/>clock past last and seen"]
        Q["queue"]
        VER["Verdict<br/>drop the answered op"]
        PULL["Pull by cursor"]
        COPY["copy: card fold in key order,<br/>field register by max key"]
        REB["Rebootstrap<br/>replace the copy, keep the queue"]
    end
    subgraph SRV["the server"]
        JUDGE["Judge<br/>token, then (origin, seq)"]
        INF["judged, uncommitted<br/>(lost on a crash)"]
        COMMIT["Commit<br/>next position; loser to conflicts"]
        LOG["log"]
        ACK["answers<br/>(lost on a crash)"]
    end
    MAKE --> Q -->|push head| JUDGE
    JUDGE -->|revoked| ACK
    JUDGE -->|key present| ACK
    JUDGE -->|new| INF --> COMMIT --> LOG
    COMMIT --> ACK --> VER --> Q
    LOG --> PULL --> COPY
    LOG --> REB --> COPY
```

## Where each property and proof sits

| place | TLA+ property | Lean entry |
|---|---|---|
| Make | CausalHlc, CausalHlcAtMake | SyncClock: `tick_gt_seen`, `tick_ge_pt`, `run_strict_mono`, `causal` |
| Judge | ExactlyOnce, RevokedDeviceRefused | PushJudge: `judge_idempotent`, `no_review_mutation` |
| Commit | NoSkip, MonotoneLog, NoSilentOverwrite, RetractOnlyOwn | - |
| Verdict, Rebootstrap | NoLostReview, RebootstrapKeepsPending | - |
| Pull and the copy | LocalIsPrefix, CanonicalCardFold, FieldConverges, UndoOwnOnly, Convergence | ReviewKey: `fold_sorted_perm_invariant`; FieldMerge: `merge_comm`, `merge_assoc`, `merge_idem`, `apply_perm_invariant` |

The Lean entries share `Formal/SyncOrder.lean`, the key and edit orders, and write their vectors through
`Formal/SyncVectors.lean`. The sync core, when it lands, covers its tick, sort, merge and judge from these entries and
reads each vectors file byte-equal in a Rust test.
