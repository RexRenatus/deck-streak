### Changed

- The harness memory test now ends the app at the end of each pass, after its measured window
  closes, as the cold-start test already does; the window, the passes and the waits keep their
  bounds. It is not proven to cure the iPad simulator's failures in that step (SPEC-690).
