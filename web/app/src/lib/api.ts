import { telegram } from './telegram.svelte';

/** What a screen gets from a call: the value, or why there is none. */
export type Answer<T> =
  | { readonly kind: 'ok'; readonly value: T }
  | { readonly kind: 'reopen' }
  | { readonly kind: 'unavailable' };

/** The owner's session as `GET /api/me` answers it. */
export interface Me {
  /** The server's study day, as an ISO date. */
  readonly studyDay: string;
}

/** What the client needs from its caller. */
export interface ApiOptions {
  /** The raw, signed launch data to open a session with, or null outside Telegram. */
  readonly launchData: () => string | null;
  /** The transport; the page's own `fetch` by default. */
  readonly fetch?: typeof globalThis.fetch;
}

/** The Mini App's one API client. */
export interface Api {
  /** The owner's session, with the server's study day. */
  me(): Promise<Answer<Me>>;
}

/** A client over `options`. */
export function createApi(options: ApiOptions): Api {
  void options;
  return { me: async () => ({ kind: 'unavailable' }) };
}

/** The app's client. */
export const api: Api = createApi({ launchData: () => telegram.launchData });
