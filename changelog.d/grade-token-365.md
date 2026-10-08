### Changed

- Only the owner's press records a grade (SPEC-365, ADR-376). The engine core holds AnswerCard for
  an owner's answer, a one-use value naming the card a press showed and the grade it pressed, and
  records only a request that names that card and that grade; `run` refuses the call on both
  transports. The web's `rate` and the native `Engine::answer` mint the answer from the card the
  review showed and the next state its grade picks, and the page's queue-head `answer` operation
  is gone. Every engine door records Again or Good: the wire refuses Hard and Easy by name, and the
  native codec, the review screen and its answer bar offer the two grades, Again then Good.
