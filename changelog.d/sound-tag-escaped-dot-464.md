### Fixed

- A sound tag whose file name escapes its dot or a letter of its extension, such as
  `[sound:clip%2Emp4]` or `[sound:clip.%6Dp4]`, is now decided on the name the media rules read: on
  the iPhone and iPad card view an escaped `mp4`, `m4v` or `mov` name shows its video as the plain
  spelling `[sound:clip.mp4]` does, where before it played as a sound or was left out. The web card
  view keeps every sound tag a sound, as before, and the media rules and size caps are unchanged
  (SPEC-464, #820).
