/**
 * Timer module type definitions
 * Corresponds to: modules/rong_timer
 */

export type TimerCallback = () => void;
export type TimerId = number;

export interface TimersNamespace {
  /**
   * Promise-based timeout that resolves with a timestamp (ms since epoch).
   */
  setTimeout(delay?: number): Promise<number>;

  /**
   * Promise-based immediate that resolves with a timestamp (ms since epoch).
   */
  setImmediate(): Promise<number>;

  /**
   * Async iterator that yields timestamps (ms since epoch) on each interval tick.
   */
  setInterval(delay?: number): AsyncIterableIterator<number>;
}

declare global {
  /**
   * Set a timer that executes a callback once after a delay
   * @param callback - Function to execute
   * @param delay - Delay in milliseconds (default: 0)
   * @returns Timer ID that can be used with clearTimeout
   */
  function setTimeout(callback: TimerCallback, delay?: number): TimerId;

  /**
   * Clear a timer set with setTimeout
   * @param id - Timer ID returned by setTimeout
   */
  function clearTimeout(id: TimerId): void;

  /**
   * Set a timer that executes a callback repeatedly with a delay between executions
   * @param callback - Function to execute
   * @param delay - Delay in milliseconds between executions (default: 0)
   * @returns Timer ID that can be used with clearInterval
   */
  function setInterval(callback: TimerCallback, delay?: number): TimerId;

  /**
   * Clear a timer set with setInterval
   * @param id - Timer ID returned by setInterval
   */
  function clearInterval(id: TimerId): void;

  /**
   * Promise-based timer namespace
   */
  const timers: TimersNamespace;
}

export {};
