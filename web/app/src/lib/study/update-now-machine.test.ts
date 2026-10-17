import { describe, expect, it } from 'vitest';
import type { StatusWord, Synced, Unsynced } from '$lib/engine/protocol';
import { UpdateNow, type UpdateClient } from './update-now';

// SPEC-409 R2, R4 to R7, R12; ADR-423 D1, D2, D7. The offer's state machine alone, driven without
// a screen, so each guard of it is held by a test of its own.
const PROMPT = 'Your reviews stay on this device until they sync. You can sync them now.';
const DONE = 'Sync finished.';
const OFFLINE = 'The sync server cannot be reached right now. Your reviews stay on this device.';
const ABSENT = 'You are not signed in to sync.';

/** A client whose sync answers when the test says so. */
class Held {
  calls: string[] = [];
  #waiting: Array<(answer: Synced) => void> = [];
  readonly client: UpdateClient = {
    sync: () => {
      this.calls.push('sync');
      return new Promise<Synced>((resolve) => this.#waiting.push(resolve));
    },
    unsynced: async (): Promise<Unsynced> => {
      this.calls.push('unsynced');
      return { reviews: 3 } as Unsynced;
    }
  };
  answer(index: number, status: StatusWord): void {
    this.#waiting[index]?.({ status, required: null });
  }
}

function machine(held: Held | undefined) {
  const changes = { count: 0 };
  const offer = new UpdateNow(
    () => (held === undefined ? undefined : async () => held.client),
    () => {
      changes.count += 1;
    }
  );
  return { offer, changes };
}

const later = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('the offer machine', () => {
  it('shows once, on the prompt, with no link and no count', () => {
    const { offer, changes } = machine(new Held());
    expect(offer.face).toEqual({ shown: false, working: false, line: PROMPT, link: false, count: null });
    offer.raise();
    offer.raise();
    expect(changes.count).toBe(1);
    expect(offer.face).toEqual({ shown: true, working: false, line: PROMPT, link: false, count: null });
  });

  it('a screen with no client never shows the offer and a tap does nothing', async () => {
    const { offer, changes } = machine(undefined);
    offer.raise();
    await offer.tap();
    expect(changes.count).toBe(0);
    expect(offer.face.shown).toBe(false);
    expect(offer.face.working).toBe(false);
    expect(offer.face.line).toBe(PROMPT);
  });

  it('a second tap while one runs syncs nothing more', async () => {
    const held = new Held();
    const { offer, changes } = machine(held);
    offer.raise();
    const first = offer.tap();
    await later();
    await offer.tap();
    expect(held.calls).toEqual(['sync']);
    expect(changes.count).toBe(2);
    held.answer(0, 'held');
    await first;
    expect(held.calls).toEqual(['sync', 'unsynced']);
  });

  it('a held answer is done with no link, and a signed-out answer links', async () => {
    const held = new Held();
    const { offer } = machine(held);
    offer.raise();
    const run = offer.tap();
    await later();
    held.answer(0, 'held');
    await run;
    expect(offer.face).toEqual({ shown: true, working: false, line: DONE, link: false, count: 3 });

    const signed = new Held();
    const other = machine(signed).offer;
    other.raise();
    const again = other.tap();
    await later();
    signed.answer(0, 'absent');
    await again;
    expect(other.face).toEqual({ shown: true, working: false, line: ABSENT, link: true, count: 3 });

    const offline = new Held();
    const third = machine(offline).offer;
    third.raise();
    const last = third.tap();
    await later();
    offline.answer(0, 'offline');
    await last;
    expect(third.face).toEqual({ shown: true, working: false, line: OFFLINE, link: false, count: 3 });
  });

  it('a stopped engine says so with no link and no count', async () => {
    const broken: UpdateClient = {
      sync: () => Promise.reject(new Error('stopped')),
      unsynced: () => Promise.reject(new Error('stopped'))
    };
    const offer = new UpdateNow(() => async () => broken, () => undefined);
    offer.raise();
    await offer.tap();
    expect(offer.face).toEqual({
      shown: true,
      working: false,
      line: 'The study engine stopped. Try again.',
      link: false,
      count: null
    });
  });

  it('not now ends the offer, the work and the say of a running sync', async () => {
    const held = new Held();
    const { offer, changes } = machine(held);
    offer.raise();
    const run = offer.tap();
    await later();
    offer.dismiss();
    expect(offer.face).toEqual({ shown: false, working: false, line: PROMPT, link: false, count: null });
    held.answer(0, 'held');
    await run;
    expect(changes.count).toBe(3);
    expect(offer.face).toEqual({ shown: false, working: false, line: PROMPT, link: false, count: null });
    offer.raise();
    expect(offer.face.shown).toBe(false);
  });

  it('a dismissed tap does not speak for the next one', async () => {
    const held = new Held();
    const { offer } = machine(held);
    offer.raise();
    const first = offer.tap();
    await later();
    offer.dismiss();
    const second = offer.tap();
    await later();
    expect(offer.face.working).toBe(true);
    held.answer(0, 'held');
    await first;
    expect(offer.face.working).toBe(true);
    expect(offer.face.line).toBe('Syncing…');
    held.answer(1, 'offline');
    await second;
    expect(offer.face.line).toBe(OFFLINE);
  });

  it('not now on an idle offer leaves it able to start a sync', async () => {
    const held = new Held();
    const { offer } = machine(held);
    offer.dismiss();
    expect(offer.face.working).toBe(false);
    const run = offer.tap();
    await later();
    expect(held.calls).toEqual(['sync']);
    held.answer(0, 'held');
    await run;
  });
});
