### Added

- The card-script switch on iPhone and iPad (SPEC-355, ADR-366): the switch defaults off on iOS
  pending a measured containment layer. A card's own scripts run only when the switch is on and
  the factory reads back every control from the view it built; any missing control yields the
  scripts-off card view. Two layers join the seven: a document-start user
  script in every frame removes the peer-connection globals before card code runs, and the card
  view's store sends every connection to a loopback hold the app owns, which answers no byte and
  closes it. The window refusal also answers script dialogs, media capture and motion requests
  with a refusal. The planted suite gains a scripted card for every script-driven channel, each
  proved against a reference that is not blind, and a multicast DNS witness for lookups. It closes
  #677: a followed link opens no connection from the card view.
