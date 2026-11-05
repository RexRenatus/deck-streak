### Fixed

- The sign-in methods list keeps the keyboard focus through its Remove confirm step: pressing a
  passkey's Remove moves the focus to Keep inside a confirm step that a screen reader announces by
  its question, Keep and Escape give the focus back to that passkey's Remove, and a removal lands
  it on the next passkey's Remove, else the previous one's, else the list. Each passkey's Remove is
  now named for its place and the day it was added, in every language, so two passkeys added on one
  day no longer share a name (SPEC-463, #818).
