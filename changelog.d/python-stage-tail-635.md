### Changed

- The gate script's python stage now runs six more test modules in its module pool, and its serial
  tail keeps only the deploy test pattern and `test_engine_pin.py`, the modules that cannot yet
  share a run; its last line and its exit are as before, and `PYTHON_STAGE_JOBS=1` still runs every
  module one after another.
