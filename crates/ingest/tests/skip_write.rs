//! The skip day's take on a working copy, against the engine's own sync server behind the recording
//! layer (SPEC-083 A5, A9, A25, A26, A28, A29, A34, A36, A38 to A40, A44, A47 to A49, A53, A54 and
//! A56). This target is compiled at edition 2021 so that its tests set the process's zone without
//! `unsafe` (SPEC-083 section 3), and every test in it holds the target's one lock for its whole run.
