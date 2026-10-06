/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Action, Side } from '$lib/remote/actions';
import type { PadSnapshot } from '$lib/remote/gamepad';
import type { KeyInput } from '$lib/remote/keys';
import { BUTTONS, KEYS, STICK } from '$lib/remote/mapping';
import { KEY_SWITCH, StudyInput } from './input';
import { buttonOf, keyOf, MappingStore } from './mapping-store';
import MappingScreen from './MappingScreen.svelte';

// SPEC-350 R18, A30; ADR-361 D15. The remote's mapping is kept per device under one local-storage
// key, a mode the learner never changed keeps its default, and the review's input passes each
// mode's map to #663's readers. On the mapping screen a learner changes the key or the gamepad
// button an action takes by pressing the new one, restores each mode apart, turns the left stick's
// grades off, and turns the single-key shortcuts off as the review does.

/** The one key the mapping is stored under, written here so a change to it reddens this test. */
const STORED = 'deck-streak.study.mapping';

function key(name: string, extra: Partial<KeyInput> = {}): KeyInput {
  return {
    key: name,
    code: '',
    repeat: false,
    isComposing: false,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    target: null,
    preventDefault: () => undefined,
    ...extra
  };
}

function pad(pressed: number[] = [], axes: number[] = [0, 0, 0, 0], mapping = 'standard'): PadSnapshot {
  return {
    index: 0,
    id: 'a remote',
    mapping,
    buttons: Array.from({ length: 17 }, (_, at) => pressed.includes(at)),
    axes
  };
}

class MemoryStorage {
  readonly items = new Map<string, string>();

  getItem(name: string): string | null {
    return this.items.get(name) ?? null;
  }

  setItem(name: string, value: string): void {
    this.items.set(name, value);
  }
}

/** A storage the browser refuses to write. */
class RefusingStorage extends MemoryStorage {
  override setItem(): void {
    throw new DOMException('the storage is full', 'QuotaExceededError');
  }
}

function holding(value: unknown): MemoryStorage {
  const storage = new MemoryStorage();
  storage.setItem(STORED, typeof value === 'string' ? value : JSON.stringify(value));
  return storage;
}

/** A review that records each action, on a side the test sets. */
function review(side: Side = 'answer') {
  const acted: Action[] = [];
  return { acted, target: { act: (action: Action) => acted.push(action), side: () => side, focus: () => undefined } };
}

const DEFAULTS = { keys: KEYS, buttons: BUTTONS, stick: STICK };

describe('the mapping', () => {
  it('a stored mapping drives the review, and each mode keeps its default', () => {
    // a keyboard remote that sends j and k: the stored keys replace the default keys
    const keyboard = holding({ keyboard: [['j', 'good'], ['k', 'undo']] });
    const typed = review();
    const keyed = new StudyInput(typed.target, keyboard, new MappingStore(keyboard).mapping);
    keyed.key(key('j'));
    keyed.key(key('3'));
    keyed.key(key('k'));
    // and the gamepad keeps its default: button 15 is Good, the stick's left lean is Again
    keyed.pads([pad()]);
    keyed.pads([pad([15])]);
    keyed.pads([pad([15], [-0.8, 0, 0, 0])]);
    expect(typed.acted).toEqual(['good', 'undo', 'good', 'again']);

    // a gamepad whose Good is button 7 and whose stick is off: the keyboard keeps its default
    const gamepad = holding({ gamepad: [[7, 'good']], stick: false });
    const pressed = review();
    const padded = new StudyInput(pressed.target, gamepad, new MappingStore(gamepad).mapping);
    padded.pads([pad()]);
    padded.pads([pad([15])]);
    padded.pads([pad([7])]);
    padded.pads([pad([], [-0.8, 0, 0, 0])]);
    padded.key(key('1'));
    expect(pressed.acted).toEqual(['good', 'again']);
  });

  it('a change maps one input to one action, and each mode returns to its default apart', () => {
    const storage = new MemoryStorage();
    const store = new MappingStore(storage);
    // the key j now takes Good alone: 3 no longer does
    store.bindKey('good', 'j');
    // and moves to Again, which loses 1
    store.bindKey('again', 'j');
    store.bindButton('good', 7);
    store.bindButton('easy', 7);
    store.bindButton('flag', 0);
    store.setStick(false);
    const keys = [[' ', 'confirm'], ['Enter', 'confirm'], ['2', 'hard'], ['4', 'easy'], ['u', 'undo'], ['-', 'bury'], ['r', 'replay'], ['j', 'again']];
    const buttons = [[1, 'confirm'], [14, 'again'], [13, 'hard'], [4, 'undo'], [5, 'bury'], [2, 'replay'], [7, 'easy'], [0, 'flag']];
    expect(JSON.parse(storage.getItem(STORED) ?? 'null')).toEqual({ keyboard: keys, gamepad: buttons, stick: false });
    expect(store.mapping).toEqual({ keys: new Map(keys), buttons: new Map(buttons), stick: [] });
    // the device reads it back
    expect(new MappingStore(storage).mapping).toEqual(store.mapping);

    // the stick back on is stored as no choice
    store.setStick(true);
    expect(JSON.parse(storage.getItem(STORED) ?? 'null')).toEqual({ keyboard: keys, gamepad: buttons });
    expect(store.mapping.stick).toEqual(STICK);
    store.setStick(false);

    // the keys return to their default and the gamepad keeps its own; then the gamepad, stick included
    store.restore('keyboard');
    expect(JSON.parse(storage.getItem(STORED) ?? 'null')).toEqual({ gamepad: buttons, stick: false });
    expect(store.mapping).toEqual({ keys: KEYS, buttons: new Map(buttons), stick: [] });
    store.restore('gamepad');
    expect(JSON.parse(storage.getItem(STORED) ?? 'null')).toEqual({});
    expect(store.mapping).toEqual(DEFAULTS);
  });

  it('a value the page cannot read is no mapping, and a refused storage keeps a change for the page', () => {
    for (const value of ['not json', 'null', '5', { keyboard: 5 }, { gamepad: 5 }, { stick: 'off' }]) {
      expect(new MappingStore(holding(value)).mapping, JSON.stringify(value)).toEqual(DEFAULTS);
    }
    // a pair whose action the review does not know is dropped, and the rest are kept
    const unknown = holding({ keyboard: [['j', 'leap'], ['k', 'good']], gamepad: [[7, 'leap'], [8, 'bury']] });
    expect(new MappingStore(unknown).mapping).toEqual({
      keys: new Map([['k', 'good']]),
      buttons: new Map([[8, 'bury']]),
      stick: STICK
    });
    expect(new MappingStore(undefined).mapping).toEqual(DEFAULTS);

    for (const storage of [undefined, new RefusingStorage()]) {
      const store = new MappingStore(storage);
      expect(() => store.bindKey('good', 'j')).not.toThrow();
      expect(store.mapping.keys.get('j')).toBe('good');
    }
  });

  it('a capture takes the key or the button a remote sends, never a modifier, Tab or Escape', () => {
    expect(['j', ' ', 'PageDown', 'ArrowRight'].map((name) => keyOf(key(name)))).toEqual(['j', ' ', 'PageDown', 'ArrowRight']);
    // the keys that move focus or cancel, and a modifier alone, map nothing
    for (const name of ['Tab', 'Escape', 'Shift', 'Control', 'Alt', 'Meta']) {
      expect(keyOf(key(name)), name).toBeNull();
    }
    // nor does a key the review's reader would never read
    for (const extra of [{ repeat: true }, { isComposing: true }, { altKey: true }, { ctrlKey: true }, { metaKey: true }]) {
      expect(keyOf(key('j', extra)), JSON.stringify(extra)).toBeNull();
    }

    // a button that rose since the last frame, the first when two rose
    expect(buttonOf(pad(), pad([0]))).toBe(0);
    expect(buttonOf(pad([7]), pad([7, 9, 11]))).toBe(9);
    // a gamepad's first frame is its baseline, a held button did not rise, and a frame with none
    expect(buttonOf(undefined, pad([7]))).toBeNull();
    expect(buttonOf(pad([7]), pad([7]))).toBeNull();
    expect(buttonOf(pad(), pad())).toBeNull();
    // only the standard mapping, the one the review's reader fires for
    expect(buttonOf(pad(), pad([3], [0, 0, 0, 0], 'xinput'))).toBeNull();
  });
});

describe('the mapping screen', () => {
  let gamepads: (PadSnapshot | null)[] = [];
  const frames = new Map<number, FrameRequestCallback>();
  let frameIds = 0;
  let errors: unknown[] = [];
  const caught = (event: ErrorEvent) => errors.push(event.error);

  /** A gamepad as the browser lists it, holding `pressed`. */
  function remote(pressed: number[] = [], mapping = 'standard'): PadSnapshot {
    return { ...pad(pressed, [0, 0, 0, 0], mapping), buttons: pad(pressed).buttons.map((down) => ({ pressed: down })) } as unknown as PadSnapshot;
  }

  /** Runs the animation frame the screen asked for, then lets Svelte update the page. */
  function runFrame(): void {
    const due = [...frames.values()];
    frames.clear();
    for (const callback of due) callback(0);
    flushSync();
  }

  function press(name: string): void {
    fireEvent.keyDown(window, { key: name });
    flushSync();
  }

  function click(name: string): void {
    fireEvent.click(screen.getByRole('button', { name }));
    flushSync();
  }

  function cell(name: string): string {
    return screen.getByRole('button', { name }).textContent?.trim() ?? '';
  }

  function prompt(): string {
    return screen.getByRole('status').textContent?.trim() ?? '';
  }

  function stored(): unknown {
    return JSON.parse(localStorage.getItem(STORED) ?? 'null');
  }

  beforeEach(() => {
    gamepads = [];
    frames.clear();
    frameIds = 0;
    errors = [];
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frameIds += 1;
      frames.set(frameIds, callback);
      return frameIds;
    });
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
    Object.defineProperty(navigator, 'getGamepads', { configurable: true, value: () => gamepads });
    window.addEventListener('error', caught);
    localStorage.clear();
  });

  afterEach(() => {
    window.removeEventListener('error', caught);
    vi.unstubAllGlobals();
    Reflect.deleteProperty(navigator, 'getGamepads');
  });

  it('the mapping screen changes a key and a button by pressing them, and restores each mode', () => {
    render(MappingScreen);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Remote mapping');
    expect(screen.getAllByRole('row').map((row) => row.querySelector('th')?.textContent?.trim())).toEqual([
      'Action',
      'Show answer, then Good',
      'Again',
      'Hard',
      'Good',
      'Easy',
      'Undo',
      'Bury',
      'Flag',
      'Replay'
    ]);
    expect(cell('Change the key for Show answer, then Good: Space, Enter')).toBe('Space, Enter');
    expect(cell('Change the gamepad button for Show answer, then Good: Button 0, Button 1')).toBe('Button 0, Button 1');
    expect(cell('Change the key for Flag: None')).toBe('None');

    // a key: the screen asks for it, and the next key the remote sends takes the action
    click('Change the key for Good: 3');
    expect(prompt()).toBe('Press the new key for Good. Escape cancels.');
    press('j');
    expect(prompt()).toBe('');
    expect(cell('Change the key for Good: j')).toBe('j');
    expect(stored()).toEqual({ keyboard: expect.arrayContaining([['j', 'good']]) });

    // a button: the screen reads the gamepads each frame, the first frame as their baseline
    click('Change the gamepad button for Good: Button 15');
    expect(prompt()).toBe('Press the new gamepad button for Good. Escape cancels.');
    gamepads = [null, remote([7])];
    runFrame();
    expect(prompt()).not.toBe('');
    gamepads = [null, remote([7, 9])];
    runFrame();
    expect(prompt()).toBe('');
    expect(cell('Change the gamepad button for Good: Button 9')).toBe('Button 9');
    expect(frames.size).toBe(0);

    // the keys return to their default, and the gamepad keeps its own; then the gamepad
    click('Restore the default keys');
    expect(cell('Change the key for Good: 3')).toBe('3');
    expect(cell('Change the gamepad button for Good: Button 9')).toBe('Button 9');
    click('Restore the default buttons');
    expect(cell('Change the gamepad button for Good: Button 15')).toBe('Button 15');
    expect(stored()).toEqual({});
    expect(errors).toEqual([]);
  });

  it('a capture waits for its own mode, and Escape or Cancel ends it', () => {
    render(MappingScreen);
    // a key with no capture waiting maps nothing
    press('j');
    expect(cell('Change the key for Again: 1')).toBe('1');

    // Escape ends a capture, and so does Cancel; a key after either maps nothing
    click('Change the key for Again: 1');
    press('Shift');
    expect(prompt()).toBe('Press the new key for Again. Escape cancels.');
    press('Escape');
    expect(prompt()).toBe('');
    click('Change the key for Again: 1');
    click('Cancel');
    expect(prompt()).toBe('');
    press('j');
    expect(cell('Change the key for Again: 1')).toBe('1');

    // a key does not answer a button's capture, and a button does not answer a key's
    click('Change the gamepad button for Again: Button 14');
    press('j');
    expect(prompt()).toBe('Press the new gamepad button for Again. Escape cancels.');
    press('Escape');
    expect(frames.size).toBe(0);
    click('Change the key for Again: 1');
    gamepads = [remote()];
    runFrame();
    gamepads = [remote([7])];
    runFrame();
    expect(prompt()).toBe('Press the new key for Again. Escape cancels.');
    click('Cancel');

    // a gamepad of another mapping is not read, and a capture cancelled stops reading
    click('Change the gamepad button for Again: Button 14');
    gamepads = [remote([], 'xinput')];
    runFrame();
    gamepads = [remote([7], 'xinput')];
    runFrame();
    expect(cell('Change the gamepad button for Again: Button 14')).toBe('Button 14');
    click('Cancel');
    expect(frames.size).toBe(0);
    expect(cell('Change the gamepad button for Again: Button 14')).toBe('Button 14');
    expect(stored()).toBeNull();
    expect(errors).toEqual([]);
  });

  it('the stick and the single-key shortcuts are kept on this device', () => {
    const first = render(MappingScreen);
    const stick = screen.getByRole('checkbox', { name: 'The left stick grades' }) as HTMLInputElement;
    const keys = screen.getByRole('checkbox', { name: 'Single-key shortcuts' }) as HTMLInputElement;
    expect([stick.checked, keys.checked]).toEqual([true, true]);
    fireEvent.click(stick);
    fireEvent.click(keys);
    flushSync();
    expect(stored()).toEqual({ stick: false });
    expect(localStorage.getItem(KEY_SWITCH)).toBe('off');

    // the next visit reads both back
    first.unmount();
    render(MappingScreen);
    const again = screen.getAllByRole('checkbox') as HTMLInputElement[];
    expect(again.map((box) => box.checked)).toEqual([false, false]);
    fireEvent.click(again[0]);
    flushSync();
    expect(stored()).toEqual({});
    expect(errors).toEqual([]);
  });
});
