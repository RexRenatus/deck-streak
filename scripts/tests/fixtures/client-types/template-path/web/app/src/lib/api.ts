import { parseTally } from './tally';

export const api = {
  tally: (id) =>
    read(`/api/tally/${encodeURIComponent(id)}`, (body) => {
      const parsed = parseTally(body);
      return parsed === undefined ? null : { value: parsed };
    }),
};
