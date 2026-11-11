export const TALLY_PATH = '/api/elsewhere';

export interface TallyView {
  points: number;
  streak: number;
}

export function parseTally(body: unknown): TallyView {
  const { points, streak } = body as Record<string, unknown>;
  return { points: Number(points), streak: Number(streak) };
}
