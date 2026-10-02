### Fixed

- The open lapse's count of silent days is a named step, `next_silent_count`, that saturates at `u32::MAX`, and a test pins its exact values at and below that bound, so replacing the step with a wrapping add is now caught (#534).
