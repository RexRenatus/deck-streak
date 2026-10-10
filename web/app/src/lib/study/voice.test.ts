/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CardView, Clip, Faces, FaceView, UndoOffer } from '$lib/engine/protocol';
import { Review, type StudyClient } from './review';
import { deviceSpeaker, NATIVE_DEFAULT_RATE, WebSpeaker, type Synthesis, type Utterance } from './speech';
import { VOICE_CHOICES, VoiceChoices, type Voice } from './voice';
import VoicePicker from './VoicePicker.svelte';

// SPEC-350 R16, A27; ADR-361 D8, D13. A speech clip is spoken in the language the core gives it, at
// the core's rate divided by the native default of 0.5 (SPEC-348 P3), with the device's stored voice
// for that language, else the language's default voice: no voice, which the browser reads as that
// language's default. The picker lists the device's voices for each language the card speaks, and
// stores the choice per language under one local-storage key; a storage the browser refuses keeps
// the choice for the page.

/** A device's local storage, in memory. */
class MemoryStorage {
  readonly items = new Map<string, string>();

  getItem(key: string): string | null {
    return this.items.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.items.set(key, value);
  }
}

/** A storage the browser refuses to write. */
class RefusingStorage extends MemoryStorage {
  override setItem(): void {
    throw new DOMException('the storage is full', 'QuotaExceededError');
  }
}

function stored(text: string): MemoryStorage {
  const storage = new MemoryStorage();
  storage.setItem(VOICE_CHOICES, text);
  return storage;
}

const voice = (voiceURI: string, lang: string): Voice => ({ voiceURI, name: `${voiceURI} voice`, lang });
const AMELIE = voice('amelie', 'fr-FR');
const ANNA = voice('anna', 'de-DE');
const MARKUS = voice('markus', 'de-AT');
// some devices join a tag's parts with an underscore
const PETRA = voice('petra', 'de_CH');
const KYOKO = voice('kyoko', 'ja-JP');
const VOICES = [AMELIE, ANNA, MARKUS, PETRA, KYOKO];

class FakeUtterance implements Utterance {
  lang = '';
  rate = 1;
  voice: Voice | null = null;
  onend: ((event: SpeechSynthesisEvent) => void) | null = null;
  onerror: ((event: SpeechSynthesisErrorEvent) => void) | null = null;

  constructor(readonly text: string) {}
}

/** The browser's speech synthesis: each utterance it was given, which ends on its own or fails,
 * each cancel, and each listener of its voice list. */
class FakeSynthesis implements Synthesis {
  spoken: FakeUtterance[] = [];
  log: string[] = [];
  failing = false;
  listeners: (() => void)[] = [];

  constructor(public voices: Voice[]) {}

  getVoices(): Voice[] {
    return this.voices;
  }

  speak(utterance: FakeUtterance): void {
    this.spoken.push(utterance);
    setTimeout(() => {
      if (this.failing) utterance.onerror?.(new Event('error') as SpeechSynthesisErrorEvent);
      else utterance.onend?.(new Event('end') as SpeechSynthesisEvent);
    }, 0);
  }

  cancel(): void {
    this.log.push('cancel');
  }

  addEventListener(type: 'voiceschanged', listener: () => void): void {
    this.log.push(`listen ${type}`);
    this.listeners.push(listener);
  }

  removeEventListener(type: 'voiceschanged', listener: () => void): void {
    this.log.push(`forget ${type}`);
    this.listeners = this.listeners.filter((held) => held !== listener);
  }
}

const speech = (text: string, language: string, rate: number): Clip => ({ kind: 'speech', text, language, rate });

/** What each utterance was: its text, language, rate and voice. */
function said(synthesis: FakeSynthesis): [string, string, number, Voice | null][] {
  return synthesis.spoken.map((utterance) => [utterance.text, utterance.lang, utterance.rate, utterance.voice]);
}

/** Each picker on the page: its label, its options, and the option it shows chosen. */
function pickers(): [string, string[], number][] {
  return screen.getAllByRole('combobox').map((element) => {
    const select = element as HTMLSelectElement;
    const label = [...(select.closest('label')?.childNodes ?? [])].filter((node) => node !== select);
    return [
      label.map((node) => node.textContent).join('').trim(),
      [...select.options].map((option) => option.textContent?.trim() ?? ''),
      select.selectedIndex
    ];
  });
}

function face(autoplay: Clip[], replay: Clip[]): FaceView {
  return { text: '<p>a face</p>', css: '', autoplay, replay, omitted: [] };
}

const CARD: CardView = {
  id: 1n,
  ordinal: 0,
  flag: 0,
  question: '<p>question</p>',
  answer: '<p>answer</p>',
  css: '',
  labels: ['<1m', '<6m', '<10m', '4d'],
  undo: null,
  late: false,
  withheld: false
};

/** The engine with one card whose faces speak German, a sound and Japanese. */
class SpeakingClient implements StudyClient {
  async card() {
    return { counts: { new: 1, learning: 0, review: 0 }, card: CARD };
  }
  async faces(): Promise<Faces> {
    const sound: Clip = { kind: 'sound', name: 'a.mp3', type: 'audio/mpeg', bytes: new Uint8Array([1]) };
    return {
      question: face([speech('der Hund', 'de-DE', 0.5)], [speech('der Hund', 'de-DE', 0.5), sound]),
      answer: face([sound, speech('犬', 'ja-JP', 0.5)], [speech('der Hund', 'de-DE', 0.5), speech('犬', 'ja-JP', 0.5)]),
      wanted: []
    };
  }
  async rate(): Promise<null> {
    return null;
  }
  async bury(): Promise<null> {
    return null;
  }
  async flag(): Promise<number> {
    return 1;
  }
  async undo(): Promise<null> {
    return null;
  }
  async undoOffer(): Promise<UndoOffer> {
    return { offer: null, why: 'none' };
  }
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('the voice', () => {
  it("a speech clip's voice is the stored choice, else the language's default", () => {
    const choices = new VoiceChoices(stored(JSON.stringify({ 'de-DE': 'markus', 'fr-FR': 'gone' })));

    // the stored choice for the clip's language
    expect(choices.voiceFor(VOICES, 'de-DE')).toBe(MARKUS);
    // a language with no choice, and a choice the device no longer has, take the language's default
    expect(choices.voiceFor(VOICES, 'ja-JP')).toBeNull();
    expect(choices.voiceFor(VOICES, 'fr-FR')).toBeNull();
    // a choice is its language's alone
    expect(choices.voiceFor(VOICES, 'de-AT')).toBeNull();

    // the device's voices for a language are those of its language subtag, in the device's order
    expect(choices.voices(VOICES, 'de-DE')).toEqual([ANNA, MARKUS, PETRA]);
    expect(choices.voices(VOICES, 'ja-JP')).toEqual([KYOKO]);
    expect(choices.voices(VOICES, 'ko-KR')).toEqual([]);

    // a device with no storage, none stored, and a stored value it cannot read hold no choice
    for (const storage of [undefined, new MemoryStorage(), stored('{'), stored('"markus"')]) {
      expect(new VoiceChoices(storage).voiceFor(VOICES, 'de-DE')).toBeNull();
    }
  });

  it("a speech clip is spoken in its language at the web's rate", async () => {
    const synthesis = new FakeSynthesis(VOICES);
    const choices = new VoiceChoices(stored(JSON.stringify({ 'de-DE': 'anna' })));
    const speaker = new WebSpeaker(synthesis, choices, (text) => new FakeUtterance(text));

    await speaker.speak({ kind: 'speech', text: 'der Hund', language: 'de-DE', rate: 0.5 });
    await speaker.speak({ kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.75 });
    // the web's normal rate is 1 and the native platform's 0.5, so 0.5 speaks at 1 (SPEC-348 P3)
    expect(said(synthesis)).toEqual([
      ['der Hund', 'de-DE', 1, ANNA],
      ['犬', 'ja-JP', 1.5, null]
    ]);
    expect(NATIVE_DEFAULT_RATE).toBe(0.5);

    // an utterance that fails settles too, so the clips after it still play
    synthesis.failing = true;
    await speaker.speak({ kind: 'speech', text: 'die Katze', language: 'de-DE', rate: 0.5 });
    expect(synthesis.spoken).toHaveLength(3);
    speaker.cancel();
    expect(synthesis.log).toEqual(['cancel']);

    // a browser with no speech synthesis speaks nothing, and the clips after it still play
    const silent = deviceSpeaker(choices);
    await expect(silent.speak({ kind: 'speech', text: 'die Maus', language: 'de-DE', rate: 0.5 })).resolves.toBeUndefined();
    silent.cancel();

    // a browser with one speaks through it
    synthesis.failing = false;
    vi.stubGlobal('speechSynthesis', synthesis);
    vi.stubGlobal('SpeechSynthesisUtterance', FakeUtterance);
    const device = deviceSpeaker(choices);
    await device.speak({ kind: 'speech', text: 'das Pferd', language: 'de-DE', rate: 1 });
    device.cancel();
    expect(said(synthesis).slice(3)).toEqual([['das Pferd', 'de-DE', 2, ANNA]]);
    expect(synthesis.log).toEqual(['cancel', 'cancel']);
  });

  it('the picker stores the choice per language', async () => {
    // the languages the card speaks, each once, in the order its faces give them
    const review = new Review(
      async () => new SpeakingClient(),
      () => 0,
      () => undefined
    );
    expect(review.languages).toEqual([]);
    review.start();
    await review.settled();
    expect(review.languages).toEqual(['de-DE', 'ja-JP']);

    const storage = stored(JSON.stringify({ 'ja-JP': 'kyoko' }));
    const choices = new VoiceChoices(storage);
    const synthesis = new FakeSynthesis([AMELIE, KYOKO]);
    const picker = render(VoicePicker, { choices, languages: [...review.languages, 'ko-KR'], synthesis });

    // one picker per language, the language's default first, then the device's voices for it
    expect(pickers()).toEqual([
      ['Voice for de-DE', ['Default voice'], 0],
      ['Voice for ja-JP', ['Default voice', 'kyoko voice'], 1],
      ['Voice for ko-KR', ['Default voice'], 0]
    ]);

    // the device lists more voices once it has loaded them
    synthesis.voices = VOICES;
    for (const listener of synthesis.listeners) listener();
    flushSync();
    expect(pickers()[0]).toEqual(['Voice for de-DE', ['Default voice', 'anna voice', 'markus voice', 'petra voice'], 0]);

    // a choice is stored for its language, beside the others; the default is a choice too
    const [german, japanese] = screen.getAllByRole('combobox');
    await fireEvent.change(german, { target: { value: 'markus' } });
    await fireEvent.change(japanese, { target: { value: '' } });
    expect(JSON.parse(storage.getItem(VOICE_CHOICES) ?? '')).toEqual({ 'ja-JP': '', 'de-DE': 'markus' });
    expect([choices.voiceFor(VOICES, 'de-DE'), choices.voiceFor(VOICES, 'ja-JP')]).toEqual([MARKUS, null]);

    // the picker stops following the device's voices when it closes
    picker.unmount();
    expect(synthesis.log).toEqual(['listen voiceschanged', 'forget voiceschanged']);
    expect(synthesis.listeners).toEqual([]);

    // a browser with no speech synthesis lists the defaults alone
    render(VoicePicker, { choices, languages: ['fr-FR'], synthesis: undefined });
    expect(pickers()).toEqual([['Voice for fr-FR', ['Default voice'], 0]]);

    // a storage the browser refuses keeps the choice for the page
    const refused = new VoiceChoices(new RefusingStorage());
    refused.choose('de-DE', 'petra');
    expect(refused.voiceFor(VOICES, 'de-DE')).toBe(PETRA);
  });

  it('the voice choices are kept under their own name on this device', () => {
    // the name is written here, so a change to it reddens this test
    const storage = new MemoryStorage();
    new VoiceChoices(storage).choose('de-DE', 'markus');
    expect([...storage.items.keys()]).toEqual(['deck-streak.study.voices']);
    expect(JSON.parse(storage.getItem('deck-streak.study.voices') ?? '')).toEqual({ 'de-DE': 'markus' });
    expect(new VoiceChoices(storage).voiceFor(VOICES, 'de-DE')).toBe(MARKUS);
  });
});
