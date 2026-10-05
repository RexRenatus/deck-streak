import { describe, expect, it, vi } from 'vitest';
import type { Action, Side } from '$lib/remote/actions';
import type { PadSnapshot } from '$lib/remote/gamepad';
import type { KeyInput } from '$lib/remote/keys';
import { deviceStorage, KEY_SWITCH, StudyInput, type SwitchStorage } from './input';

// SPEC-350 R8, A14, A15; ADR-361. Every source reaches the review's one handler through #663's
// modules; a switch kept per device silences the single-character keys and nothing else (WCAG
// 2.1.4); and focus returns to the review after an action, and when a pointer moved it into the
// card frame, never when a Tab did.
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

function pad(pressed: number[] = [], axes: number[] = [0, 0, 0, 0]): PadSnapshot {
  return {
    index: 0,
    id: 'a standard gamepad',
    mapping: 'standard',
    buttons: Array.from({ length: 17 }, (_, at) => pressed.includes(at)),
    axes
  };
}

class MemoryStorage implements SwitchStorage {
  items = new Map<string, string>();
  getItem(name: string): string | null {
    return this.items.get(name) ?? null;
  }
  setItem(name: string, value: string): void {
    this.items.set(name, value);
  }
}

/** A review that records each action and each return of focus, on a side the test sets. */
function target(side: Side = 'answer') {
  const record = { side, acted: [] as Action[], focused: 0 };
  return {
    record,
    target: {
      act: (action: Action) => record.acted.push(action),
      side: () => record.side,
      focus: () => record.focused++
    }
  };
}

const CHARACTER_KEYS = ['1', '2', '3', '4', 'u', '-', 'r', ' '];

describe('the review input', () => {
  it('every source reaches one action', () => {
    const { record, target: review } = target('question');
    const input = new StudyInput(review, new MemoryStorage());

    // a key, read against the side: Space shows the answer, then 3 is Good
    input.key(key(' '));
    record.side = 'answer';
    input.key(key('3'));
    // a gamepad: its first snapshot is a baseline, then button 15 rises (Good) and the stick leans left (Again)
    input.pads([pad()]);
    input.pads([pad([15])]);
    input.pads([pad([15], [-0.8, 0, 0, 0])]);
    // a click names its action on either side
    record.side = 'question';
    input.click('bury');
    // and the flag's key, with Control
    record.side = 'answer';
    input.key(key('1', { ctrlKey: true }));

    expect(record.acted).toEqual(['show-answer', 'good', 'good', 'again', 'bury', 'flag']);

    // a grade on the question side fires nothing, as the remote's reviewer has it
    record.side = 'question';
    input.key(key('4'));
    expect(record.acted).toHaveLength(6);
  });

  it('the key switch silences single-character keys', () => {
    const storage = new MemoryStorage();
    storage.setItem(KEY_SWITCH, 'off');
    const { record, target: review } = target('answer');
    const input = new StudyInput(review, storage);

    // switched off on this device: every single-character key fires nothing
    for (const name of CHARACTER_KEYS) input.key(key(name));
    expect(record.acted).toEqual([]);
    // Enter, the flag's Control 1, a gamepad and a click still act
    input.key(key('Enter'));
    input.key(key('1', { ctrlKey: true }));
    input.pads([pad()]);
    input.pads([pad([13])]);
    input.click('easy');
    expect(record.acted).toEqual(['good', 'flag', 'hard', 'easy']);
    expect(input.characterKeys).toBe(false);

    // switched on, kept under the one key, and read back by the next screen on this device
    input.characterKeys = true;
    expect([...storage.items]).toEqual([[KEY_SWITCH, 'on']]);
    record.acted = [];
    for (const name of CHARACTER_KEYS) input.key(key(name));
    expect(record.acted).toEqual(['again', 'hard', 'good', 'easy', 'undo', 'bury', 'replay', 'good']);
    expect(new StudyInput(review, storage).characterKeys).toBe(true);
    input.characterKeys = false;
    expect([...storage.items]).toEqual([[KEY_SWITCH, 'off']]);
    expect(new StudyInput(review, storage).characterKeys).toBe(false);

    // a device with nothing stored has them on, and one whose storage refuses keeps the page's choice
    expect(new StudyInput(review, new MemoryStorage()).characterKeys).toBe(true);
    expect(new StudyInput(review, undefined).characterKeys).toBe(true);
    const refusing = new StudyInput(review, {
      getItem: () => null,
      setItem: () => {
        throw new Error('the storage is full');
      }
    });
    refusing.characterKeys = false;
    expect(refusing.characterKeys).toBe(false);
  });

  it('focus returns to the review', () => {
    const { record, target: review } = target('answer');
    const input = new StudyInput(review, new MemoryStorage());
    const frame = { tagName: 'IFRAME' };

    // a pointer moved focus into the card frame: the review takes it back
    input.pointer();
    input.blur(frame, frame);
    expect(record.focused).toBe(1);
    // a Tab moved it there: it stays
    input.key(key('Tab'));
    input.blur(frame, frame);
    expect(record.focused).toBe(1);
    // focus that left for anything but the frame, or with no frame shown, stays where it went
    input.pointer();
    input.blur({ tagName: 'BUTTON' }, frame);
    input.blur(null, null);
    expect(record.focused).toBe(1);

    // after an action by key, by gamepad and by click, focus returns to the review
    input.key(key('2'));
    input.pads([pad()]);
    input.pads([pad([12])]);
    input.click('flag');
    expect([record.acted, record.focused]).toEqual([['hard', 'easy', 'flag'], 4]);
    // a key that fires nothing moves nothing
    input.key(key('x'));
    expect(record.focused).toBe(4);
  });

  // Ruling 317 OQ2. A page in a frame can be refused its storage, and reading `localStorage` then
  // throws: the switch is kept for the page alone, as with no storage at all.
  it("the device's storage is the browser's, or none where reading it throws", () => {
    const memory = new MemoryStorage();
    vi.stubGlobal('localStorage', memory);
    try {
      expect(deviceStorage()).toBe(memory);
      Object.defineProperty(globalThis, 'localStorage', {
        configurable: true,
        get: () => {
          throw new DOMException('the storage is refused', 'SecurityError');
        }
      });
      expect(deviceStorage()).toBeUndefined();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
