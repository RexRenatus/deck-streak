/**
 * @vitest-environment jsdom
 */
import { createRawSnippet } from 'svelte';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import InstrumentSection from './InstrumentSection.svelte';
import type { Envelope } from './insights';

// SPEC-094 R19, A19. A section whose run failed or read something it could not read says so: the
// failure line names each failed read, and the body, an all-clear included, is not drawn.
const body = createRawSnippet(() => ({ render: () => '<p>All clear: nothing found.</p>' }));

function stored(failedReads: string[], report: unknown): Envelope {
  return { instrument: 'synthetic', studyDay: 100, failedReads, report };
}

describe('InstrumentSection', () => {
  it('renders a failed read as a failure line', () => {
    render(InstrumentSection, {
      props: { title: 'Synthetic', envelope: stored(['templates', 'fields'], { found: 0 }), children: body }
    });
    const line = screen.getByRole('alert');
    expect(line.textContent).toContain('templates');
    expect(line.textContent).toContain('fields');
    expect(screen.queryByText(/all clear/i)).toBeNull();
  });

  it('renders a run that failed outright as a failure line', () => {
    render(InstrumentSection, {
      props: { title: 'Synthetic', envelope: stored(['run'], null), children: body }
    });
    expect(screen.getByRole('alert').textContent).toContain('run');
    expect(screen.queryByText(/all clear/i)).toBeNull();
  });

  it('draws the body of a clean read', () => {
    render(InstrumentSection, {
      props: { title: 'Synthetic', envelope: stored([], { found: 0 }), children: body }
    });
    expect(screen.getByText('All clear: nothing found.')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Synthetic' })).toBeTruthy();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
