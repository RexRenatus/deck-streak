// The study suite (SPEC-350 R13, A20; ADR-361): the built Mini App over the real module, staged at
// `/engine/` beside the build, in Chromium and WebKit. Each test runs in a persistent profile of its
// own, as a user's browser has (an empty path is a fresh temporary profile), seeds that profile's
// collection through the app's own Worker (seed.ts), and then reviews from the deck list: show
// answer, the intervals on the buttons, a rating by key and one by button, undo, bury and flag, a
// key after a tap on the card, and the accessibility audit of both screens.
// study-coverage.test.ts holds this file to those steps.
import AxeBuilder from '@axe-core/playwright';
import {
  expect,
  test,
  type BrowserContext,
  type Page,
  type PlaywrightWorkerArgs,
  type PlaywrightWorkerOptions
} from '@playwright/test';
import { seed } from './seed';

/** The notes each test seeds: enough that two ratings and a bury still leave a new card. */
const NOTES = 5;

/** The seeded collection's one deck, as the deck list names its button. */
const DECK = `Default ${NOTES} new, 0 learning, 0 to review`;

/** The card frame's title on each side. */
const QUESTION = "The card's question";
const ANSWER = "The card's answer";

/** axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA, as the app's audit runs them. */
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/** Telegram's script, which the app loads only on a launch (SPEC-400); this suite opens it outside
 * one, and the route answers the script empty should it be asked for, so nothing reaches the network. */
const TELEGRAM_SDK = 'https://telegram.org/js/telegram-web-app.js';

/** A persistent profile of the test's own browser, its collection seeded, on the deck list. */
async function studying(
  playwright: PlaywrightWorkerArgs['playwright'],
  browserName: PlaywrightWorkerOptions['browserName'],
  baseURL: string | undefined
): Promise<{ context: BrowserContext; page: Page }> {
  const context = await playwright[browserName].launchPersistentContext('', { baseURL });
  await context.route(`${TELEGRAM_SDK}*`, (route) =>
    route.fulfill({ contentType: 'text/javascript', body: '' })
  );
  const page = await context.newPage();
  await page.goto('/');
  expect(await seed(page, NOTES)).toBe(NOTES);
  await page.goto('/study');
  return { context, page };
}

/** Chooses the seeded deck on the deck list, and returns the number of the card the review shows. */
async function review(page: Page): Promise<string> {
  await page.getByRole('button', { name: DECK, exact: true }).click();
  await expect(page).toHaveURL(/\/study\/review$/);
  return shown(page);
}

/** The number of the seeded note whose question the card frame shows. */
async function shown(page: Page): Promise<string> {
  const question = page.getByTitle(QUESTION, { exact: true });
  await expect(question).toHaveAttribute('srcdoc', /front \d+ /);
  const number = /front (\d+) /.exec((await question.getAttribute('srcdoc')) ?? '')?.[1] ?? '';
  expect(number).toMatch(/^\d+$/);
  return number;
}

/** Waits for the question of a card other than `previous`, and returns its number. */
async function next(page: Page, previous: string): Promise<string> {
  await expect(page.getByTitle(QUESTION, { exact: true })).toHaveAttribute(
    'srcdoc',
    new RegExp(`front (?!${previous} )\\d+ `)
  );
  return shown(page);
}

/** Shows the answer of card `number` with Space, and waits for its answer in the frame. */
async function reveal(page: Page, number: string): Promise<void> {
  await page.keyboard.press('Space');
  await expect(page.getByTitle(ANSWER, { exact: true })).toHaveAttribute(
    'srcdoc',
    new RegExp(`back ${number} `)
  );
}

test('the deck list shows the seeded deck and opens the review', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    await expect(page.getByRole('heading', { name: 'Study', exact: true, level: 1 })).toBeVisible();
    await expect(page.getByRole('button', { name: DECK, exact: true })).toBeVisible();
    await review(page);
    // the review's first view: the question, and Show answer to reveal it
    await expect(page.getByRole('heading', { name: 'Review', exact: true, level: 1 })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Show answer', exact: true })).toBeVisible();
    await expect(page.getByTitle(ANSWER, { exact: true })).toHaveCount(0);
  } finally {
    await context.close();
  }
});

test('show answer reveals the answer and the buttons show the intervals', async ({
  playwright,
  browserName,
  baseURL
}) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    const card = await review(page);
    await page.getByRole('button', { name: 'Show answer', exact: true }).click();
    await expect(page.getByTitle(ANSWER, { exact: true })).toHaveAttribute('srcdoc', new RegExp(`back ${card} `));
    await expect(page.getByRole('button', { name: 'Show answer', exact: true })).toHaveCount(0);
    // each answer button is named by its grade and the engine's interval for it
    for (const grade of ['Again', 'Good']) {
      await expect(page.getByRole('button', { name: new RegExp(`^${grade} \\S`) })).toBeVisible();
    }
  } finally {
    await context.close();
  }
});

test('a rating by key and one by button each show the next card', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    const first = await review(page);
    await reveal(page, first);
    await page.keyboard.press('3');
    const second = await next(page, first);
    await reveal(page, second);
    await page.getByRole('button', { name: /^Good \S/ }).click();
    const third = await next(page, second);
    expect(new Set([first, second, third]).size).toBe(3);
  } finally {
    await context.close();
  }
});

test('undo returns the rated card', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    const rated = await review(page);
    const undo = page.getByRole('button', { name: 'Undo answer', exact: true });
    await expect(undo).toBeDisabled();
    await reveal(page, rated);
    await page.keyboard.press('3');
    await next(page, rated);
    await expect(undo).toBeEnabled();
    await undo.click();
    // the review asks first (SPEC-371 R10): the dialog's own Undo answer confirms
    const dialog = page.getByRole('alertdialog');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Undo answer', exact: true }).click();
    await expect(page.getByTitle(QUESTION, { exact: true })).toHaveAttribute('srcdoc', new RegExp(`front ${rated} `));
  } finally {
    await context.close();
  }
});

test('bury and flag act on the shown card', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    const card = await review(page);
    const flag = page.getByRole('button', { name: 'Flag', exact: true });
    await expect(flag).toHaveAttribute('aria-pressed', 'false');
    await flag.click();
    await expect(flag).toHaveAttribute('aria-pressed', 'true');
    await page.getByRole('button', { name: 'Bury', exact: true }).click();
    // the next card is another, and the flag was the buried card's alone
    await next(page, card);
    await expect(flag).toHaveAttribute('aria-pressed', 'false');
    // the buried card has left the deck's new cards
    await page.getByRole('link', { name: 'Back to decks', exact: true }).click();
    await expect(page).toHaveURL(/\/study$/);
    await expect(
      page.getByRole('button', { name: `Default ${NOTES - 1} new, 0 learning, 0 to review`, exact: true })
    ).toBeVisible();
  } finally {
    await context.close();
  }
});

test('a key after a tap on the card still rates', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    const tapped = await review(page);
    await reveal(page, tapped);
    // a tap on the card moves the focus into the frame; the review takes it back
    await page.getByTitle(ANSWER, { exact: true }).click();
    await expect(page.getByRole('region', { name: 'Review', exact: true })).toBeFocused();
    await page.keyboard.press('3');
    await next(page, tapped);
  } finally {
    await context.close();
  }
});

test('axe passes on both screens', async ({ playwright, browserName, baseURL }) => {
  const { context, page } = await studying(playwright, browserName, baseURL);
  try {
    // the deck list, with the seeded deck on it
    await expect(page.getByRole('button', { name: DECK, exact: true })).toBeVisible();
    const decks = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
    expect(decks.violations).toEqual([]);
    // the review, with a card in the frame
    await page.getByRole('button', { name: DECK, exact: true }).click();
    await expect(page).toHaveURL(/\/study\/review$/);
    await shown(page);
    // the card frame is sandboxed with no script (SPEC-341 R2, ADR-352), so no axe mode can run inside it and the default mode's frame walk never returns; the legacy mode audits the review screen around the frame
    const card = await new AxeBuilder({ page }).withTags(WCAG_22_AA).setLegacyMode().analyze();
    expect(card.violations).toEqual([]);
  } finally {
    await context.close();
  }
});
