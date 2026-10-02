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

They are green at 31c1af96, the vault's implementation: the vault's suite ran 111 of 111 green,
offline against the committed `.sqlx/` cache, where the stubs' commit ran 100 green and these 11
red. At that commit A4's last assertion lists the inbox's two files in the sorted order its helper
returns (`.md` before `.pdf`); the set it requires is unchanged, and its red above was an earlier
assertion.

The route's criteria, A15 to A18, were red at the route stub's commit: the route is served behind
the owner's session and answers 501, so each failed by its first status assertion. They are green
at the route's implementation, where the API's suite ran 48 of 48 green. A17's retry is sent the
same UTC day through the route; a retry after UTC midnight is the vault's A24, whose unique is the
route's `capture_id` (ruling (h)).

The screen's criteria, A20 and A21, were red at the screen stub's commit: the stub renders the
form, its save does nothing, and the client's capture answers unavailable without sending, so each
failed by its first assertion, that a capture reached the server. They are green at the screen's
implementation, where the web suite ran 159 of 159 green (148 before, and 11 red at the stub's
commit). Beside them, at the same two commits, the client's three capture tests (ruling (l)) and
the startapp case for the capture token (ruling (m)) went red then green. A20's words do not cover
the token, so its record is this sentence: `the capture token opens the quick capture screen` was
red at 2dc557fb1681c95c9d02271eab67c5a9fc1a6ff2, expected '/' to be '/capture' at
web/app/src/lib/startapp.test.ts:79, and green at 13f68a2d22ecc9510e6f4f4619ce77cd334f4455.

```red-first
A1: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_stub_and_stem_match_the_predecessors_golden panicked at crates/vault/tests/inbox_capture.rs:178: the attachment of the golden's first case, left "" right "2031-12-18-photo-hMpVD00JNQo8.jpg"
A1: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A2: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_attachment_lands_before_its_stub panicked at crates/vault/tests/inbox_capture.rs:328: the capture answers its attachment's name, left Saved { name: "" } right Saved { name: "2024-10-04-photo-AgADBAADr6cxG.jpg" }
A2: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A3: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_missing_inbox_is_refused_and_never_created panicked at crates/vault/tests/inbox_capture.rs:379: a missing vault root is refused with vault_missing: Ok(Inbox(..))
A3: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A4: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_capture_sent_twice_is_written_once panicked at crates/vault/tests/inbox_capture.rs:427: left Saved { name: "" } right Saved { name: "2024-10-04-document-BQACAgQAAxkB.pdf" }
A4: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A5: red at 9ebf336e0bf0f72654f33980b2a847056f679155: no_vault_write_reaches_a_journal_folder panicked at crates/vault/tests/atomic.rs:260: <vault>/Journal lies under the journal: Ok(())
A5: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A6: red at 9ebf336e0bf0f72654f33980b2a847056f679155: every_vault_file_write_is_the_atomic_writer panicked at crates/vault/tests/atomic.rs:539: file-writing calls outside the atomic writer: ["config.rs:287: create_new("]
A6: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A15: red at 60a734354969f1b1da60957b7240dd3a9216e248: a_quick_capture_writes_the_miniapp_stub panicked at crates/api/tests/inbox_capture_route.rs:210: {"reason":"not_implemented"}, left 501 right 201
A15: green at ed9d286ec8f41c5abdc0ae9ef0b55a8c6a81233d
A16: red at 60a734354969f1b1da60957b7240dd3a9216e248: a_journal_quick_capture_lands_in_the_inbox panicked at crates/api/tests/inbox_capture_route.rs:233: {"reason":"not_implemented"}, left 501 right 201
A16: green at ed9d286ec8f41c5abdc0ae9ef0b55a8c6a81233d
A17: red at 60a734354969f1b1da60957b7240dd3a9216e248: a_retried_quick_capture_answers_the_same_name panicked at crates/api/tests/inbox_capture_route.rs:260: {"reason":"not_implemented"}, left 501 right 201
A17: green at ed9d286ec8f41c5abdc0ae9ef0b55a8c6a81233d
A18: red at 60a734354969f1b1da60957b7240dd3a9216e248: the_quick_capture_is_owner_only_and_bounds_its_text panicked at crates/api/tests/inbox_capture_route.rs:296: 4000 characters with the session: {"reason":"not_implemented"}, left 501 right 201
A18: green at ed9d286ec8f41c5abdc0ae9ef0b55a8c6a81233d
A19: red at 9ebf336e0bf0f72654f33980b2a847056f679155: inbox_captures_export_and_erase_leave_the_files panicked at crates/vault/tests/inbox_capture.rs:503: inbox_captures is exported and erased
A19: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A20: red at 2dc557fb1681c95c9d02271eab67c5a9fc1a6ff2: QuickCapture > sends the text, the kind and one capture id per capture, and the same id on a retry failed at web/app/src/lib/capture/QuickCapture.test.ts:76: expected [] to have a length of 1 but got +0
A20: green at 13f68a2d22ecc9510e6f4f4619ce77cd334f4455
A21: red at 2dc557fb1681c95c9d02271eab67c5a9fc1a6ff2: QuickCapture > shows the saved name or the failure line failed at web/app/src/lib/capture/QuickCapture.test.ts:76: expected [] to have a length of 1 but got +0
A21: green at 13f68a2d22ecc9510e6f4f4619ce77cd334f4455
A22: red at 9ebf336e0bf0f72654f33980b2a847056f679155: the_layout_in_force_is_the_owners_or_the_default panicked at crates/vault/tests/layout_in_force.rs:23: unset, the vendored inbox is in force, left "" right "90-Inbox"
A22: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A23: red at 9ebf336e0bf0f72654f33980b2a847056f679155: an_md_attachment_never_takes_its_stubs_name panicked at crates/vault/tests/inbox_capture.rs:590: left "" right "2024-10-04-document-BQACAgQAAxkB"
A23: red at 9ebf336e0bf0f72654f33980b2a847056f679155: every_extension_keeps_its_bytes_apart_from_the_stub panicked at crates/vault/tests/inbox_capture.rs:639: the "md" attachment's name, left Saved { name: "" } right Saved { name: "2024-10-04-document-BQACAgQAAxk0.attachment.md" }
A23: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
A24: red at 9ebf336e0bf0f72654f33980b2a847056f679155: a_miniapp_retry_on_a_later_utc_day_answers_the_first_name panicked at crates/vault/tests/inbox_capture.rs:680: left Saved { name: "" } right Saved { name: "2024-10-04-text-6f1d2c9a-retry.md" }
A24: green at 31c1af96809f9cadad6da4c6e73d322fb7ebcfb6
```
