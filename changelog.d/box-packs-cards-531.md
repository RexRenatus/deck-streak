### Changed

- The box-pack runner now removes the temporary cards directory it created when it exits, whether
  the verdict passed, failed or the run stopped on an error (#531). A `BOX_PACKS_OUT` directory is
  never removed, and `BOX_PACKS_KEEP=1` keeps the runner's own directory and prints its path.
