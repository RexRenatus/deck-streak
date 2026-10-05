/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EngineError } from '$lib/engine/client';
import type { CardView, Head } from '$lib/engine/protocol';
import { telegram } from '$lib/telegram.svelte';
import { KEY_SWITCH } from './input';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

// SPEC-350 R7, R8, R9, R10, A16; ADR-361. The review screen draws the review's face in the card
// frame, with the card's classes and the engine's intervals; a key, a gamepad and a click reach the
// review, and a tap on the card leaves the keys with it; the status region announces a refusal, a
// card the frame refused and a done deck; and the screen lock is held while a card is shown, a
// gamepad is connected and the page is visible. The browser's gamepads, wake lock, visibility and
// animation frames are stubbed here, and the frames are run by hand.
const COUNTS = { new: 1, learning: 0, review: 2 };
const ESCAPES = '</style><base href="https://cards.example/">';

function view(id: number, extra: Partial<CardView> = {}): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '.card { color: navy; }',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: '',
    ...extra
  };
}

function head(card: CardView | null): Head {
  return { counts: COUNTS, card };
}

/** The engine as the screen sees it: each call recorded, each answer queued or refused once. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];
  refusals: Record<string, EngineError> = {};

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  card(): Promise<Head> {
    this.calls.push('card');
    return this.#answer('card', () => this.heads.shift() as Head);
  }

  rate(card: bigint, rating: number, ms: number): Promise<null> {
    this.calls.push(`rate ${card} ${rating} ${ms}`);
    return this.#answer('rate', () => null);
  }

  bury(card: bigint): Promise<null> {
    this.calls.push(`bury ${card}`);
    return this.#answer('bury', () => null);
  }

  flag(card: bigint): Promise<number> {
    this.calls.push(`flag ${card}`);
    return this.#answer('flag', () => 1);
  }

  undo(): Promise<null> {
    this.calls.push('undo');
    return this.#answer('undo', () => null);
  }

  async #answer<T>(op: string, value: () => T): Promise<T> {
    const refusal = this.refusals[op];
    if (refusal !== undefined) {
      delete this.refusals[op];
      throw refusal;
    }
    return value();
  }
}

interface FakePad {
  index: number;
  id: string;
  mapping: string;
  connected: boolean;
  buttons: { pressed: boolean }[];
  axes: number[];
}

function pad(index: number): FakePad {
  const buttons = Array.from({ length: 17 }, () => ({ pressed: false }));
  return { index, id: 'synthetic remote', mapping: 'standard', connected: true, buttons, axes: [0, 0, 0, 0] };
}

class FakeSentinel {
  released = 0;

  release(): Promise<void> {
    this.released += 1;
    return Promise.resolve();
  }

  addEventListener(_type: 'release', _listener: () => void): void {}
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

let gamepads: (FakePad | null)[] = [];
let visibility: DocumentVisibilityState = 'visible';
/** The animation frames asked for and not yet run or cancelled, by id. */
const frames = new Map<number, FrameRequestCallback>();
let frameIds = 0;
let cancelled: number[] = [];
let lock: FakeWakeLock;
let now = 1000;

/** Lets every answer arrive and every timer of zero run, then lets Svelte update the page. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** Runs the animation frame the screen asked for, then lets Svelte update the page. */
function runFrame(): void {
  const due = [...frames.values()];
  frames.clear();
  for (const callback of due) callback(0);
  flushSync();
}

function connectPad(remote: FakePad, event = 'gamepadconnected'): void {
  window.dispatchEvent(Object.assign(new Event(event), { gamepad: remote }));
  flushSync();
}

function showPage(state: DocumentVisibilityState): void {
  visibility = state;
  document.dispatchEvent(new Event('visibilitychange'));
  flushSync();
}

/** The card frame, and the body of the document it shows. */
function frame(): HTMLIFrameElement {
  return document.querySelector('iframe') as HTMLIFrameElement;
}

function body(): { title: string; classes: string; text: string } {
  const doc = new DOMParser().parseFromString(frame().getAttribute('srcdoc') ?? '', 'text/html');
  return { title: frame().title, classes: doc.body.className, text: doc.body.textContent?.trim() ?? '' };
}

function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

function review(): HTMLElement {
  return screen.getByRole('region', { name: 'Review' });
}

function button(name: string | RegExp): HTMLButtonElement {
  return screen.getByRole('button', { name }) as HTMLButtonElement;
}

beforeEach(() => {
  gamepads = [];
  visibility = 'visible';
  frames.clear();
  frameIds = 0;
  cancelled = [];
  lock = new FakeWakeLock();
  now = 1000;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frameIds += 1;
    frames.set(frameIds, callback);
    return frameIds;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => {
    cancelled.push(id);
    frames.delete(id);
  });
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  Object.defineProperty(navigator, 'wakeLock', { configurable: true, value: lock });
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => visibility });
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  // the default page has no gamepad API at all, as jsdom has none
  Reflect.deleteProperty(navigator, 'getGamepads');
  telegram.colorScheme = 'light';
});

/** Gives the page the gamepad API, answering `gamepads`. */
function withGamepads(): void {
  Object.defineProperty(navigator, 'getGamepads', { configurable: true, value: () => gamepads });
}

describe('the review screen', () => {
  it('the screen lock follows the review, the gamepad and the page', async () => {
    withGamepads();
    const client = new FakeClient([head(view(1)), head(view(2)), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // no lock is requested while no gamepad is connected, though a card is shown and the page visible
    expect([lock.requests, frames.size]).toEqual([0, 0]);

    // a gamepad connects: the lock is requested, and the gamepad is read each animation frame
    const first = pad(0);
    gamepads = [first];
    connectPad(first);
    expect([lock.requests, frames.size]).toEqual([1, 1]);
    const held = lock.grant();
    await settle();

    // the page is hidden: the lock is released and the frames stop; visible again, it is asked again
    showPage('hidden');
    expect([held.released, cancelled]).toEqual([1, [1]]);
    showPage('visible');
    expect([lock.requests, frames.size]).toEqual([2, 1]);
    const again = lock.grant();
    await settle();

    // a second gamepad connects and the first leaves: one is still connected, so the lock stays
    const second = pad(1);
    gamepads = [second];
    connectPad(second);
    connectPad(first, 'gamepaddisconnected');
    expect([lock.requests, again.released]).toEqual([2, 0]);

    // a rating in flight and the next card keep the lock; the done deck shows no card, and releases it
    await fireEvent.click(button('Show answer'));
    await fireEvent.click(button('Easy 4d'));
    await settle();
    expect([client.calls, again.released]).toEqual([['card', 'rate 1 4 0', 'card'], 0]);
    await fireEvent.click(button('Show answer'));
    await fireEvent.click(button('Easy 4d'));
    await settle();
    expect([status(), again.released, lock.requests]).toEqual(['This deck is done for today.', 1, 2]);

    // the last gamepad leaves: the frames stop
    connectPad(second, 'gamepaddisconnected');
    expect(cancelled).toHaveLength(2);
  });

  it('the review screen shows the card, reveals it and rates it', async () => {
    const client = new FakeClient([head(view(1)), head(view(2, { ordinal: 2 })), head(null)]);
    render(ReviewScreen, { client: async () => client });

    // before the first card: the loading line, a quiet status region, and no frame
    expect([screen.queryByText('Loading…') !== null, status(), document.querySelector('iframe')]).toEqual([
      true,
      '',
      null
    ]);
    await settle();

    // the question in the frame, titled by its side, its body carrying the card's classes; the counts;
    // and Show answer the one primary control, with no answer button yet
    expect(body()).toEqual({ title: "The card's question", classes: 'card card1', text: 'question 1' });
    expect(screen.queryByText('1 new, 0 learning, 2 to review')).not.toBeNull();
    expect([screen.queryByText('Loading…'), status()]).toEqual([null, '']);
    expect(button('Show answer').className).toContain('bg-foreground');
    expect(screen.queryAllByRole('button', { name: /^Again/ })).toEqual([]);

    // show answer reveals the answer with the engine's intervals, and focus returns to the review
    await fireEvent.click(button('Show answer'));
    expect(body()).toEqual({ title: "The card's answer", classes: 'card card1', text: 'answer 1' });
    expect(screen.queryByRole('button', { name: 'Show answer' })).toBeNull();
    expect(document.activeElement).toBe(review());

    // a rating names the shown card, its grade and the milliseconds since its question showed
    now = 3500.4;
    await fireEvent.click(button('Good <10m'));
    await settle();
    expect(client.calls).toEqual(['card', 'rate 1 3 2500', 'card']);
    expect(body()).toEqual({ title: "The card's question", classes: 'card card3', text: 'question 2' });

    // in Telegram's dark palette the body carries the night-mode classes too
    telegram.colorScheme = 'dark';
    flushSync();
    expect(body().classes).toBe('card card3 nightMode night_mode');
  });

  it('undo, bury and flag act on the shown card', async () => {
    const client = new FakeClient([head(view(1)), head(view(2, { undo: 'Undo Bury' })), head(view(1))]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // nothing to undo: undo is disabled; the flag is off
    expect([button('Undo').disabled, button('Flag').getAttribute('aria-pressed')]).toEqual([true, 'false']);

    // the flag acts on the shown card on the question side, which stays, and shows it pressed
    await fireEvent.click(button('Flag'));
    await settle();
    expect([client.calls, button('Flag').getAttribute('aria-pressed'), body().title]).toEqual([
      ['card', 'flag 1'],
      'true',
      "The card's question"
    ]);

    // bury moves on, and the engine names an undoable action, so undo is enabled and returns the card
    await fireEvent.click(button('Bury'));
    await settle();
    expect([client.calls.slice(2), button('Undo').disabled, button('Flag').getAttribute('aria-pressed')]).toEqual([
      ['bury 1', 'card'],
      false,
      'false'
    ]);
    await fireEvent.click(button('Undo'));
    await settle();
    expect([client.calls.slice(4), body().text, document.activeElement]).toEqual([
      ['undo', 'card'],
      'question 1',
      review()
    ]);
  });

  it('keys, the gamepad and a tap on the card reach the review', async () => {
    withGamepads();
    const remote = pad(1);
    gamepads = [null, remote];
    const client = new FakeClient([head(view(1)), head(view(2)), head(view(3)), head(view(4))]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // Space shows the answer, and 3 rates Good
    await fireEvent.keyDown(window, { key: ' ' });
    expect(body().title).toBe("The card's answer");
    await fireEvent.keyDown(window, { key: '3' });
    await settle();
    expect(client.calls).toEqual(['card', 'rate 1 3 0', 'card']);

    // the gamepad the page had before the screen opened is read each frame: its first frame is a
    // baseline, then its face button shows the answer, and pressed again answers Good
    runFrame();
    remote.buttons[0].pressed = true;
    runFrame();
    expect(body().title).toBe("The card's answer");
    remote.buttons[0].pressed = false;
    runFrame();
    remote.buttons[0].pressed = true;
    runFrame();
    await settle();
    expect(client.calls.slice(3)).toEqual(['rate 2 3 0', 'card']);

    // the key switch, turned off, is kept on this device and silences single-character keys only
    const keys = screen.getByRole('checkbox', { name: 'Single-key shortcuts' }) as HTMLInputElement;
    expect(keys.checked).toBe(true);
    await fireEvent.click(keys);
    expect(localStorage.getItem(KEY_SWITCH)).toBe('off');
    await fireEvent.keyDown(window, { key: ' ' });
    expect(body().title).toBe("The card's question");
    await fireEvent.keyDown(window, { key: 'Enter' });
    expect(body().title).toBe("The card's answer");

    // a tap on the card moves focus into the frame; the review takes it back for the next key
    await fireEvent.pointerDown(window);
    frame().focus();
    window.dispatchEvent(new FocusEvent('blur'));
    await settle();
    expect(document.activeElement).toBe(review());
    // a Tab that moved focus into the frame leaves it there
    await fireEvent.keyDown(window, { key: 'Tab' });
    frame().focus();
    window.dispatchEvent(new FocusEvent('blur'));
    await settle();
    expect(document.activeElement).toBe(frame());
  });

  it('a refusal, a card the frame refuses and a done deck are announced', async () => {
    const client = new FakeClient([head(view(1, { css: ESCAPES })), head(view(2)), head(null)]);
    client.refusals.card = new EngineError('collection-busy', 'another tab has the collection');
    render(ReviewScreen, { client: async () => client });
    await settle();

    // a refusal: announced, no card, no loading line, and a way to ask again
    expect(status()).toBe('Your collection is open in another tab. Close it there, then try again.');
    expect([document.querySelector('iframe'), screen.queryByText('Loading…')]).toEqual([null, null]);
    await fireEvent.click(button('Try again'));
    await settle();

    // a card the frame refuses: announced, its frame empty and marked, and its controls still there
    expect(status()).toBe('This card cannot be shown safely here. You can still answer, bury or flag it.');
    expect([frame().getAttribute('data-card-refused'), screen.queryByRole('button', { name: 'Try again' })]).toEqual([
      'escaped',
      null
    ]);
    expect(document.activeElement).toBe(review());
    await fireEvent.click(button('Show answer'));
    await fireEvent.click(button('Hard <6m'));
    await settle();
    expect([client.calls, status()]).toEqual([['card', 'card', 'rate 1 2 0', 'card'], '']);

    // the done deck: announced, no card and no controls, and the way back to the deck list
    await fireEvent.click(button('Show answer'));
    await fireEvent.click(button('Again <1m'));
    await settle();
    expect([status(), document.querySelector('iframe'), screen.queryByRole('button', { name: 'Bury' })]).toEqual([
      'This deck is done for today.',
      null,
      null
    ]);
    expect(screen.getByRole('link', { name: 'Back to decks' }).getAttribute('href')).toBe('/study');
  });

  it('a gamepad the page already had holds the lock once the page is visible, and closing releases it', async () => {
    withGamepads();
    gamepads = [null, pad(2)];
    visibility = 'hidden';
    const shown = render(ReviewScreen, { client: async () => new FakeClient([head(view(1))]) });
    await settle();
    expect([lock.requests, frames.size]).toEqual([0, 0]);

    showPage('visible');
    expect([lock.requests, frames.size]).toEqual([1, 1]);
    const held = lock.grant();
    await settle();

    shown.unmount();
    expect([held.released, cancelled]).toEqual([1, [1]]);
  });

  // Mutation coverage: each case holds behaviour the criteria's tests do not reach.
  it('the screen is headed Review', async () => {
    render(ReviewScreen, { client: async () => new FakeClient([head(view(1))]) });
    await settle();
    expect(screen.getByRole('heading', { level: 1, name: 'Review' }).textContent).toBe('Review');
  });

  it('the one gamepad the page already had leaves, and the lock goes with it', async () => {
    withGamepads();
    const only = pad(2);
    gamepads = [null, only];
    render(ReviewScreen, { client: async () => new FakeClient([head(view(1))]) });
    await settle();
    expect([lock.requests, frames.size]).toEqual([1, 1]);
    const held = lock.grant();
    await settle();

    connectPad(only, 'gamepaddisconnected');
    expect([held.released, cancelled]).toEqual([1, [1]]);
  });

  it('a gamepad that left is read afresh: a button it holds fires nothing on its next frame', async () => {
    withGamepads();
    const left = pad(0);
    const stays = pad(1);
    gamepads = [left, stays];
    render(ReviewScreen, { client: async () => new FakeClient([head(view(1))]) });
    await settle();
    runFrame();

    left.buttons[0].pressed = true;
    connectPad(left, 'gamepaddisconnected');
    runFrame();
    expect(body().title).toBe("The card's question");
    // and the next press is read as one
    left.buttons[0].pressed = false;
    runFrame();
    left.buttons[0].pressed = true;
    runFrame();
    expect(body().title).toBe("The card's answer");
  });

  it('a pointer after a Tab gives the focus the frame takes back', async () => {
    render(ReviewScreen, { client: async () => new FakeClient([head(view(1))]) });
    await settle();
    await fireEvent.keyDown(window, { key: 'Tab' });
    await fireEvent.pointerDown(window);
    frame().focus();
    window.dispatchEvent(new FocusEvent('blur'));
    await settle();
    expect(document.activeElement).toBe(review());
  });
});
