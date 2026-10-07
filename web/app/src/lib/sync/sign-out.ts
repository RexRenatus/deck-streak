// The page's sign-out (SPEC-363 R14): it forgets the sync key in the Worker first, and then ends
// the web session, so the key is gone even when the device is offline.
import type { StatusWord } from '../engine/protocol';

/** The engine client's one credential operation the sign-out needs. */
export interface Forgetter {
  forgetSync(): Promise<StatusWord>;
}

/** Ends the web session. StateChange admits it only as JSON from the page's own site; a session
 * whose end never arrives answers false. */
async function endSession(fetch: typeof globalThis.fetch): Promise<boolean> {
  try {
    const response = await fetch('/api/session', {
      method: 'DELETE',
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' }
    });
    return response.ok;
  } catch {
    return false;
  }
}

/** Signs out: the sync key is forgotten first, and the web session is ended whether or not the
 * forget succeeded. Answers whether the session ended; a forget that failed rejects after the
 * session's end was sent, so the page can say the key may remain. */
export async function signOut(engine: Forgetter, fetch: typeof globalThis.fetch): Promise<boolean> {
  let ended = false;
  try {
    await engine.forgetSync();
  } finally {
    ended = await endSession(fetch);
  }
  return ended;
}
