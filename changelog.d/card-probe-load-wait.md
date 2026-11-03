### Fixed

- The card view's planted suite now starts each card's load wait once the card's page first
  answers the app, so a slow start of the page's own process is no longer read as a card that
  never loaded. The 10-second load wait, every test and every assertion are unchanged, and each
  load prints how long its page took to answer.
