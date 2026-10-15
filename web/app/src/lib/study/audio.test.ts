import { describe, expect, it } from 'vitest';
import type { CardView, Clip, Faces, FaceView, Head, UndoOffer } from '$lib/engine/protocol';
import { Player, type AudioOut, type ObjectUrls, type Speaker, type SpeechClip } from './audio';
import { Review, type StudyClient } from './review';

// SPEC-350 R15, A26; ADR-361 D12. The page plays a face's clips in the core's order: its autoplay
// clips when a new card shows its question and when the answer is revealed, and its replay clips on
// the Replay control or the remote's replay, on either side. A sound plays from a page URL made from
// its bytes and the type the core gave it, revoked once it is played; speech goes to the speaker;
// nothing reaches the card frame. A play the browser blocks stops the sequence and leaves the
// Replay control.

/** Everything the page's audio and speech did, in order. */
type Log = string[];

/** The page's audio element: each play of its source, each pause, and a play the browser may block.
 * A played source ends on its own, unless the test holds it. */
class FakeAudio implements AudioOut {
  src = '';
  blocked = false;
  held = false;
  onended: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onpause: ((event: Event) => void) | null = null;

  constructor(readonly log: Log) {}

  async play(): Promise<void> {
    this.log.push(`play ${this.src}`);
    if (this.blocked) throw new DOMException('the page has no user activation', 'NotAllowedError');
    if (!this.held) setTimeout(() => this.onended?.(new Event('ended')), 0);
  }

  pause(): void {
    this.log.push('pause');
    this.onpause?.(new Event('pause'));
  }
}

/** The browser's URL: each blob a page URL is made from, and each URL revoked. */
class FakeUrls implements ObjectUrls {
  made: Blob[] = [];
  revoked: string[] = [];

  createObjectURL(blob: Blob): string {
    this.made.push(blob);
    return `blob:page/${this.made.length}`;
  }

  revokeObjectURL(url: string): void {
    this.revoked.push(url);
  }
}

class FakeSpeaker implements Speaker {
  constructor(readonly log: Log) {}

  async speak(clip: SpeechClip): Promise<void> {
    this.log.push(`speak ${clip.text} ${clip.language}`);
  }

  cancel(): void {
    this.log.push('cancel');
  }
}

const sound = (name: string, type: string | null, bytes: number[]): Clip => ({
  kind: 'sound',
  name,
  type,
  bytes: new Uint8Array(bytes)
});
const speech = (text: string, language: string): Clip => ({ kind: 'speech', text, language, rate: 0.5 });

function face(text: string, autoplay: Clip[], replay: Clip[]): FaceView {
  return { text, css: '', autoplay, replay, omitted: [] };
}

function view(id: number): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false,
    withheld: false
  };
}

/** The engine as the review sees it, with each card's faces. */
class FacesClient implements StudyClient {
  calls: string[] = [];
  constructor(
    readonly heads: Head[],
    readonly faced: Map<bigint, Faces>
  ) {}
  async card(): Promise<Head> {
    this.calls.push('card');
    return this.heads.shift() as Head;
  }
  async faces(card: bigint): Promise<Faces> {
    this.calls.push(`faces ${card}`);
    return this.faced.get(card) as Faces;
  }
  async rate(card: bigint): Promise<null> {
    this.calls.push(`rate ${card}`);
    return null;
  }
  async bury(card: bigint): Promise<null> {
    this.calls.push(`bury ${card}`);
    return null;
  }
  async flag(card: bigint): Promise<number> {
    this.calls.push(`flag ${card}`);
    return 1;
  }
  async undo(): Promise<null> {
    this.calls.push('undo');
    return null;
  }
  async undoOffer(): Promise<UndoOffer> {
    this.calls.push('undo-offer');
    return { offer: null, why: 'none' };
  }
}

const COUNTS = { new: 1, learning: 0, review: 0 };
const DOG = sound('dog.mp3', 'audio/mpeg', [1, 2]);
const HUND = speech('der Hund', 'de-DE');
const BARK = sound('bark.ogg', null, [3]);
const CAT = sound('cat.opus', 'audio/ogg', [4]);

/** Card 1 plays a sound and speech on its question and a sound on its answer; card 2 replays one. */
const FACES = new Map<bigint, Faces>([
  [
    1n,
    {
      question: face('<p>the dog</p>', [DOG, HUND], [DOG, HUND]),
      answer: face('<p>der Hund</p>', [BARK], [DOG, HUND, BARK]),
      wanted: []
    }
  ],
  [2n, { question: face('<p>the cat</p>', [], [CAT]), answer: face('<p>die Katze</p>', [], [CAT]), wanted: [] }]
]);

/** A player whose every sequence the test can wait for. */
function recorded(player: Player) {
  const played: Promise<boolean>[] = [];
  return {
    played,
    player: {
      play: (clips: readonly Clip[]) => {
        const done = player.play(clips);
        played.push(done);
        return done;
      }
    }
  };
}

/** What a page URL was made from: its type and its bytes. */
async function contents(blob: Blob): Promise<[string, number[]]> {
  return [blob.type, [...new Uint8Array(await blob.arrayBuffer())]];
}

describe('the page plays the face', () => {
  it("the face's clips play in order on show and reveal, and replay replays them", async () => {
    const log: Log = [];
    const urls = new FakeUrls();
    const { played, player } = recorded(new Player(new FakeAudio(log), urls, new FakeSpeaker(log)));
    const client = new FacesClient([{ counts: COUNTS, card: view(1) }, { counts: COUNTS, card: view(2) }], FACES);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined,
      player
    );

    // the question's autoplay clips, in the core's order, when the card shows
    review.start();
    await review.settled();
    expect(await Promise.all(played)).toEqual([true]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/1', 'speak der Hund de-DE']);

    // the answer's when it is revealed
    log.length = 0;
    review.act('show-answer');
    expect(await Promise.all(played)).toEqual([true, true]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/2']);

    // replay replays the side's replay clips, and the side stays
    log.length = 0;
    review.act('replay');
    expect(await Promise.all(played)).toEqual([true, true, true]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/3', 'speak der Hund de-DE', 'play blob:page/4']);
    expect([review.phase, review.side]).toEqual(['answer', 'answer']);

    // a reveal that moves nothing and a flag's return play nothing
    review.act('show-answer');
    review.act('flag');
    await review.settled();
    expect(played).toHaveLength(3);
    expect(review.phase).toBe('answer');

    // the next card's question has no autoplay clip, and replay plays its one on the question side
    log.length = 0;
    review.act('good');
    await review.settled();
    expect(review.view?.id).toBe(2n);
    review.act('replay');
    expect(await Promise.all(played)).toEqual([true, true, true, true, true]);
    expect(log).toEqual(['pause', 'cancel', 'pause', 'cancel', 'play blob:page/5']);

    // each sound's page URL was made from its bytes and the core's type, and revoked once played
    expect(await Promise.all(urls.made.map(contents))).toEqual([
      ['audio/mpeg', [1, 2]],
      ['', [3]],
      ['audio/mpeg', [1, 2]],
      ['', [3]],
      ['audio/ogg', [4]]
    ]);
    expect(urls.revoked).toEqual(['blob:page/1', 'blob:page/2', 'blob:page/3', 'blob:page/4', 'blob:page/5']);
    expect(client.calls).toEqual(['card', 'faces 1', 'flag 1', 'rate 1', 'card', 'faces 2']);
  });

  it('a blocked play leaves the replay control', async () => {
    const log: Log = [];
    const audio = new FakeAudio(log);
    const urls = new FakeUrls();
    const { played, player } = recorded(new Player(audio, urls, new FakeSpeaker(log)));
    const client = new FacesClient([{ counts: COUNTS, card: view(1) }], FACES);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined,
      player
    );

    // the browser blocks the first sound: nothing after it plays, and its URL is revoked
    audio.blocked = true;
    review.start();
    await review.settled();
    expect(await Promise.all(played)).toEqual([false]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/1']);
    expect(urls.revoked).toEqual(['blob:page/1']);

    // the review stays on its side with the Replay control, which plays once the page may
    expect([review.phase, review.controls]).toEqual(['question', ['show-answer', 'replay', 'bury', 'flag']]);
    audio.blocked = false;
    log.length = 0;
    review.act('replay');
    expect(await Promise.all(played)).toEqual([false, true]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/2', 'speak der Hund de-DE']);
  });

  it('a new face stops the clips of the last', async () => {
    // a sound still playing when the next sequence starts is paused, and the rest of its sequence
    // never plays
    const log: Log = [];
    const audio = new FakeAudio(log);
    const urls = new FakeUrls();
    const player = new Player(audio, urls, new FakeSpeaker(log));
    audio.held = true;
    const first = player.play([DOG, BARK]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    audio.held = false;
    const second = player.play([CAT]);
    expect(await Promise.all([first, second])).toEqual([true, true]);
    expect(log).toEqual(['pause', 'cancel', 'play blob:page/1', 'pause', 'cancel', 'play blob:page/2']);
    expect(urls.revoked).toEqual(['blob:page/1', 'blob:page/2']);
    expect(await contents(urls.made[1])).toEqual(['audio/ogg', [4]]);
  });
});
