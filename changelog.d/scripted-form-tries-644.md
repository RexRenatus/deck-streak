### Fixed

- The card view's scripted form card now submits its form up to five times, as the scripted
  navigation card already assigns its location, so one try that fails before it is sent no longer
  reads the form's channel as unobservable on one simulator. No reading or test is retried, and
  every wait and assertion is unchanged.
