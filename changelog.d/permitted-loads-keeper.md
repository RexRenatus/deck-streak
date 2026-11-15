### Fixed

- The card view's planted suite now holds one kept card across the views of its permitted-loads
  test, as its factory test already does, so a card view built after another view was taken down no
  longer waits on WebKit processes that exited idle, and the test no longer fails for it. Every
  view, wait and assertion of the test is unchanged.
