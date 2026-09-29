/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import DarkFields from './DarkFields.svelte';
import type { DarkFieldsReport } from './insights';

// SPEC-094 R19, A20. Zero dark fields is a checked result: the section says how many note types
// it checked, never an empty list; a cold collection says there was nothing to check.
function report(over: Partial<DarkFieldsReport>): DarkFieldsReport {
  return {
    darkFields: [],
    darkFieldsTotal: 0,
    unparseable: [],
    unparseableTotal: 0,
    notetypesChecked: 4,
    reviewedNoteCount: 250,
    isCold: false,
    ...over
  };
}

describe('DarkFields', () => {
  it('states zero dark fields as a checked result', () => {
    render(DarkFields, { props: { report: report({}) } });
    expect(screen.getByText(/checked 4 note types/i).textContent).toContain('no dark fields');
    expect(screen.queryByRole('list')).toBeNull();
  });

  it('lists each dark field with its note type and counts what is not shown', () => {
    render(DarkFields, {
      props: {
        report: report({
          darkFields: [{ noteType: 'Type A', field: 'Extra', reviewedNotes: 7 }],
          darkFieldsTotal: 9
        })
      }
    });
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain('Type A');
    expect(items[0].textContent).toContain('Extra');
    expect(items[0].textContent).toContain('7');
    expect(screen.getByText(/8 more/i)).toBeTruthy();
  });

  it('says a cold collection had nothing to check', () => {
    render(DarkFields, { props: { report: report({ isCold: true, notetypesChecked: 0 }) } });
    expect(screen.getByText(/nothing to check/i)).toBeTruthy();
    expect(screen.queryByText(/no dark fields/i)).toBeNull();
  });

  it('says nothing is left out when every dark field is listed', () => {
    render(DarkFields, {
      props: {
        report: report({
          darkFields: [{ noteType: 'Type A', field: 'Extra', reviewedNotes: 7 }],
          darkFieldsTotal: 1
        })
      }
    });
    expect(screen.queryByText(/more not shown/i)).toBeNull();
    expect(screen.queryByText(/0 more/i)).toBeNull();
  });

  it('names the note types whose templates could not be verified, and only then', () => {
    const { unmount } = render(DarkFields, { props: { report: report({}) } });
    expect(screen.queryByText(/could not be verified/i)).toBeNull();
    unmount();
    render(DarkFields, {
      props: {
        report: report({
          unparseable: [
            { noteType: 'Alpha', noteTypeId: 1 },
            { noteType: 'Beta', noteTypeId: 2 }
          ],
          unparseableTotal: 2
        })
      }
    });
    expect(screen.getByText(/could not be verified/i).textContent).toContain('Alpha, Beta');
  });
});
