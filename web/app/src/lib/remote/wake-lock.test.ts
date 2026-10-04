import { describe, expect, it } from 'vitest';
import {
  step,
  WakeLockHolder,
  type Conditions,
  type Effect,
  type LockEvent,
  type LockState,
  type SentinelLike
} from './wake-lock';

// SPEC-343 R15, A28 to A32: the holder requests the screen lock while a review is open, a gamepad
// is connected and the page is visible, releases it when that falls, requests again after a
// release while hidden, records a refusal without repeating it, and follows the schematic's table.

class FakeSentinel implements SentinelLike {
  released = 0;
  #listeners: (() => void)[] = [];

  release(): Promise<void> {
    this.released += 1;
    return Promise.resolve();
  }

  addEventListener(_type: 'release', listener: () => void): void {
    this.#listeners.push(listener);
  }

  /** The browser releases the lock, as it does when the page is hidden. */
  browserReleases(): void {
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

class FakeWakeLock {
  requests = 0;
  #pending: { resolve: (sentinel: SentinelLike) => void; reject: (error: unknown) => void }[] = [];

  request(_type: 'screen'): Promise<SentinelLike> {
    this.requests += 1;
    return new Promise((resolve, reject) => this.#pending.push({ resolve, reject }));
  }

  #next() {
    const pending = this.#pending.shift();
    if (pending === undefined) {
      throw new Error('no request is in flight');
    }
    return pending;
  }

  grant(): FakeSentinel {
    const sentinel = new FakeSentinel();
    this.#next().resolve(sentinel);
    return sentinel;
  }

  deny(name: string): void {
    this.refuse(new DOMException('synthetic refusal', name));
  }

  refuse(error: unknown): void {
    this.#next().reject(error);
  }
}

const ALL: Conditions = { review: true, gamepad: true, visible: true };

async function holding(): Promise<{ lock: FakeWakeLock; holder: WakeLockHolder; sentinel: FakeSentinel }> {
  const lock = new FakeWakeLock();
  const holder = new WakeLockHolder(lock);
  holder.set(ALL);
  expect([lock.requests, holder.state]).toEqual([1, 'requesting']);
  const sentinel = lock.grant();
  await holder.settled();
  expect(holder.state).toBe('held');
  return { lock, holder, sentinel };
}

describe('the wake lock holder', () => {
  it('the lock is requested when all three hold', async () => {
    const lock = new FakeWakeLock();
    const holder = new WakeLockHolder(lock);
    holder.set({ review: true, gamepad: false, visible: true });
    holder.set({ review: true, gamepad: true, visible: true });
    expect([lock.requests, holder.state]).toEqual([1, 'requesting']);
    lock.grant();
    await holder.settled();
    expect(holder.state).toBe('held');

    const parts: Conditions[] = [
      { review: false, gamepad: false, visible: false },
      { review: true, gamepad: false, visible: false },
      { review: false, gamepad: true, visible: false },
      { review: false, gamepad: false, visible: true },
      { review: true, gamepad: true, visible: false },
      { review: true, gamepad: false, visible: true },
      { review: false, gamepad: true, visible: true }
    ];
    for (const conditions of parts) {
      const idle = new FakeWakeLock();
      const waiting = new WakeLockHolder(idle);
      waiting.set(conditions);
      expect([idle.requests, waiting.state], JSON.stringify(conditions)).toEqual([0, 'off']);
    }
    console.log(`examined ${parts.length} partial conditions`);
  });

  it('the lock is released on disconnect and on close', async () => {
    const disconnect = await holding();
    disconnect.holder.set({ ...ALL, gamepad: false });
    expect([disconnect.sentinel.released, disconnect.holder.state]).toEqual([1, 'off']);

    const close = await holding();
    close.holder.set({ ...ALL, review: false });
    expect([close.sentinel.released, close.holder.state]).toEqual([1, 'off']);
  });

  it('a lock released while hidden is requested again on visible', async () => {
    const releasedFirst = await holding();
    releasedFirst.sentinel.browserReleases();
    expect(releasedFirst.holder.state).toBe('released');
    releasedFirst.holder.set({ ...ALL, visible: false });
    expect([releasedFirst.lock.requests, releasedFirst.holder.state]).toEqual([1, 'off']);
    releasedFirst.holder.set(ALL);
    expect([releasedFirst.lock.requests, releasedFirst.holder.state]).toEqual([2, 'requesting']);

    const hiddenFirst = await holding();
    hiddenFirst.holder.set({ ...ALL, visible: false });
    expect([hiddenFirst.sentinel.released, hiddenFirst.holder.state]).toEqual([1, 'off']);
    hiddenFirst.sentinel.browserReleases();
    expect(hiddenFirst.holder.state, 'a release from a dropped lock changes nothing').toBe('off');
    hiddenFirst.holder.set(ALL);
    expect([hiddenFirst.lock.requests, hiddenFirst.holder.state]).toEqual([2, 'requesting']);
    const again = hiddenFirst.lock.grant();
    await hiddenFirst.holder.settled();
    hiddenFirst.sentinel.browserReleases();
    expect(hiddenFirst.holder.state, 'the old lock still changes nothing').toBe('held');
    again.browserReleases();
    expect(hiddenFirst.holder.state).toBe('released');
  });

  it('a refusal is recorded and not repeated until the condition rises again', async () => {
    const lock = new FakeWakeLock();
    const holder = new WakeLockHolder(lock);
    expect(holder.refusal).toBeNull();
    holder.set(ALL);
    expect([lock.requests, holder.state]).toEqual([1, 'requesting']);
    lock.deny('NotAllowedError');
    await holder.settled();
    expect([holder.state, holder.refusal]).toEqual(['refused', 'NotAllowedError']);

    holder.set({ ...ALL });
    expect(lock.requests, 'the same condition requests nothing').toBe(1);
    holder.set({ ...ALL, gamepad: false });
    expect([lock.requests, holder.state]).toEqual([1, 'off']);
    holder.set(ALL);
    expect([lock.requests, holder.state]).toEqual([2, 'requesting']);
  });

  it('every state answers every event as the table says', () => {
    const STATES: LockState[] = [
      'off',
      'requesting',
      'cancelling',
      'held',
      'released',
      'refused',
      'unsupported'
    ];
    const EVENTS: LockEvent[] = ['want', 'unwant', 'granted', 'denied', 'released'];
    // The schematic's table, section 4, one row per state in EVENTS' order.
    const TABLE: Record<LockState, [LockState, Effect][]> = {
      off: [['requesting', 'request'], ['off', 'none'], ['off', 'none'], ['off', 'none'], ['off', 'none']],
      requesting: [['requesting', 'none'], ['cancelling', 'none'], ['held', 'none'], ['refused', 'none'], ['requesting', 'none']],
      cancelling: [['requesting', 'none'], ['cancelling', 'none'], ['off', 'release'], ['off', 'none'], ['cancelling', 'none']],
      held: [['held', 'none'], ['off', 'release'], ['held', 'none'], ['held', 'none'], ['released', 'none']],
      released: [['requesting', 'request'], ['off', 'none'], ['released', 'none'], ['released', 'none'], ['released', 'none']],
      refused: [['refused', 'none'], ['off', 'none'], ['refused', 'none'], ['refused', 'none'], ['refused', 'none']],
      unsupported: EVENTS.map(() => ['unsupported', 'none'])
    };
    let cells = 0;
    for (const state of STATES) {
      EVENTS.forEach((event, column) => {
        const [next, effect] = TABLE[state][column];
        expect(step(state, event), `${state} on ${event}`).toEqual({ state: next, effect });
        cells += 1;
      });
    }
    console.log(`examined ${cells} cells`);
    expect(cells).toBe(35);
  });

  it('one request is in flight at most, and a browser with no wake lock reads unsupported', async () => {
    const lock = new FakeWakeLock();
    const holder = new WakeLockHolder(lock);
    holder.set(ALL);
    holder.set({ ...ALL, visible: false });
    expect(holder.state).toBe('cancelling');
    holder.set(ALL);
    expect([lock.requests, holder.state]).toEqual([1, 'requesting']);
    holder.set({ ...ALL, visible: false });
    const late = lock.grant();
    await holder.settled();
    expect([late.released, holder.state], 'a lock granted after the fall is released').toEqual([1, 'off']);

    const changes: string[] = [];
    const watched = new WakeLockHolder(lock, () => changes.push('changed'));
    watched.set(ALL);
    expect(changes).toEqual(['changed']);

    const none = new WakeLockHolder(undefined);
    expect(none.state).toBe('unsupported');
    none.set(ALL);
    expect(none.state).toBe('unsupported');
  });

  // MUTATION COVERAGE: green when written. A refusal is named by its `name` whatever realm made it,
  // and one with no name, or no object at all, by its own text.
  it('a refusal is named by its name, or else by its text', async () => {
    const REFUSALS: [unknown, string][] = [
      [{ name: 'NotAllowedError', message: 'from another realm' }, 'NotAllowedError'],
      ['synthetic text', 'synthetic text'],
      [{ name: 7 }, '[object Object]'],
      [undefined, 'undefined'],
      [null, 'null']
    ];
    for (const [error, name] of REFUSALS) {
      const lock = new FakeWakeLock();
      const holder = new WakeLockHolder(lock);
      holder.set(ALL);
      lock.refuse(error);
      await holder.settled();
      expect([holder.state, holder.refusal], String(error)).toEqual(['refused', name]);
    }
    console.log(`examined ${REFUSALS.length} refusals`);
  });
});
