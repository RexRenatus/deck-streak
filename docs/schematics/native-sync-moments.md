# The native client's sync moments (SPEC-600)

The sequence of the five moments at which the iPhone and iPad app runs its normal sync, the guards
each passes, and the deck list read again after a sync completes. Each arrow is one call; the sync
itself is SPEC-503's, unchanged (`docs/schematics/native-first-sync.md`).

```mermaid
sequenceDiagram
    actor Learner
    participant Shell as ShellView
    participant App as AppModel
    participant Moments as SyncMoments
    participant Sync as SyncModel
    participant Engine as ShellEngine (EngineSession)

    Shell->>App: start()
    App->>Engine: open(arguments)
    App->>Engine: decks()
    App->>Moments: opened (a)
    Shell->>App: phaseChanged(phase)
    App->>Moments: entered(phase) (b): background, then active
    Learner->>App: signIn()
    App->>Engine: login, then the key is stored
    App->>Moments: stored (c)
    Learner->>Shell: the review's close
    Shell->>App: closeReview()
    App->>Moments: closed (d): the deck cleared, the review closed
    Learner->>Sync: Sync button (e), unchanged
    Moments->>Moments: (a) and (b) wait while a review is open
    Moments->>Moments: signed in, none running, no choice open, else dropped
    Moments->>Sync: sync()
    Sync->>Engine: sync(hostKey, endpoint)
    alt a full sync is needed
        Sync->>Engine: unsynced(), then countChoice()
        Sync-->>Learner: the counts wait in the account sheet#59; nothing is written
    else no full sync is needed
        Sync->>App: completed()
        App->>Engine: decks()
        App->>Moments: kept(chosen deck, new list)
        Moments-->>App: the chosen deck, or none when the list lacks it
    end
```

## Where each rule lives

| Rule | Where |
| --- | --- |
| The moments, the review-open wait, the drop and the chosen deck kept | `ios/App/Sources/SyncMoments.swift` |
| The moments' call sites, the seam and the deck list read again | `ios/App/Sources/AppModel.swift` |
| The completed sync reported | `ios/App/Sources/SyncModel.swift` |
| The scene phase and the review's close handed to the model | `ios/App/Sources/DeckListView.swift` |
| The normal sync, the counts and the download | SPEC-503, unchanged |
