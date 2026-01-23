/**
 * Assert module type definitions
 * Corresponds to: modules/rong_assert
 */

export type AssertionErrorMessage = string | Error;

export interface AssertFunction {
  /**
   * Assert that a value is truthy
   * @param value - Value to check
   * @param message - Optional error message
   * @throws Error if value is falsy
   */
  (value: any, message?: AssertionErrorMessage): asserts value;

  /**
   * Assert that a value is truthy (alias)
   * @param value - Value to check
   * @param message - Optional error message
   * @throws Error if value is falsy
   */
  ok(value: any, message?: AssertionErrorMessage): asserts value;

  /**
   * Assert that two values are equal
   * @param left - Left value
   * @param right - Right value
   * @param message - Optional error message
   * @throws Error if values are not equal
   */
  equal(left: any, right: any, message?: AssertionErrorMessage): void;

  /**
   * Force assertion failure
   * @param message - Error message
   * @throws Error always
   */
  fail(message?: AssertionErrorMessage): never;

  /**
   * Assert that a function does not throw
   * @param fn - Function to execute
   * @param message - Optional error message
   * @throws Error if function throws
   */
  doesNotThrow(fn: () => void, message?: AssertionErrorMessage): void;
}

// Note: assert is declared as a global in global.d.ts
export {};
