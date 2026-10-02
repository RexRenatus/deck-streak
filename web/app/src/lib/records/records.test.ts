import { describe, expect, it } from 'vitest';
import { parseRecords } from './records';

// SPEC-073 R13, R17. The records screen reads `GET /api/records` as the server answers it, and
// refuses a body that is not one rather than showing a part of it.
const LINE = {
  kind: 'best_score',
  label: 'Best daily score',
  value: 120,
  study_day: '2025-01-09',
  previous: 100,
  today: 77,
  distance: 43
};
const BODY = { records: [LINE], chase: { kind: 'best_score', label: 'Best daily score', gap: 43 } };

describe('parseRecords', () => {
  it('reads each record with its distance from today, and the record to chase', () => {
    expect(parseRecords(BODY)).toEqual({
      records: [
        {
          kind: 'best_score',
          label: 'Best daily score',
          value: 120,
          studyDay: '2025-01-09',
          previous: 100,
          today: 77,
          distance: 43
        }
      ],
      chase: { kind: 'best_score', label: 'Best daily score', gap: 43 }
    });
  });

  it('reads an owner with no records and nothing to chase', () => {
    expect(parseRecords({ records: [], chase: null })).toEqual({ records: [], chase: null });
  });

  it('refuses a body that is not the records answer', () => {
    const refused = [
      null,
      'records',
      {},
      { records: [] },
      { records: {}, chase: null },
      { records: [{ ...LINE, study_day: '9 January 2025' }], chase: null },
      { records: [{ ...LINE, distance: '43' }], chase: null },
      { records: [{ ...LINE, label: undefined }], chase: null },
      { records: [{ ...LINE, previous: null }], chase: null },
      { records: [LINE], chase: { kind: 'best_score', label: 'Best daily score' } },
      { records: [LINE], chase: { kind: 'best_score', gap: 43 } }
    ];
    for (const body of refused) {
      expect(parseRecords(body), JSON.stringify(body)).toBeNull();
    }
  });
});
