import type { RoutePath } from './routes';

/** The route a startapp token opens. */
export function routeFor(token: string | null | undefined): RoutePath {
  return `/${token ?? ''}` as RoutePath;
}
