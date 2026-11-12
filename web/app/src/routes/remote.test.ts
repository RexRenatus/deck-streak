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
  #pending: { resolve: (sentinel: FakeSentinel) => void; reject: (error: unknown) => void }[] = [];

  request(_type: 'screen'): Promise<FakeSentinel> {
    this.requests += 1;
    return new Promise((resolve, reject) => this.#pending.push({ resolve, reject }));
  }

  grant(): FakeSentinel {
    const sentinel = new FakeSentinel();
    this.#pending.shift()?.resolve(sentinel);
    return sentinel;
  }

  deny(name: string): void {
    this.#pending.shift()?.reject(new DOMException('synthetic refusal', name));
  }
}

let gamepads: FakePad[] = [];
let visibility: DocumentVisibilityState = 'visible';
let frames: FrameRequestCallback[] = [];
let frameIds = 0;
let cancelled: number[] = [];
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
  frameIds = 0;
  cancelled = [];
  lock = new FakeWakeLock();
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frames.push(callback);
    frameIds += 1;
    return frameIds;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => cancelled.push(id));
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

  // MUTATION COVERAGE: green when written, after the screen. Each holds a branch of the screen that
  // A33 does not examine: what each source logs and when a frame logs nothing, a disconnect's own
  // effects, the refusal's name, the condition's gamepad part, and what closing the screen undoes.

  it('each source logs its raw input, and a frame that changed nothing logs nothing', async () => {
    render(Remote);
    const remote = pad(1, 'synthetic pad', 'standard');
    gamepads = [null as unknown as FakePad, remote];
    window.dispatchEvent(Object.assign(new Event('gamepadconnected'), { gamepad: remote }));
    runFrame();
    runFrame();
    remote.buttons[0].pressed = true;
    remote.buttons[2].pressed = true;
    runFrame();
    runFrame();
    remote.buttons[0].pressed = false;
    remote.buttons[2].pressed = false;
    runFrame();
    await fireEvent.click(screen.getByRole('button', { name: 'Open the review' }));
    expect(screen.getByRole('button', { name: 'Close the review' })).toBeTruthy();
    visibility = 'hidden';
    document.dispatchEvent(new Event('visibilitychange'));
    window.dispatchEvent(Object.assign(new Event('gamepaddisconnected'), { gamepad: remote }));
    flushSync();

    // oldest first: source, raw input, action, visibility, lock
    expect(logRows().map((row) => row.slice(1)).reverse()).toEqual([
      ['visibility', 'visible', '', 'visible', 'off'],
      ['connection', 'connected 1', '', 'visible', 'off'],
      ['gamepad', '[]', '', 'visible', 'off'],
      ['gamepad', '[0 2]', 'show-answer', 'visible', 'off'],
      ['gamepad', '[0 2]', 'replay', 'visible', 'off'],
      ['gamepad', '[]', '', 'visible', 'off'],
      ['review', 'true', '', 'visible', 'off'],
      ['lock', 'requesting', '', 'visible', 'requesting'],
      ['visibility', 'hidden', '', 'hidden', 'requesting'],
      ['lock', 'cancelling', '', 'hidden', 'cancelling'],
      ['connection', 'disconnected 1', '', 'hidden', 'cancelling']
    ]);
  });

  it('a disconnect drops its gamepad at once, and its return is a new baseline', async () => {
    render(Remote);
    const first = pad(0, 'synthetic first', 'standard');
    const second = pad(1, 'synthetic second', 'standard');
    gamepads = [first, second];
    runFrame();
    await fireEvent.click(screen.getByRole('button', { name: 'Open the review' }));
    const sentinel = lock.grant();
    await waitFor(() => expect(shown('Wake lock')).toBe('held'));

    window.dispatchEvent(Object.assign(new Event('gamepaddisconnected'), { gamepad: second }));
    flushSync();
    expect([shown('Gamepad'), shown('Wake lock')]).toEqual(['synthetic first', 'held']);

    window.dispatchEvent(Object.assign(new Event('gamepaddisconnected'), { gamepad: first }));
    flushSync();
    expect([shown('Gamepad'), shown('Wake lock'), sentinel.released]).toEqual([null, 'off', 1]);
    expect(screen.getByText('No gamepad yet. Press a button on it to connect it.')).toBeTruthy();

    // the browser still names the first gamepad, its button already down: a baseline fires nothing
    first.buttons[0].pressed = true;
    gamepads = [first];
    runFrame();
    expect([shown('Gamepad'), shown('Side')]).toEqual(['synthetic first', 'question']);
  });

  it('a refusal shows its name, the lock waits for a gamepad, and closing the screen releases it', async () => {
    const { unmount } = render(Remote);
    expect(shown('Refusal')).toBe('None');
    await fireEvent.click(screen.getByRole('button', { name: 'Open the review' }));
    expect([lock.requests, shown('Wake lock')]).toEqual([0, 'off']);

    gamepads = [pad(0, 'synthetic remote', 'standard')];
    runFrame();
    expect([lock.requests, shown('Wake lock')]).toEqual([1, 'requesting']);
    lock.deny('NotAllowedError');
    await waitFor(() => expect(shown('Wake lock')).toBe('refused'));
    expect(shown('Refusal')).toBe('NotAllowedError');

    await fireEvent.click(screen.getByRole('button', { name: 'Close the review' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open the review' }));
    expect(lock.requests).toBe(2);
    const sentinel = lock.grant();
    await waitFor(() => expect(shown('Wake lock')).toBe('held'));

    const pending = frameIds;
    unmount();
    expect([sentinel.released, cancelled]).toEqual([1, [pending]]);
  });

  // MUTATION COVERAGE: green when written. Every label the screen shows, from the base locale.
  it('the screen labels its parts', () => {
    render(Remote);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Remote harness');
    expect(document.querySelector('main > p')?.textContent).toBe(
      'Reads a gamepad, or a remote in keyboard mode, and logs every event. It sends nothing anywhere.'
    );
    expect(screen.getAllByRole('heading', { level: 2 }).map((heading) => heading.textContent)).toEqual([
      'Status',
      'Gamepads',
      'Log'
    ]);
    const log = screen.getByRole('table', { name: 'Log' });
    expect(log.querySelector('caption')?.textContent).toBe('The last 200 events, the newest first');
    expect(screen.getAllByRole('columnheader').map((header) => header.textContent)).toEqual([
      'Time (ms)',
      'Source',
      'Input',
      'Action',
      'Visibility',
      'Wake lock'
    ]);
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
  });
});
