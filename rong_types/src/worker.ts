/**
 * Worker module type definitions
 * Corresponds to: modules/rong_worker
 *
 * The runtime exposes a global `Worker` constructor. This file documents the
 * runtime-specific subset and event payload shapes.
 */

export interface WorkerMessageEvent {
  readonly data: any;
}

export interface WorkerErrorEvent {
  readonly type: 'error';
  readonly message: string;
}

export interface RongWorker {
  onmessage: ((event: WorkerMessageEvent) => void) | null;
  onerror: ((event: WorkerErrorEvent) => void) | null;
  postMessage(data: any): void;
  terminate(): void;
}

export interface RongWorkerConstructor {
  new(path: string): RongWorker;
  prototype: RongWorker;
}

export {};
