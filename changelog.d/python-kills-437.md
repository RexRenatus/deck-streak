### Changed

- Each priced Python file's mutation runs are interleaved across its legs, so a costly region of
  a file is spread over all of them while every leg's count and projection stay as they were;
  each equivalence record is judged only in its own class; the write-back script refuses an
  unknown key before it writes anything; and the parity oracle's generator gains behavioural
  tests (SPEC-437, #822).
