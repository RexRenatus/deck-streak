// SPEC-350 R5; ADR-361. The app's engine. Stub: it starts nothing.
import type { EngineClient, EnginePort } from '$lib/engine/client';

/** The Worker as the study engine sees it: a port it can also end. */
export interface WorkerLike extends EnginePort {
  terminate(): void;
}

/** What the engine hears from the page: that it is hidden. */
export interface PageLike {
  addEventListener(type: 'pagehide', listener: () => void): void;
}

export class StudyEngine {
  constructor(make: () => WorkerLike, page: PageLike, origin: string, languages: () => string[]) {
    void make;
    void page;
    void origin;
    void languages;
  }

  client(): Promise<EngineClient> {
    return Promise.reject(new Error('the study engine starts nothing yet'));
  }

  closed(): Promise<void> {
    return Promise.resolve();
  }
}
