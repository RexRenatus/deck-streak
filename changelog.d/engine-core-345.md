### Added

- An engine core that holds Anki's engine for both clients behind one table, with a column per
  transport and the owner's exempt writes held apart. The native adapter and the web engine now
  reach the engine only through it, each keeping its own table checked first, and a census keeps
  every other member from naming the core.
