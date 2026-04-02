/**
 * Error type definitions for Rong runtime
 *
 * Rong can throw or reject with any JavaScript value.
 *
 * Common host-generated cases are:
 * - standard JS `Error` objects (including `TypeError`, `RangeError`, etc.)
 * - `DOMException`-like objects for web-style failures (e.g. `AbortError`)
 * - preserved JavaScript payloads from `throw <value>` / Promise rejection / abort reasons
 *
 * This module provides type guards and utilities for the error-object subset.
 */

/**
 * Known error names commonly produced by Rong host APIs.
 */
export type KnownRongErrorName =
  | 'Error'
  | 'TypeError'
  | 'RangeError'
  | 'ReferenceError'
  | 'AbortError'
  | 'NetworkError'
  | 'SyntaxError'
  | 'InvalidStateError'
  | 'NotSupportedError'
  | 'TimeoutError'
  | 'SecurityError'
  | 'QuotaExceededError'
  | 'NotFoundError'
  | 'DataCloneError'
  | 'InvalidAccessError'
  | 'TypeMismatchError'
  | 'URLMismatchError';

/**
 * Backwards-compatible alias for the known error-name set.
 */
export type RongErrorName = KnownRongErrorName;

/**
 * Catch/reject payload from Rong.
 *
 * This is intentionally `unknown`: runtime code may preserve arbitrary JavaScript values.
 */
export type RongThrowable = unknown;

/**
 * Error-object shape commonly produced by Rong host APIs.
 *
 * Note that not every caught Rong throwable matches this interface.
 */
export interface RongError extends Error {
  /** Error name identifying the error type */
  readonly name: string;

  /** Human-readable error message */
  readonly message: string;

  /** Stable Rong error code (e.g. "E_IO", "E_TIMEOUT", ...) when available */
  readonly code?: string;

  /** Optional structured error data when provided by the runtime */
  readonly data?: unknown;

  /** Stack trace (if available) */
  readonly stack?: string;
}

/**
 * Error thrown when an operation is aborted via AbortSignal.
 *
 * @example
 * ```typescript
 * const controller = new AbortController();
 * setTimeout(() => controller.abort(), 1000);
 *
 * try {
 *   await fetch('https://api.example.com/slow', {
 *     signal: controller.signal
 *   });
 * } catch (error) {
 *   if (isAbortError(error)) {
 *     console.log('Operation cancelled');
 *   }
 * }
 * ```
 */
export interface AbortError extends RongError {
  readonly name: 'AbortError';
}

export interface NetworkError extends RongError {
  readonly name: 'NetworkError';
}

function getField(error: unknown, key: string): unknown {
  if ((typeof error === 'object' && error !== null) || typeof error === 'function') {
    return (error as Record<string, unknown>)[key];
  }
  return undefined;
}

/**
 * Type guard to check if a caught value is an Error-like object from Rong.
 *
 * @param error - The error to check
 * @returns true if the value behaves like a RongError object
 *
 * @example
 * ```typescript
 * try {
 *   await Rong.file('/file.txt').text();
 * } catch (error) {
 *   if (isRongError(error)) {
 *     console.error(`Rong error [${error.name}]: ${error.message}`);
 *   }
 * }
 * ```
 */
export function isRongError(error: unknown): error is RongError {
  return (error instanceof Error || error instanceof DOMException) &&
    typeof getField(error, 'name') === 'string' &&
    typeof getField(error, 'message') === 'string';
}

/**
 * Type guard to check if an error is an AbortError.
 */
export function isAbortError(error: unknown): error is AbortError {
  return isRongError(error) && error.name === 'AbortError';
}

/**
 * Type guard to check if an error is a NetworkError.
 */
export function isNetworkError(error: unknown): error is NetworkError {
  return isRongError(error) && error.name === 'NetworkError';
}

/**
 * Returns the `.name` of an Error-like throwable when present.
 */
export function getErrorName(error: unknown): string | undefined {
  const name = getField(error, 'name');
  return typeof name === 'string' ? name : undefined;
}

/**
 * Returns the stable Rong `.code` when present on an Error-like throwable.
 */
export function getErrorCode(error: unknown): string | undefined {
  const code = getField(error, 'code');
  return typeof code === 'string' ? code : undefined;
}

/**
 * Get a human-readable message from any throwable value.
 */
export function getErrorMessage(error: unknown): string {
  if (typeof error === 'string') {
    return error;
  }

  const message = getField(error, 'message');
  if (typeof message === 'string') {
    return message;
  }

  return String(error);
}

/**
 * Get a human-readable error message from any error.
 *
 * @param error - The error to format
 * @returns Formatted error message
 *
 * @example
 * ```typescript
 * try {
 *   await someOperation();
 * } catch (error) {
 *   console.error(formatError(error));
 * }
 * ```
 */
export function formatError(error: unknown): string {
  const name = getErrorName(error);
  const code = getErrorCode(error);
  const message = getErrorMessage(error);

  if (name) {
    const formattedCode = code ? ` ${code}` : '';
    return `[${name}${formattedCode}] ${message}`;
  }

  return message;
}

/**
 * Assert that a value is a RongError-like object, throwing if not.
 *
 * @param error - The value to check
 * @throws {TypeError} If the value is not a RongError-like object
 */
export function assertRongError(error: unknown): asserts error is RongError {
  if (!isRongError(error)) {
    throw new TypeError('Expected RongError');
  }
}

export {};
