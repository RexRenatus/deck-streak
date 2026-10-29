/**
 * @vitest-environment jsdom
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CardView, Clip, Faces, FaceView, UndoOffer } from '#lib/engine/protocol.js';
import { Player, type AudioOut, type ObjectUrls } from './audio';
import { Review, type StudyClient } from './review';
import {
  deviceSpeaker,
  NATIVE_DEFAULT_RATE,
  WebSpeaker,
  type SpeechOutcome,
  type Synthesis,
  type Utterance
} from './speech';
import { VOICE_CHOICES, VoiceChoices, type Voice } from './voice';
import VoicePicker from './VoicePicker.svelte';

// SPEC-350 R16, A27; ADR-361 D8, D13; SPEC-506; ADR-524. A speech clip is spoken in the language the
// core gives it, at the core's rate divided by the native default of 0.5 (SPEC-348 P3), with a voice
// the device runs itself: the device's stored voice for that language while the device lists it as
// local, else the language's first local voice. A language with no local voice is not spoken, and the
// review's status region says so. The picker lists the device's local voices for each language the
// card speaks, and stores the choice per language under one local-storage key; a storage the browser
// refuses keeps the choice for the page, and a stored remote choice is ignored and kept.

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

/** A voice as the device lists it; `localService` false is a remote service's voice. */
const voice = (voiceURI: string, lang: string, localService = true): Voice => ({
  voiceURI,
  name: `${voiceURI} voice`,
  lang,
  localService
});
const AMELIE = voice('amelie', 'fr-FR');
const ANNA = voice('anna', 'de-DE');
const MARKUS = voice('markus', 'de-AT');
// some devices join a tag's parts with an underscore
const PETRA = voice('petra', 'de_CH');
const KYOKO = voice('kyoko', 'ja-JP');
const VOICES = [AMELIE, ANNA, MARKUS, PETRA, KYOKO];

// a device whose default Japanese voice is a remote service's, listed first, beside a local one
const REMOTE_JA: Voice & { default: boolean } = { ...voice('remote-ja', 'ja-JP', false), default: true };
const LOCAL_JA = voice('local-ja', 'ja-JP');
const REMOTE_KO = voice('remote-ko', 'ko-KR', false);

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

/** The page's audio element: each sound it played, each ending on its own. */
class FakeAudio implements AudioOut {
  src = '';
  played: string[] = [];
  onended: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onpause: ((event: Event) => void) | null = null;

  async play(): Promise<void> {
    this.played.push(this.src);
    setTimeout(() => this.onended?.(new Event('ended')), 0);
  }

  pause(): void {}
}

const URLS: ObjectUrls = { createObjectURL: () => 'blob:sound', revokeObjectURL: () => undefined };

const speech = (text: string, language: string, rate: number): Clip => ({ kind: 'speech', text, language, rate });
const SOUND: Clip = { kind: 'sound', name: 'a.mp3', type: 'audio/mpeg', bytes: new Uint8Array([1]) };

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
  late: false
};

/** The engine with one card whose faces speak as `faces` gives them. */
class SpeakingClient implements StudyClient {
  constructor(readonly faces: () => Promise<Faces>) {}

  async card() {
    return { counts: { new: 1, learning: 0, review: 0 }, card: CARD };
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

/** A card whose faces speak German, a sound and Japanese. */
async function germanAndJapanese(): Promise<Faces> {
  return {
    question: face([speech('der Hund', 'de-DE', 0.5)], [speech('der Hund', 'de-DE', 0.5), SOUND]),
    answer: face([SOUND, speech('犬', 'ja-JP', 0.5)], [speech('der Hund', 'de-DE', 0.5), speech('犬', 'ja-JP', 0.5)]),
    wanted: []
  };
}

/** A card whose question speaks Japanese and then plays a sound, and whose answer plays nothing. */
async function japaneseThenSound(): Promise<Faces> {
  return { question: face([speech('犬', 'ja-JP', 0.5), SOUND], []), answer: face([], []), wanted: [] };
}

/**
 * The planted speaker build: the voice choices with their one read of `localService` taken out, so
 * they list every voice of the language and fall back to its first, the remote one included. Each
 * check below must fail on it.
 */
class IgnoresLocalService extends VoiceChoices {
  override voices<V extends Voice>(all: readonly V[], tag: string): V[] {
    return all.filter((each) => each.lang.split(/[-_]/)[0] === tag.split(/[-_]/)[0]);
  }
}

/** A speaker over `voices` and `choices`, and the outcome it recorded for each clip, by language. */
function recordingSpeaker(voices: Voice[], choices: VoiceChoices) {
  const synthesis = new FakeSynthesis(voices);
  const outcomes: [string, SpeechOutcome][] = [];
  const speaker = new WebSpeaker(
    synthesis,
    choices,
    (text) => new FakeUtterance(text),
    (clip, outcome) => outcomes.push([clip.language, outcome])
  );
  return { synthesis, outcomes, speaker };
}

/** remote_voice_never_speaks's check: on a device whose default Japanese voice is remote, a Japanese
 * clip speaks with the local one, and its outcome is recorded. */
async function speaksLocally(choices: VoiceChoices): Promise<void> {
  const { synthesis, outcomes, speaker } = recordingSpeaker([REMOTE_JA, LOCAL_JA], choices);
  await speaker.speak({ kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.5 });
  expect(said(synthesis)).toEqual([['犬', 'ja-JP', 1, LOCAL_JA]]);
  expect(outcomes).toEqual([['ja-JP', 'local-voice']]);
}

/** no_local_voice_says_so's check: on a device with only a remote Japanese voice, a Japanese clip is
 * not spoken, and its outcome is recorded. */
async function staysSilent(choices: VoiceChoices): Promise<void> {
  const { synthesis, outcomes, speaker } = recordingSpeaker([REMOTE_JA], choices);
  await speaker.speak({ kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.5 });
  expect(said(synthesis)).toEqual([]);
  expect(outcomes).toEqual([['ja-JP', 'no-local-voice']]);
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** web/app/src, where the census reads the page's own code. */
const SRC = resolve(import.meta.dirname, '../..');

/** Every source file the page ships under web/app/src, by its path from there, with its text: test
 * files and the generated messages are left out. */
function sources(dir = SRC): [string, string][] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry): [string, string][] => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : sources(path);
    if (!/\.(ts|js|svelte)$/.test(entry.name) || /\.(test|spec)\.(ts|js)$/.test(entry.name)) return [];
    return [[relative(SRC, path).split(sep).join('/'), readFileSync(path, 'utf8')]];
  });
}

/** The speak calls the page may make, each once: the synthesis inside `WebSpeaker`, and the player
 * asking its speaker. */
const SPEAK_SITES = ['lib/study/speech.ts: this.#synthesis.speak(', 'lib/study/audio.ts: this.#speaker.speak('];

/** What the census finds in `files`: a speak call it does not list, a listed one missing or held
 * twice, and an utterance made outside `speech.ts`. */
function speakSites(files: [string, string][]): string[] {
  const found: string[] = [];
  const seen: string[] = [];
  for (const [path, text] of files) {
    for (const match of text.matchAll(/([\w$#.?]*)\.\s*speak\s*\(|\[\s*['"`]speak['"`]\s*\]\s*\(/g)) {
      const site = `${path}: ${match[1] ?? ''}${match[1] === undefined ? match[0] : '.speak('}`;
      if (SPEAK_SITES.includes(site)) seen.push(site);
      else found.push(`${site} is not a WebSpeaker call`);
    }
    if (/new\s+SpeechSynthesisUtterance\b/.test(text) && path !== 'lib/study/speech.ts') {
      found.push(`${path}: makes an utterance outside speech.ts`);
    }
  }
  for (const site of SPEAK_SITES) {
    const count = seen.filter((each) => each === site).length;
    if (count !== 1) found.push(`${site} is held ${count} times, not once`);
  }
  return found;
}

/** web/app/tests, where the browser audits live. */
const TESTS = resolve(import.meta.dirname, '../../../tests');

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('the voice', () => {
  it("a speech clip's voice is the stored choice, else the language's default", () => {
    const choices = new VoiceChoices(stored(JSON.stringify({ 'de-DE': 'markus', 'fr-FR': 'gone' })));

    // the stored choice for the clip's language
    expect(choices.voiceFor(VOICES, 'de-DE')).toBe(MARKUS);
    // a language with no choice, and a choice the device no longer has, take the language's first
    // local voice, its default on this device
    expect(choices.voiceFor(VOICES, 'ja-JP')).toBe(KYOKO);
    expect(choices.voiceFor(VOICES, 'fr-FR')).toBe(AMELIE);
    // a choice is its language's alone: de-AT has none, so it takes the first local German voice
    expect(choices.voiceFor(VOICES, 'de-AT')).toBe(ANNA);

    // the device's voices for a language are those of its language subtag, in the device's order
    expect(choices.voices(VOICES, 'de-DE')).toEqual([ANNA, MARKUS, PETRA]);
    expect(choices.voices(VOICES, 'ja-JP')).toEqual([KYOKO]);
    expect(choices.voices(VOICES, 'ko-KR')).toEqual([]);

    // a device with no storage, none stored, and a stored value it cannot read hold no choice
    for (const storage of [undefined, new MemoryStorage(), stored('{'), stored('"markus"')]) {
      expect(new VoiceChoices(storage).voiceFor(VOICES, 'de-DE')).toBe(ANNA);
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
      ['犬', 'ja-JP', 1.5, KYOKO]
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
      async () => new SpeakingClient(germanAndJapanese),
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

    // a choice is stored for its language, beside the others; the default is a choice too, and
    // speaks with the language's first local voice
    const [german, japanese] = screen.getAllByRole('combobox');
    await fireEvent.change(german, { target: { value: 'markus' } });
    await fireEvent.change(japanese, { target: { value: '' } });
    expect(JSON.parse(storage.getItem(VOICE_CHOICES) ?? '')).toEqual({ 'ja-JP': '', 'de-DE': 'markus' });
    expect([choices.voiceFor(VOICES, 'de-DE'), choices.voiceFor(VOICES, 'ja-JP')]).toEqual([MARKUS, KYOKO]);

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

describe('only voices that run on the device speak', () => {
  it('a voice is local only when the device marks it local', () => {
    const choices = new VoiceChoices(undefined);
    // a remote voice, a voice the device lists with no field, and one whose field is not the boolean
    const unmarked = { voiceURI: 'unmarked-ja', name: 'unmarked-ja voice', lang: 'ja-JP' } as Voice;
    const worded = { ...voice('worded-ja', 'ja-JP'), localService: 'true' } as unknown as Voice;
    expect(choices.voices([REMOTE_JA, unmarked, worded, LOCAL_JA], 'ja-JP')).toEqual([LOCAL_JA]);
    expect(choices.voiceFor([REMOTE_JA, unmarked, worded], 'ja-JP')).toBeNull();
    // every voice the device marks local counts, in the device's order
    expect(choices.voices(VOICES, 'de-DE')).toEqual([ANNA, MARKUS, PETRA]);
  });

  it('the picker lists only the local voices of the language', () => {
    const choices = new VoiceChoices(new MemoryStorage());
    const device = [REMOTE_JA, LOCAL_JA, REMOTE_KO, AMELIE];
    expect(choices.voices(device, 'ja-JP')).toEqual([LOCAL_JA]);
    expect(choices.voices(device, 'ko-KR')).toEqual([]);

    // the language's default first, then the local voices alone; a language with only a remote
    // voice offers the default alone
    render(VoicePicker, { choices, languages: ['ja-JP', 'ko-KR'], synthesis: new FakeSynthesis(device) });
    expect(pickers()).toEqual([
      ['Voice for ja-JP', ['Default voice', 'local-ja voice'], 0],
      ['Voice for ko-KR', ['Default voice'], 0]
    ]);
  });

  it('a clip voice is the stored local choice else the first local voice else none', () => {
    const SECOND_JA = voice('second-ja', 'ja-JP');
    const device = [REMOTE_JA, LOCAL_JA, SECOND_JA, REMOTE_KO, AMELIE];

    // no choice: the language's first local voice, though the browser's default is remote
    const none = new VoiceChoices(undefined);
    expect(none.voiceFor(device, 'ja-JP')).toBe(LOCAL_JA);
    // only a remote voice for the language, or no voice at all: none
    expect(none.voiceFor(device, 'ko-KR')).toBeNull();
    expect(none.voiceFor(device, 'de-DE')).toBeNull();

    const choices = new VoiceChoices(
      stored(JSON.stringify({ 'ja-JP': 'second-ja', 'fr-FR': 'gone', 'ko-KR': 'remote-ko' }))
    );
    // the stored local choice, which the picker shows chosen
    expect([choices.voiceFor(device, 'ja-JP'), choices.chosen(device, 'ja-JP')]).toEqual([SECOND_JA, SECOND_JA]);
    // a choice the device no longer has: the first local voice, and the picker shows the default
    expect([choices.voiceFor(device, 'fr-FR'), choices.chosen(device, 'fr-FR')]).toEqual([AMELIE, null]);
    // a stored remote choice, and no local voice: none
    expect([choices.voiceFor(device, 'ko-KR'), choices.chosen(device, 'ko-KR')]).toEqual([null, null]);

    // a choice stored for another language's voice does not cross languages
    const crossed = new VoiceChoices(stored(JSON.stringify({ 'ja-JP': 'amelie' })));
    expect([crossed.voiceFor(device, 'ja-JP'), crossed.chosen(device, 'ja-JP')]).toEqual([LOCAL_JA, null]);
  });

  it('a stored remote choice is ignored and kept', async () => {
    const storage = stored(JSON.stringify({ 'ja-JP': 'remote-ja' }));
    const choices = new VoiceChoices(storage);
    const device = [REMOTE_JA, LOCAL_JA];

    // the stored remote voice never speaks: the local one does
    expect(choices.voiceFor(device, 'ja-JP')).toBe(LOCAL_JA);
    const synthesis = new FakeSynthesis(device);
    const speaker = new WebSpeaker(synthesis, choices, (text) => new FakeUtterance(text));
    await speaker.speak({ kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.5 });
    expect(said(synthesis)).toEqual([['犬', 'ja-JP', 1, LOCAL_JA]]);

    // the picker shows the default chosen and lists no remote voice
    render(VoicePicker, { choices, languages: ['ja-JP'], synthesis: new FakeSynthesis(device) });
    expect(pickers()).toEqual([['Voice for ja-JP', ['Default voice', 'local-ja voice'], 0]]);

    // nothing deleted or rewritten: the storage holds the choice as it was written
    expect([...storage.items.entries()]).toEqual([[VOICE_CHOICES, '{"ja-JP":"remote-ja"}']]);

    // the choice applies again once the device lists that voice as local
    const installed = voice('remote-ja', 'ja-JP');
    expect(choices.voiceFor([installed, LOCAL_JA], 'ja-JP')).toBe(installed);
  });

  it('no_local_voice_says_so', async () => {
    // a device with only a remote Japanese voice: the clip is not spoken, and its outcome is recorded
    await staysSilent(new VoiceChoices(new MemoryStorage()));
    await staysSilent(new VoiceChoices(stored(JSON.stringify({ 'ja-JP': 'remote-ja' }))));

    // a browser with no speech synthesis speaks nothing, and records the same outcome
    const outcomes: [string, SpeechOutcome][] = [];
    const silent = deviceSpeaker(new VoiceChoices(undefined), (clip, outcome) => outcomes.push([clip.language, outcome]));
    await silent.speak({ kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.5 });
    expect(outcomes).toEqual([['ja-JP', 'no-local-voice']]);

    // the clip settles at once, so the sound after it plays; the review's status region names the
    // language until the learner's next gesture
    const synthesis = new FakeSynthesis([REMOTE_JA]);
    const audio = new FakeAudio();
    let review: Review | undefined;
    const speaker = new WebSpeaker(
      synthesis,
      new VoiceChoices(undefined),
      (text) => new FakeUtterance(text),
      (clip, outcome) => {
        if (outcome === 'no-local-voice') review?.unvoiced(clip.language);
      }
    );
    const player = new Player(audio, URLS, speaker);
    const played: boolean[] = [];
    review = new Review(
      async () => new SpeakingClient(japaneseThenSound),
      () => 0,
      () => undefined,
      {
        play: async (clips) => {
          const done = await player.play(clips);
          played.push(done);
          return done;
        }
      }
    );
    review.start();
    await review.settled();
    await vi.waitFor(() => expect(played).toEqual([true]));
    expect([said(synthesis), audio.played, review.statusLine]).toEqual([
      [],
      ['blob:sound'],
      'No voice on this device for ja-JP.'
    ]);
    review.act('show-answer');
    expect(review.statusLine).toBeNull();

    // planted: a speaker build that ignores localService speaks the clip with the browser's default
    await expect(staysSilent(new IgnoresLocalService(undefined))).rejects.toThrow();
  });

  it('remote_voice_never_speaks', async () => {
    // the browser's default for Japanese is the remote voice, listed first; with no choice, and with a
    // stored remote choice, the local one speaks
    await speaksLocally(new VoiceChoices(new MemoryStorage()));
    await speaksLocally(new VoiceChoices(stored(JSON.stringify({ 'ja-JP': 'remote-ja' }))));

    // planted: a speaker build that ignores localService turns this check and no_local_voice_says_so's
    // red, with no choice and with the remote one stored
    await expect(speaksLocally(new IgnoresLocalService(undefined))).rejects.toThrow();
    await expect(speaksLocally(new IgnoresLocalService(stored(JSON.stringify({ 'ja-JP': 'remote-ja' }))))).rejects.toThrow();
    await expect(staysSilent(new IgnoresLocalService(undefined))).rejects.toThrow();
    await expect(staysSilent(new IgnoresLocalService(stored(JSON.stringify({ 'ja-JP': 'remote-ja' }))))).rejects.toThrow();
  });

  it('speak_sites_census', () => {
    const files = examined('source files under web/app/src', sources());
    expect(speakSites(files)).toEqual([]);

    // planted: a screen that speaks through the browser's synthesis itself is reported by name
    const screenFile: [string, string] = [
      'lib/study/Planted.svelte',
      "<script>speechSynthesis.speak(new SpeechSynthesisUtterance('犬'));</script>"
    ];
    expect(speakSites([...files, screenFile])).toEqual([
      'lib/study/Planted.svelte: speechSynthesis.speak( is not a WebSpeaker call',
      'lib/study/Planted.svelte: makes an utterance outside speech.ts'
    ]);

    // planted: a second call in speech.ts, around the speaker, and one by bracket
    const around = files.map(([path, text]): [string, string] =>
      path === 'lib/study/speech.ts' ? [path, `${text}\nglobalThis.speechSynthesis?.speak(utterance);\n`] : [path, text]
    );
    expect(speakSites(around)).toEqual(['lib/study/speech.ts: globalThis.speechSynthesis?.speak( is not a WebSpeaker call']);
    const bracket: [string, string] = ['lib/Planted.ts', "window.speechSynthesis['speak'](utterance);"];
    expect(speakSites([...files, bracket])).toEqual(["lib/Planted.ts: ['speak']( is not a WebSpeaker call"]);

    // planted: a tree whose speaker no longer reaches the synthesis is reported, so the census
    // cannot pass on a tree it did not read
    const unread = files.filter(([path]) => path !== 'lib/study/speech.ts');
    expect(speakSites(unread)).toEqual(['lib/study/speech.ts: this.#synthesis.speak( is held 0 times, not once']);
  });

  it('the picker accessibility audit runs axe in both colour schemes', () => {
    // SPEC-506 A6's browser leg is a Playwright spec, which the tdd probe does not resolve; this test
    // holds its shape, as a11y-coverage.test.ts holds the app's audit
    const spec = readFileSync(join(TESTS, 'voice-picker-a11y.spec.ts'), 'utf8');
    const audit = readFileSync(join(TESTS, 'a11y.spec.ts'), 'utf8');

    // each palette of the palette module, painted and emulated
    expect(spec).toMatch(/import\s*\{\s*THEMES\s*\}\s*from\s*'\.\/telegram-palettes'/);
    expect(spec).toMatch(/for\s*\(\s*const\s+\[\s*scheme\s*,\s*themeParams\s*\]\s+of\s+Object\.entries\(\s*THEMES\s*\)\s*\)/);
    expect(spec).toMatch(/emulateMedia\(\s*\{\s*colorScheme:\s*scheme/);

    // the device lists a local and a remote Japanese voice and a remote Korean one, so the picker
    // lists a local voice and the line names Korean
    expect(spec).toMatch(/lang:\s*'ja-JP',\s*localService:\s*true/);
    expect(spec).toMatch(/lang:\s*'ja-JP',\s*localService:\s*false/);
    expect(spec).toMatch(/lang:\s*'ko-KR',\s*localService:\s*false/);

    // one visit, to the review; the picker and the line show before axe runs, with the app's tags,
    // and no violation is allowed
    expect(spec.match(/page\.goto\(/g) ?? []).toHaveLength(1);
    const at = (text: string) => spec.indexOf(text);
    const order = [
      "page.goto('/study/review')",
      "name: 'Voice for ja-JP'",
      "'No voice on this device for ko-KR.'",
      '.analyze()',
      'expect(results.violations).toEqual([])'
    ].map(at);
    expect(order.every((index) => index >= 0)).toBe(true);
    expect(order).toEqual([...order].sort((left, right) => left - right));
    const tags = (text: string) => /const WCAG_22_AA = (\[[^\]]*\]);/.exec(text)?.[1];
    expect(tags(spec)).toBeDefined();
    expect(tags(spec)).toBe(tags(audit));
    expect(spec).toMatch(/new AxeBuilder\(\{\s*page\s*\}\)\s*\.withTags\(WCAG_22_AA\)/);
  });
});
