### Added

- The web study screens, part 1 (SPEC-350): `/study` lists the engine's decks with their new,
  learning and review counts, and `/study/review` shows the deck's next card from the engine in
  the sealed card frame, reveals its answer, and rates it with four buttons that carry the engine's
  own interval labels. Undo, bury and the red flag act on the card on screen and on no other: the
  web engine keeps the card it showed and refuses a rating, bury or flag for any other id. Anki's
  keys and the remote's gamepad reach one handler through the remote's modules, a switch stored on
  the device turns the single-character keys off, and focus comes back to the review after a tap on
  the card. The engine core's web column and the web engine's study calls grow to the review's
  sixteen pairs, and CI's web-engine job runs the review end to end in Chromium and WebKit over the
  real module. ADR-361 records the choices and what each was chosen against; media, sound, speech,
  the Home Screen and the mapping screen are the next part (#630).
