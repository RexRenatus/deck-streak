/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import AnswerButtons from './AnswerButtons.svelte';

// SPEC-350 R7, R10, A13; ADR-361. The answer buttons are native buttons, in grade order, each named
// by its grade and the interval the engine describes for it (SPEC-334 R18), each reached by Tab
// and each answering its own grade.
describe('the answer buttons', () => {
  it('each answer button names its grade and its interval', async () => {
    const answered: string[] = [];
    render(AnswerButtons, {
      labels: ['<1m', '<6m', '<10m', '4d'],
      onanswer: (grade: string) => answered.push(grade)
    });

    // each button's accessible name, as the accessibility tree computes it, in document order
    const names: string[] = [];
    const buttons = screen.queryAllByRole('button', {
      name: (name) => {
        names.push(name);
        return true;
      }
    });
    expect(names).toEqual(['Again <1m', 'Hard <6m', 'Good <10m', 'Easy 4d']);

    // native buttons in the tab order, none disabled, so Tab reaches all four in grade order
    expect(buttons.map((button) => [button.tagName, button.getAttribute('type'), button.tabIndex])).toEqual([
      ['BUTTON', 'button', 0],
      ['BUTTON', 'button', 0],
      ['BUTTON', 'button', 0],
      ['BUTTON', 'button', 0]
    ]);
    expect(buttons.filter((button) => (button as HTMLButtonElement).disabled)).toEqual([]);

    // each answers its own grade
    for (const button of buttons) await fireEvent.click(button);
    expect(answered).toEqual(['again', 'hard', 'good', 'easy']);
  });
});
