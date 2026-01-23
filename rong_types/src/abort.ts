/**
 * Abort module type definitions
 * Corresponds to: modules/rong_abort
 */

import { EventTarget } from './event';

export interface AbortSignal extends EventTarget {
  /** Whether the signal has been aborted */
  readonly aborted: boolean;

  /** Reason for abort (if any) */
  readonly reason: any;

  /** Abort event handler */
  onabort: ((event: Event) => void) | null;

  /** If the signal has been aborted, throw the abort reason */
  throwIfAborted(): void;
}

export interface AbortSignalConstructor {
  prototype: AbortSignal;

  /** Returns an AbortSignal that is aborted when any of the given signals are aborted */
  any(signals: AbortSignal[]): AbortSignal;

  /** Returns an AbortSignal that is already aborted */
  abort(reason?: any): AbortSignal;

  /** Returns an AbortSignal that will abort after the specified milliseconds */
  timeout(milliseconds: number): AbortSignal;
}

export interface AbortController {
  /** The AbortSignal associated with this controller */
  readonly signal: AbortSignal;

  /** Abort the associated signal */
  abort(reason?: any): void;
}

export interface AbortControllerConstructor {
  new(): AbortController;
  prototype: AbortController;
}

// Note: AbortSignal and AbortController are provided by the global environment
// These type definitions are for reference and extend the standard Web API
export {};
