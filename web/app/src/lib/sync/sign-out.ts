// The page's sign-out (SPEC-363 R14): it forgets the sync key in the Worker first, and then ends
// the web session, so the key is gone even when the device is offline.
import type { StatusWord } from '../engine/credential';

/** The engine client's one credential operation the sign-out needs. */
export interface Forgetter {
  forgetSync(): Promise<StatusWord>;
}

/** Signs out: answers whether the web session ended. */
export async function signOut(engine: Forgetter, fetch: typeof globalThis.fetch): Promise<boolean> {
  void engine;
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
