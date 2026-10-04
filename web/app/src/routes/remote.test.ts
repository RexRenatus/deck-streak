/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Remote from './remote/+page.svelte';

// SPEC-343 R16, A33. The harness screen reads each gamepad every animation frame and the keys from
// the window, and shows what it read, the review's side, the page's visibility and the wake lock's
// state, with a log of the last 200 events. The browser's gamepads, wake lock, visibility and
// animation frames are stubbed here, and the frames are run by hand.

interface FakePad {
  index: number;
  id: string;
  mapping: string;
  connected: boolean;
  buttons: { pressed: boolean }[];
  axes: number[];
}

function pad(index: number, id: string, mapping: string): FakePad {
  const buttons = Array.from({ length: 17 }, () => ({ pressed: false }));
  return { index, id, mapping, connected: true, buttons, axes: [0, 0, 0, 0] };
}

class FakeSentinel {
  released = 0;
  #listeners: (() => void)[] = [];

  release(): Promise<void> {
    this.released += 1;
    return Promise.resolve();
  }

  addEventListener(_type: 'release', listener: () => void): void {
    this.#listeners.push(listener);
  }
}

class FakeWakeLock {
  requests = 0;
  #pending: ((sentinel: FakeSentinel) => void)[] = [];

  request(_type: 'screen'): Promise<FakeSentinel> {
    this.requests += 1;
    return new Promise((resolve) => this.#pending.push(resolve));
  }

  grant(): FakeSentinel {
    const sentinel = new FakeSentinel();
    this.#pending.shift()?.(sentinel);
    return sentinel;
  }
}

let gamepads: FakePad[] = [];
let visibility: DocumentVisibilityState = 'visible';
let frames: FrameRequestCallback[] = [];
let lock: FakeWakeLock;

/** Runs the animation frame the screen asked for, then lets Svelte update the page. */
function runFrame(): void {
  const due = frames;
  frames = [];
  for (const callback of due) {
    callback(0);
  }
  flushSync();
}

/** The text the screen shows beside `label`, or `null` when it shows no such label. */
function shown(label: string): string | null {
  const term = [...document.querySelectorAll('dt')].find((dt) => dt.textContent?.trim() === label);
  return term?.nextElementSibling?.textContent?.trim() ?? null;
}

/** The log's rows, newest first, each as its cells' text. */
function logRows(): string[][] {
  const table = screen.queryByRole('table', { name: /log/i });
  const rows = table === null ? [] : [...table.querySelectorAll('tbody tr')];
  return rows.map((row) => [...row.querySelectorAll('td')].map((cell) => cell.textContent?.trim() ?? ''));
}

beforeEach(() => {
  gamepads = [];
  visibility = 'visible';
  frames = [];
  lock = new FakeWakeLock();
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback));
  vi.stubGlobal('cancelAnimationFrame', () => undefined);
  vi.spyOn(performance, 'now').mockReturnValue(1234.4);
  Object.defineProperty(navigator, 'getGamepads', { configurable: true, value: () => gamepads });
  Object.defineProperty(navigator, 'wakeLock', { configurable: true, value: lock });
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => visibility });
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('the remote harness', () => {
  it('the harness shows the remote, the keys and the lock', async () => {
    render(Remote);
    const remote = pad(0, 'synthetic remote', 'standard');
    gamepads = [remote];
    window.dispatchEvent(Object.assign(new Event('gamepadconnected'), { gamepad: remote }));
    runFrame();
    remote.buttons[0].pressed = true;
    remote.buttons[3].pressed = true;
    remote.axes = [0.25, -0.5, 0, 1];
    runFrame();

    // the gamepad's id, mapping, pressed buttons and axes; button 0 showed the answer
    expect([shown('Gamepad'), shown('Mapping'), shown('Pressed buttons'), shown('Axes')]).toEqual([
      'synthetic remote',
      'standard',
      '0 3',
      '0.25 -0.50 0.00 1.00'
    ]);
    expect(shown('Side')).toBe('answer');

    // a key's key and code; 3 answers Good and returns the review to the question side
    await fireEvent.keyDown(window, { key: '3', code: 'Digit3' });
    expect([shown('Key'), shown('Code'), shown('Side')]).toEqual(['"3"', 'Digit3', 'question']);

    // the visibility and the lock: requested once the review is open, held, then dropped on hidden
    expect([shown('Visibility'), shown('Wake lock')]).toEqual(['visible', 'off']);
    await fireEvent.click(screen.getByRole('button', { name: 'Open the review' }));
    expect([lock.requests, shown('Wake lock')]).toEqual([1, 'requesting']);
    const sentinel = lock.grant();
    await waitFor(() => expect(shown('Wake lock')).toBe('held'));
    visibility = 'hidden';
    document.dispatchEvent(new Event('visibilitychange'));
    flushSync();
    expect([shown('Visibility'), shown('Wake lock'), sentinel.released]).toEqual(['hidden', 'off', 1]);

    // every entry carries its time, source, raw input, action, visibility and lock state
    expect(logRows().find((row) => row[2] === '"3" Digit3')).toEqual([
      '1234',
      'key',
      '"3" Digit3',
      'good',
      'visible',
      'off'
    ]);

    // the log keeps the last 200 events, the newest first
    for (let event = 0; event < 250; event += 1) {
      await fireEvent.keyDown(window, { key: 'x', code: `Synthetic${event}` });
    }
    const rows = logRows();
    expect([rows.length, rows[0][2], rows[199][2]]).toEqual([200, '"x" Synthetic249', '"x" Synthetic50']);
  });
});
