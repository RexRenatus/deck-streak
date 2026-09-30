### Changed

- The mutation verdict step is now tested with each of its judges and its legs check failing
  alone, under the shell the workflow resolves for the step, so removing any one fold of an exit
  into the step's status, or piping a command so its exit is lost, turns a test red.
