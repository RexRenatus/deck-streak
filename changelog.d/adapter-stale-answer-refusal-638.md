### Changed

- Each adapter's handover of the engine's refusal of a stale answer is now run by a test through
  the adapter itself: a native press on the states a card was shown with, after another answer
  changed the card, reaches its caller as the engine's invalid-input refusal and records nothing;
  in the browser, a rating on a card that a sync changed while it was shown reaches the page as the
  engine's refusal and leaves the card's review count unchanged; the web boundary census now owes
  the rating the line that throws the refusal, and three mutation rows show each test failing when
  an adapter swallows the refusal or names the wrong reason (SPEC-638, #937).
