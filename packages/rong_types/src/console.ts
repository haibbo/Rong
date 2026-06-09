/**
 * Console module type definitions
 * Corresponds to: modules/rong_console
 *
 * Note: console is a global object provided by the runtime.
 * This module extends the standard Web Console API.
 */

export interface Console {
  /** Log messages to stdout */
  log(...args: any[]): void;

  /** Log error messages to stderr */
  error(...args: any[]): void;

  /** Log warning messages to stderr */
  warn(...args: any[]): void;

  /** Log informational messages to stdout */
  info(...args: any[]): void;

  /** Log debug messages to stdout */
  debug(...args: any[]): void;

  /** Log assertion failures to stderr */
  assert(condition?: any, ...args: any[]): void;

  /** Inspect a value with optional depth and length limits */
  dir(
    value?: any,
    options?: {
      depth?: number;
      maxArrayLength?: number;
      maxObjectKeys?: number;
    },
  ): void;

  /** Log a stack trace */
  trace(...args: any[]): void;

  /** Start a named timer */
  time(label?: string): void;

  /** Log the elapsed time for a timer without stopping it */
  timeLog(label?: string, ...args: any[]): void;

  /** Log the elapsed time for a timer and stop it */
  timeEnd(label?: string): void;

  /** Increment and log a named counter */
  count(label?: string): void;

  /** Reset a named counter */
  countReset(label?: string): void;

  /** Clear the console */
  clear(): void;
}

// Note: console is a global object provided by the runtime
// The standard Web Console API is available globally
// This module documents Rong-specific console features

/**
 * Rong console supports format strings in console.log():
 * - %s - String substitution
 * - %d, %i - Integer substitution
 * - %f - Float substitution
 * - %o - Object inspection
 *
 * Example:
 * console.log("Name: %s, Age: %d", "Alice", 30);
 */

export {};
