# Red-first record: SPEC-118

This record is V1a's, the first of SPEC-118's two pull requests. It records the criteria this pull
request delivers (A1 to A6 and A15 to A24); V1b adds the lines of A7 to A14 when it moves them back
into the acceptance fence (SPEC-118 section 3c).

The order of work: the SPEC moved out of `docs/specs/planned/` as a pure rename, then its status and
section 3c; the CaptureOnce model with its witness, checked before any vault code; the registry and
the `inbox_capture_stub` golden; the vault's tests beside stubs that compile and answer nothing;
their implementation; then the coordination use case, the route, the Mini App screen and the
wiring.

Each criterion is run at its red commit, selecting its own test, and fails by assertion, not by a
compile error, a missing fixture or an empty selection.

The vault's criteria were red at the stubs' commit, each by its first assertion: the stubs
compile and answer an empty name, `Ok`, an empty layout and no journal folder.

```red-first
A1: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_stub_and_stem_match_the_predecessors_golden panicked at crates/vault/tests/inbox_capture.rs:178: the attachment of the golden's first case, left "" right "2031-12-18-photo-hMpVD00JNQo8.jpg"
A2: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_attachment_lands_before_its_stub panicked at crates/vault/tests/inbox_capture.rs:328: the capture answers its attachment's name, left Saved { name: "" } right Saved { name: "2024-10-04-photo-AgADBAADr6cxG.jpg" }
A3: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_missing_inbox_is_refused_and_never_created panicked at crates/vault/tests/inbox_capture.rs:379: a missing vault root is refused with vault_missing: Ok(Inbox(..))
A4: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_capture_sent_twice_is_written_once panicked at crates/vault/tests/inbox_capture.rs:427: left Saved { name: "" } right Saved { name: "2024-10-04-document-BQACAgQAAxkB.pdf" }
A5: red at 9ebf336e0bf0f72654f33980b2a847056f679155: no_vault_write_reaches_a_journal_folder panicked at crates/vault/tests/atomic.rs:260: <vault>/Journal lies under the journal: Ok(())
A6: red at 9ebf336e0bf0f72654f33980b2a847056f679155: every_vault_file_write_is_the_atomic_writer panicked at crates/vault/tests/atomic.rs:539: file-writing calls outside the atomic writer: ["config.rs:287: create_new("]
A19: red at 9ebf336e0bf0f72654f33980b2a847056f679155: inbox_captures_export_and_erase_leave_the_files panicked at crates/vault/tests/inbox_capture.rs:503: inbox_captures is exported and erased
A22: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_layout_in_force_is_the_owners_or_the_default panicked at crates/vault/tests/layout_in_force.rs:23: unset, the vendored inbox is in force, left "" right "90-Inbox"
A23: red at 9ebf336e0bf0f72654f33980b2a847056f679155: an_md_attachment_never_takes_its_stubs_name panicked at crates/vault/tests/inbox_capture.rs:590: left "" right "2024-10-04-document-BQACAgQAAxkB"
A23: red at 9ebf336e0bf0f72654f33980b2a847056f679155: every_extension_keeps_its_bytes_apart_from_the_stub panicked at crates/vault/tests/inbox_capture.rs:639: the "md" attachment's name, left Saved { name: "" } right Saved { name: "2024-10-04-document-BQACAgQAAxk0.attachment.md" }
A24: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_miniapp_retry_on_a_later_utc_day_answers_the_first_name panicked at crates/vault/tests/inbox_capture.rs:680: left Saved { name: "" } right Saved { name: "2024-10-04-text-6f1d2c9a-retry.md" }
```
