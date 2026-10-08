### Added

- Tests show that the web review frame shows a cloze card, an answer that opens with its question
  where its template says `{{FrontSide}}`, and a note type's own CSS as the engine renders them
  (SPEC-372, ADR-383; #729). One golden file under `web/app/src/lib/study/` holds the fixture's
  inputs and the frame CSS written by hand and the engine's render: the engine's test renders the
  fixture and compares, and the review screen's test serves the render and reads the frame. No
  production file, frame policy or dependency changes, and four mutation rows pin the face's CSS
  and its text joining.
