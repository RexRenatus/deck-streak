# The native review's undo of its own last answer, bury or flag (SPEC-515)

The sequence of an undo on iPhone and iPad, from the review's open to the next queued card. Each
arrow is one call; the core decides every rule, and the engine's undo status (3,7) is read after
the open and after every answer, bury, flag and undo.

```mermaid
sequenceDiagram
    actor Owner
    participant Control as UndoControl and UndoDialogView
    participant Model as ReviewModel and ReviewUndo
    participant Session as ReviewSession and EngineSession
    participant Adapter as ffi Engine
    participant Core as engine core
    participant Engine as the engine

    Model->>Session: next() after the open
    Session->>Adapter: run(3,7)
    Adapter->>Core: dispatcher.run(3,7), native column
    Core-->>Model: the undo label (empty: the control is disabled)
    Owner->>Model: Good (or bury, or flag)
    Model->>Session: answer(rating)
    Session->>Adapter: answer(card, grade, states)
    Adapter->>Core: run_answer, then run(3,7) and native_undo::answered
    Core->>Engine: AnswerCard (13,4); GetUndoStatus; the newest review
    Core-->>Adapter: the record of the review's own last answer
    Adapter->>Adapter: the slot keeps it (a bury's or a flag's replaces it)
    Model->>Session: next(): the next card and the undo label
    Owner->>Control: Undo
    Control->>Model: perform(.undo)
    Model->>Session: undoOffer()
    Session->>Adapter: undo_offer()
    Adapter->>Core: run(3,7), native_undo::judged (judge or judge_change)
    Core-->>Adapter: admitted, with the card's question as one line
    Adapter-->>Control: the offer's words (card, answer, state it goes back to)
    Control->>Owner: the dialog, focus on Keep
    alt Keep
        Owner->>Control: Keep it
        Control->>Model: perform(.keep): nothing is written
    else Undo answer, bury or flag
        Owner->>Control: Undo answer
        Control->>Model: perform(.confirmUndo)
        Model->>Session: undo(offer)
        Session->>Adapter: undo(card, step)
        Adapter->>Core: the slot names card and step; judged again
        Adapter->>Core: run_exempt(OwnerGesture Undo on the card, the record)
        Core->>Core: run_undo or run_restore judges at the write
        Core->>Engine: Undo (3,8), empty request
        Adapter-->>Model: undone (or the sentence that says why not)
        Model->>Session: next(): the undone card again, and the label
    end
```

A refusal at the offer or at the write writes nothing, and the review shows the adapter's sentence:
synced, or something changed after it. After a normal sync the engine's undo queue is empty and
the record has synced, so the offer says the answer has synced.
