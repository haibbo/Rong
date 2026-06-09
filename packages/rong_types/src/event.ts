/**
 * Event module type definitions
 * Corresponds to: modules/rong_event
 */

export interface EventOptions {
  /** Whether the event bubbles */
  bubbles?: boolean;
  /** Whether the event can be cancelled */
  cancelable?: boolean;
  /** Whether the event can cross shadow DOM boundaries */
  composed?: boolean;
}

export interface Event {
  /** Event type name */
  readonly type: string;

  /** Whether the event bubbles */
  readonly bubbles: boolean;

  /** Whether the event can be cancelled */
  readonly cancelable: boolean;

  /** Whether the event can cross shadow DOM boundaries */
  readonly composed: boolean;
}

export interface EventConstructor {
  new(type: string, options?: EventOptions): Event;
  prototype: Event;
}

export interface CustomEventOptions extends EventOptions {
  /** Custom data associated with the event */
  detail?: any;
}

export interface CustomEvent extends Event {
  /** Custom data associated with the event */
  readonly detail: any;
}

export interface CustomEventConstructor {
  new(type: string, options?: CustomEventOptions): CustomEvent;
  prototype: CustomEvent;
}

export type EventListener = (event: Event) => void;

export interface AddEventListenerOptions {
  /** Remove listener after first invocation */
  once?: boolean;
  /** Use capture phase */
  capture?: boolean;
  /** Listener is passive (won't call preventDefault) */
  passive?: boolean;
}

export interface EventTarget {
  /** Add event listener (Web standard) */
  addEventListener(type: string, listener: EventListener, options?: boolean | AddEventListenerOptions): void;

  /** Remove event listener (Web standard) */
  removeEventListener(type: string, listener: EventListener, options?: boolean | AddEventListenerOptions): void;

  /** Dispatch event (Web standard) */
  dispatchEvent(event: Event): boolean;
}

export interface EventTargetConstructor {
  new(): EventTarget;
  prototype: EventTarget;
}

export type EventName = string | symbol;
export type EventEmitterListener = (...args: any[]) => void;

export interface EventEmitter extends EventTarget {
  /** Add event listener (Node.js style) */
  on(eventName: EventName, listener: EventEmitterListener): this;

  /** Add one-time event listener */
  once(eventName: EventName, listener: EventEmitterListener): this;

  /** Remove event listener */
  off(eventName: EventName, listener: EventEmitterListener): this;

  /** Remove event listener (alias for off) */
  removeListener(eventName: EventName, listener: EventEmitterListener): this;

  /** Remove all listeners for an event */
  removeAllListeners(eventName?: EventName): this;

  /** Add listener at the beginning of the listeners array */
  prependListener(eventName: EventName, listener: EventEmitterListener): this;

  /** Add one-time listener at the beginning of the listeners array */
  prependOnceListener(eventName: EventName, listener: EventEmitterListener): this;

  /** Emit an event */
  emit(eventName: EventName, ...args: any[]): boolean;

  /** Get array of registered event names */
  eventNames(): EventName[];

  /** Set maximum number of listeners per event (default: 10) */
  setMaxListeners(n: number): this;

  /** Get maximum number of listeners per event */
  getMaxListeners(): number;

  /** Get number of listeners for an event, optionally filtered by a specific listener */
  listenerCount(eventName: EventName, listener?: EventEmitterListener): number;
}

export interface EventEmitterConstructor {
  new(): EventEmitter;
  prototype: EventEmitter;
}

// Note: Event, CustomEvent, and EventTarget are provided by the global environment (Web API)
// EventEmitter is a Rong-specific Node.js-style event emitter
declare global {
  const EventEmitter: EventEmitterConstructor;
}

export {};
