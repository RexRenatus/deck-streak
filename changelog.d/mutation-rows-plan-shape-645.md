### Fixed

- The mutation rows runner refuses a `--rows-from` plan of the wrong shape by name in `prove`,
  `legs` and `merge`: a plan that is valid JSON but not an object, a `rows` value that is not an
  array, or a row id that is not a string, where each crashed the command with a traceback
  (SPEC-645, #950).
