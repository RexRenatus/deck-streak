import { IDBFactory } from 'fake-indexeddb';
import { describe, expect, it } from 'vitest';
import { CredentialStore } from '../engine/credential';
import {
  Bus,
  ReleaseService,
  loginAnswering,
  readStored,
  sentinel,
  workerDeps
} from '../engine/credential-stand-in.test.support';
import { signOut } from './sign-out';

// SPEC-363 section 7, B9: the page's sign-out forgets the sync key in the Worker first and then ends
// the web session, and the key is gone when the session's end never arrives.

/** A store holding a key, and the engine client's forget over it, which records when it answered. */
async function signedIn(order: string[]) {
  const database = new IDBFactory();
  const store = new CredentialStore(workerDeps(database, new ReleaseService(), new Bus()));
  await store.obtain(loginAnswering(sentinel('host', 'key', 'out')).login, sentinel('sync', 'user'), sentinel('pass', 'word'));
  const engine = {
    forgetSync: async () => {
      const word = await store.forget();
      order.push('forget');
      return word;
    }
  };
  return { database, engine };
}

describe("the page's sign-out", () => {
  it('sign-out forgets the sync key first, offline too', async () => {
    const order: string[] = [];
    const sent: [string, RequestInit | undefined][] = [];
    const online = async (input: RequestInfo | URL, init?: RequestInit) => {
      order.push('end');
      sent.push([String(input), init]);
      return new Response(null, { status: 204 });
    };
    const first = await signedIn(order);
    const ended = await signOut(first.engine, online);
    expect(order).toEqual(['forget', 'end']);
    expect(await readStored(first.database)).toEqual({ generation: 2n, sealed: undefined });
    // StateChange admits the session's end only as JSON
    expect(sent.map(([url, init]) => [url, init?.method, new Headers(init?.headers).get('content-type')])).toEqual([
      ['/api/session', 'DELETE', 'application/json']
    ]);
    expect(ended).toBe(true);

    // offline: the session's end never arrives, and the key is gone all the same
    order.length = 0;
    const offline = async () => {
      order.push('end');
      throw new TypeError('the network is down');
    };
    const second = await signedIn(order);
    expect(await signOut(second.engine, offline)).toBe(false);
    expect(order).toEqual(['forget', 'end']);
    expect(await readStored(second.database)).toEqual({ generation: 2n, sealed: undefined });

    // a forget that rejects still ends the session, and its rejection reaches the caller
    sent.length = 0;
    const failing = { forgetSync: () => Promise.reject(new Error('the engine did not load')) };
    await expect(signOut(failing, online)).rejects.toThrow('the engine did not load');
    expect(sent.map(([url, init]) => [url, init?.method])).toEqual([['/api/session', 'DELETE']]);
  });

  it("sign-out sends the session's end with the page's own session", async () => {
    // mutation coverage, written after green: the session's cookie rides only a same-origin request
    const sent: (RequestInit | undefined)[] = [];
    const online = async (_input: RequestInfo | URL, init?: RequestInit) => {
      sent.push(init);
      return new Response(null, { status: 204 });
    };
    expect(await signOut({ forgetSync: async () => 'absent' }, online)).toBe(true);
    expect(sent.map((init) => init?.credentials)).toEqual(['same-origin']);
  });
});
