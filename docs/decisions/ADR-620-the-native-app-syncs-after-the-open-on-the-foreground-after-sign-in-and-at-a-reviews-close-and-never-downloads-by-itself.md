---
status: proposed
decision-makers: "the DeckStreak architect"
---

# The native app syncs after the open, on the foreground, after sign-in and at a review's close, and never downloads by itself

## Context and Problem Statement

SPEC-503 gave the iPhone and iPad app a sync facade, the full-sync choice and a Sync button in the
account sheet (ADR-521), but the app syncs only from that button and lists its decks once, at
launch, so a fresh install that downloads its collection lists no new decks until it is opened
again. SPEC-358 R18 and R33 name the session's sync moments, and ADR-388 D12 is the web client's
same rule: one sync after the open and one at the close, neither awaited by study. SPEC-600 asks
the native app to run its normal sync at those moments and to list its decks again after each sync.
This ADR decides when the app syncs on its own, what stops it, and where the rule lives.

## Decision Drivers

- No write by itself: a full sync stays the learner's choice (SPEC-503 R4, R7).
- A normal sync discards the undo queue (SPEC-358 M10), so a sync must not land under an open
  review unless the learner asked for it.
- The app's Swift stays thin: each file keeps its role's decision ceiling with no override
  (SPEC-347 R11).
- The moments must be testable on a simulator without a sync service.

## Considered Options

### When the app syncs on its own (D1)

- After the open, on a return from the background, after a sign-in and at a review's own close,
  beside the button: chosen, because these are SPEC-358 R18's and R33's moments and ADR-388 D12's.
- On every scene phase change: rejected, because an inactive turn (a notification banner, the app
  switcher) is no return, and the first active at launch would sync twice.
- On a timer while the app is open: rejected, because a sync would land under a review and take
  the undo queue with it.

### What stops an automatic sync (D2)

- Drop a sync asked while one runs or while the choice is open: chosen, because a queued sync would
  run after the choice closes with an answer the learner never saw.
- Queue it: rejected, for the same reason, and because two queued moments would sync twice.
- Sync under an open review on a return: rejected, by the undo queue (SPEC-358 M10).

### What an automatic sync that needs a full sync does (D3)

- Stop at the unsynced read and the counts, which wait in the sync section: chosen, because the
  download is the learner's choice (SPEC-503 R4).
- Open the account sheet by itself: rejected, because a sheet that opens on its own interrupts
  study, and the status line that would say so is not this delivery.

### The decks after a sync (D4)

- List the decks again after every sync that completes: chosen, because a download replaces the
  collection and a normal sync may add or remove decks.
- List them again only after a download: rejected, because a normal sync from another device adds
  decks too.
- Clear the chosen deck through the close action when the list lacks it: rejected, because the
  close runs a sync, and listing the decks again would then start a second one.

### Where the rule lives and how a test reaches it (D7)

- One new model file, `SyncMoments.swift`, owned by the shell's model, and a seam protocol,
  `ShellEngine`, which the engine session declares: chosen, because the moments' decisions fit the
  model role's ceiling there, and a recording engine can stand in for the session.
- The moments in `AppModel.swift`: rejected, because that file would pass its role's ceiling.
- A test that drives a real collection and a stub sync service: rejected, because the app opens
  no network client of its own, and the facade's calls are SPEC-503's tests.

## Decision Outcome

- **D1. The moments.** When signed in, the app runs one normal sync through the sync section's
  path: (a) once after the collection opens and its decks are listed; (b) each time the scene
  returns to active after the background; (c) once after a sign-in stores its host key; (d) once
  after the review screen's own close action closes an open review; (e) the Sync button, unchanged.
- **D2. The guards.** Signed out, no automatic moment calls the engine and the section's sentence
  stays. A sync asked while one runs or while the choice is open is dropped. (a) and (b) wait while
  a review is open; (c), (d) and (e) run.
- **D3. No write by itself.** An automatic sync that needs a full sync stops at the counts; the
  download runs only from the learner's tap, and no sheet opens by itself.
- **D4. The decks after a sync.** After every sync that completes, the shell's model lists the
  decks again through the session; a chosen deck the new list lacks is cleared, which is no close
  moment. A list that fails shows its refusal in place of the list, as a failed list at launch does.
- **D5. No new words.** No sentence, label, status line, notice or empty state is added or changed.
- **D6. Unchanged surfaces.** The facade, the minimum-client statement read, the endpoint rule,
  media off, the choice's order, the one Keychain item and the sign-in are unchanged, as are the
  engine's calls, pairs and allow-list, and the server's own sync cadence.
- **D7. The seam and the files.** `AppModel` takes the open, the deck list, the login and the
  sync section's calls through `ShellEngine`, which `EngineSession` declares in one line; the app
  passes its one engine session, and a review takes that same session. `SyncMoments.swift` holds
  the moments, their guards, the background-to-active tracking, the drop and the chosen deck kept;
  `SyncModel.swift` only reports a completed sync; `DeckListView.swift` only hands each scene phase
  and the review's close to the model.

### Consequences

- Good: a new install lists its downloaded decks with no relaunch, and the native app syncs at the
  web client's moments within each file's decision budget.
- Bad: a return to the foreground on a slow network starts a sync the learner did not ask for.
- Neutral: the moments' sentences are the sync section's own, so nothing new is said.

### Confirmation

SPEC-600's A1 to A12 and its mutation rows in S60000-S60099.

## What would make this wrong

- The status line (SPEC-358 R19) lands: an automatic sync that needs a full sync may then say so
  outside the account sheet.
- The native client gains a background sync: (b) then meets a sync that already ran.

## More Information

ADR-388, ADR-521; SPEC-347, SPEC-358, SPEC-503.
