import { describe, expect, it } from 'vitest';
import { DARK_FIELDS_ID, parseDarkFields, parseEnvelope, parseListings } from './insights';

// SPEC-094 R18, R19. The client reads what the server stores and claims nothing it cannot parse:
// a malformed body is null (or undefined for an envelope), never a partial value.
/** Reports how many values a test examined, and refuses a population of zero. */
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

const STORED = {
  dark_fields: [{ note_type: 'Type A', field: 'Extra', reviewed_notes: 7 }],
  dark_fields_total: 9,
  unparseable: [{ note_type: 'Odd', note_type_id: 5 }],
  unparseable_total: 1,
  notetypes_checked: 4,
  reviewed_note_count: 250,
  is_cold: false
};

describe('insight parsers', () => {
  it('names Dark Fields by the id it is stored under', () => {
    expect(DARK_FIELDS_ID).toBe('dark_fields');
  });

  it('reads a listing, with a null study day for an instrument that never ran', () => {
    expect(
      parseListings({
        instruments: [
          { id: 'dark_fields', cadence: 'weekly', study_day: 12 },
          { id: 'other', cadence: 'weekly', study_day: null }
        ]
      })
    ).toEqual([
      { id: 'dark_fields', cadence: 'weekly', studyDay: 12 },
      { id: 'other', cadence: 'weekly', studyDay: null }
    ]);
  });

  it('refuses a listing that is not one', () => {
    expect(parseListings({ instruments: [{ id: 'a', cadence: 'w', study_day: 0 }] })).toEqual([
      { id: 'a', cadence: 'w', studyDay: 0 }
    ]);
    expect(parseListings(null)).toBeNull();
    expect(parseListings([])).toBeNull();
    expect(parseListings({})).toBeNull();
    expect(parseListings({ instruments: [null] })).toBeNull();
    expect(parseListings({ instruments: [{ id: 1, cadence: 'weekly', study_day: 1 }] })).toBeNull();
    expect(parseListings({ instruments: [{ id: 'a', cadence: 2, study_day: 1 }] })).toBeNull();
    expect(parseListings({ instruments: [{ id: 'a', cadence: 'w', study_day: -1 }] })).toBeNull();
    expect(parseListings({ instruments: [{ id: 'a', cadence: 'w', study_day: 1.5 }] })).toBeNull();
    expect(parseListings({ instruments: [{ id: 'a', cadence: 'w', study_day: 'x' }] })).toBeNull();
  });

  it('reads an envelope, a not-yet-run report as null, and a malformed one as undefined', () => {
    const report = { study_day: 3, failed_reads: ['templates'], report: { a: 1 } };
    expect(parseEnvelope({ instrument: 'x', report })).toEqual({
      instrument: 'x',
      studyDay: 3,
      failedReads: ['templates'],
      report: { a: 1 }
    });
    expect(parseEnvelope({ instrument: 'x', report: { ...report, report: null } })).toEqual({
      instrument: 'x',
      studyDay: 3,
      failedReads: ['templates'],
      report: null
    });
    expect(parseEnvelope({ instrument: 'x', report: null })).toBeNull();
    expect(parseEnvelope(null)).toBeUndefined();
    expect(parseEnvelope({ report })).toBeUndefined();
    expect(parseEnvelope({ instrument: 'x', report: 'no' })).toBeUndefined();
    expect(parseEnvelope({ instrument: 'x', report: { ...report, failed_reads: [1] } })).toBeUndefined();
    expect(parseEnvelope({ instrument: 'x', report: { ...report, failed_reads: 'a' } })).toBeUndefined();
    expect(parseEnvelope({ instrument: 'x', report: { ...report, study_day: -1 } })).toBeUndefined();
    expect(parseEnvelope({ instrument: 'x', report: { ...report, study_day: 1.5 } })).toBeUndefined();
  });

  it('reads Dark Fields as stored', () => {
    expect(parseDarkFields(STORED)).toEqual({
      darkFields: [{ noteType: 'Type A', field: 'Extra', reviewedNotes: 7 }],
      darkFieldsTotal: 9,
      unparseable: [{ noteType: 'Odd', noteTypeId: 5 }],
      unparseableTotal: 1,
      notetypesChecked: 4,
      reviewedNoteCount: 250,
      isCold: false
    });
    expect(parseDarkFields({ ...STORED, is_cold: true })?.isCold).toBe(true);
    expect(parseDarkFields({ ...STORED, is_cold: 'true' })?.isCold).toBe(false);
  });

  it('refuses Dark Fields that are not the stored shape', () => {
    const bad = (over: Record<string, unknown>) => parseDarkFields({ ...STORED, ...over });
    expect(bad({})?.darkFieldsTotal).toBe(9);
    expect(parseDarkFields(null)).toBeNull();
    expect(parseDarkFields([])).toBeNull();
    expect(bad({ dark_fields: 'x' })).toBeNull();
    expect(bad({ unparseable: 'x' })).toBeNull();
    expect(bad({ dark_fields: 5 })).toBeNull();
    expect(bad({ dark_fields: {} })).toBeNull();
    expect(bad({ unparseable: 5 })).toBeNull();
    expect(bad({ unparseable: {} })).toBeNull();
    expect(bad({ dark_fields_total: -1 })).toBeNull();
    expect(bad({ unparseable_total: 'x' })).toBeNull();
    expect(bad({ notetypes_checked: 1.5 })).toBeNull();
    expect(bad({ reviewed_note_count: null })).toBeNull();
    expect(bad({ dark_fields: [null] })).toBeNull();
    expect(bad({ dark_fields: [{ note_type: 1, field: 'f', reviewed_notes: 1 }] })).toBeNull();
    expect(bad({ dark_fields: [{ note_type: 'n', field: 2, reviewed_notes: 1 }] })).toBeNull();
    expect(bad({ dark_fields: [{ note_type: 'n', field: 'f', reviewed_notes: -1 }] })).toBeNull();
    expect(bad({ unparseable: [null] })).toBeNull();
    expect(bad({ unparseable: [{ note_type: 1, note_type_id: 1 }] })).toBeNull();
    expect(bad({ unparseable: [{ note_type: 'n', note_type_id: 'x' }] })).toBeNull();
  });

  it('refuses a report the stored envelope does not hold, and every value that is no report', () => {
    // The page hands parseDarkFields an envelope's report, which is undefined when the stored
    // envelope has no report key: a value no JSON body parses to, refused like every JSON shape.
    const envelope = parseEnvelope({ instrument: DARK_FIELDS_ID, report: { study_day: 3, failed_reads: [] } });
    expect(envelope).toEqual({ instrument: DARK_FIELDS_ID, studyDay: 3, failedReads: [], report: undefined });
    expect(parseDarkFields(envelope?.report)).toBeNull();
    const values = [undefined, null, true, false, 0, 3, '', 'text', [], [STORED], { nested: {} }];
    for (const value of examined('values that are no report', values)) {
      expect(parseDarkFields(value), String(value)).toBeNull();
    }
    expect(parseDarkFields(STORED)?.darkFieldsTotal).toBe(9);
  });
});
