### Fixed

- Every recompute cycle, the scheduled sync's and the owner's, now holds a notification router
  (SPEC-319, #571; ADR-319). Before it, no production cycle held one, so no badge, record, level-up
  or relight celebration was ever raised. The cycle's router has no bot transport and the sync job
  still loads no bot credential: a celebration it routes outside the quiet window is held on the
  queue, and the bot's flush after the owner's `/sync` is answered or the 07:36 held flush sends it.
- The celebrations' switch is seeded off at the job's start where no value is stored (#402 items 8
  and 11), so every celebration is withheld and marked answered until the cutover checklist turns
  it on; a stored value is never overwritten.
- A model of the held flush now covers a hold outside the quiet window, with a witness for a hold
  no later flush reaches, and mutation rows S31900 to S31905 hold the holding arm, the router each
  cycle attaches and the switch's seed.
