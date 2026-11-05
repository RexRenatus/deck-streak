import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'vitest';
import { frameDocument } from './frame-document';

// SPEC-618 R9, R13: the card frame's document (the inert parse, the strip, the composition and the
// re-parse check) over both sides of every golden template card, with the card's CSS and classes,
// and over 20 cards a seeded generator writes, so every run measures the same inputs. The bench
// asserts nothing about the documents it composes.

/** One card side as the frame takes it. */
interface Input {
  html: string;
  css: string;
  classes: string;
}

/** The golden file's render, as review-templates.test.ts reads it. */
interface Golden {
  cards: { ordinal: number; question: string; answer: string; face_css: string }[];
}

const GOLDEN = JSON.parse(
  readFileSync(resolve(import.meta.dirname, '../study/review-templates.golden.json'), 'utf8')
) as Golden;

const WORDS = ['tide', 'ember', 'quartz', 'lumen', 'fjord', 'basalt', 'cirrus', 'delta', 'moraine', 'zephyr'];

/** The cards the seeded generator writes for each iteration, beside the golden sides. */
const GENERATED_CARDS = 20;

/** A fixed-seed linear congruential generator, its values in [0, 1). */
function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 4294967296;
  };
}

/** `count` cards with markup the frame strips (a srcset, a link target) beside text and a cloze. */
function generated(count: number): Input[] {
  const next = seeded(618);
  const word = () => WORDS[Math.floor(next() * WORDS.length)];
  return Array.from({ length: count }, (_, index) => {
    const text = Array.from({ length: 4 + Math.floor(next() * 12) }, word).join(' ');
    const html =
      `<div class="front"><p><b>${word()}</b> ${text}</p>` +
      `<img alt="" src="card-${index}.png" srcset="card-${index}-2x.png 2x">` +
      `<a href="https://cards.example/${index}" target="_blank">${word()}</a>` +
      `<span class="cloze">${word()}</span></div>`;
    const css = `.card { font-size: ${14 + Math.floor(next() * 10)}px; }\r\n.cloze { font-weight: bold; }`;
    return { html, css, classes: `card card${1 + Math.floor(next() * 3)}` };
  });
}

const INPUTS: Input[] = [
  ...GOLDEN.cards.flatMap((card) =>
    [card.question, card.answer].map((html) => ({ html, css: card.face_css, classes: `card card${card.ordinal + 1}` }))
  ),
  ...generated(GENERATED_CARDS)
];

test('frameDocument', async ({ bench }) => {
  await bench('frameDocument', () => {
    for (const input of INPUTS) frameDocument(input.html, input.css, input.classes);
  }).run();
});
