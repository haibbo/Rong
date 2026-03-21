/**
 * Error type definitions for Rong runtime
 *
 * Rong can throw:
 * - standard JS `Error` objects (including `TypeError`, `RangeError`, etc.)
 * - `DOMException` instances for Abort-related errors (e.g. `AbortError`)
 *
 * This module provides type guards and utilities for error handling.
 */

/**
 * Common error names used throughout the Rong runtime (as `error.name`).
 */
export type RongErrorName =
  | 'Error'
  | 'TypeError'
  | 'RangeError'
  | 'ReferenceError'
  | 'AbortError'
  | 'NetworkError';

/**
 * Base error interface for Rong runtime errors.
 */
export interface RongError extends Error {
  /** Error name identifying the error type */
  readonly name: RongErrorName;

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

// ==================== Type Guards ====================

/**
 * Type guard to check if an error is a Rong runtime error.
 *
 * @param error - The error to check
 * @returns true if the error is a RongError
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
    typeof (error as { name?: unknown }).name === 'string';
}

/**
 * Type guard to check if an error is an AbortError.
 */
export function isAbortError(error: unknown): error is AbortError {
  return (error instanceof Error || error instanceof DOMException) &&
    (error as { name?: unknown }).name === 'AbortError';
}

/**
 * Type guard to check if an error is a NetworkError.
 */
export function isNetworkError(error: unknown): error is NetworkError {
  return (error instanceof Error || error instanceof DOMException) &&
    (error as { name?: unknown }).name === 'NetworkError';
}

// ==================== Utility Functions ====================

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
  if (isRongError(error)) {
    const code = error.code ? ` ${error.code}` : '';
    return `[${error.name}${code}] ${error.message}`;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

/**
 * Assert that a value is a RongError, throwing if not.
 *
 * @param error - The value to check
 * @throws {TypeError} If the value is not a RongError
 */
export function assertRongError(error: unknown): asserts error is RongError {
  if (!isRongError(error)) {
    throw new TypeError('Expected RongError');
  }
}

export {};
