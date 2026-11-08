### Fixed

- A header table or a row of the wrong shape in the mutation rows is now refused by name, with
  its file and its place, where it crashed the rows reader with a traceback or read an object as
  an empty table (SPEC-646, #951).
