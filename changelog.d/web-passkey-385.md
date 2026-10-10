### Added

- The web client links and signs in with a passkey pinned to the owner (SPEC-385, ADR-399).
  Inside Telegram the sign-in methods screen lists the owner's ways in, Telegram first, removes a
  passkey only after a confirm step, and opens a link page in the browser with a one-time code;
  the browser redeems the code on a tap and creates the passkey there. Outside Telegram a sign-in
  page signs in with that passkey and opens Today, and a call the server refuses for want of a
  session asks for sign-in instead of asking to reopen the app. ADR-132 is accepted (#627).
