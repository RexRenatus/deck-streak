import { parseTally, TALLY_PATH } from './tally';

export const api = {
  tally: async () => {
    const tally = await read(TALLY_PATH, parseTally);
    return tally;
  },
};
