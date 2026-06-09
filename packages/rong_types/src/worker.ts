/**
 * Worker module type definitions
 * Corresponds to: modules/rong_worker
 *
 * Rong exposes the standard global `Worker` name, but this package does not
 * redeclare the global constructor because projects are expected to include the
 * DOM lib for shared Web API types. Doing so would conflict with `lib.dom.d.ts`.
 *
 * Export the Rong-specific subset here for documentation and for precise local
 * annotations when you want the runtime surface rather than the full browser
 * Worker API.
 */

export interface RongWorkerMessageEvent<T = any> {
  readonly data: T;
}

export interface RongWorkerErrorEvent {
  readonly type: 'error';
  readonly message: string;
}

export interface RongWorker {
  onmessage: ((event: RongWorkerMessageEvent) => void) | undefined;
  onerror: ((event: RongWorkerErrorEvent) => void) | undefined;
  postMessage(data: unknown): void;
  terminate(): void;
}

export interface RongWorkerConstructor {
  new (path: string): RongWorker;
  prototype: RongWorker;
}

export {};
