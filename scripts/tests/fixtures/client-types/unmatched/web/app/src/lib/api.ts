import { parseTally, TALLY_PATH } from './tally';

export const api = {
  tally: () => read(TALLY_PATH, parseTally),
};
