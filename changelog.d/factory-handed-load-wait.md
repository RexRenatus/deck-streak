### Fixed

- The card view's factory suite now starts the load wait of its row that hands a card over with
  a base URL once the handed document first answers the app, so a slow commit of that document is
  no longer read as a card that never loaded. The load wait, every test and every assertion are
  unchanged.
