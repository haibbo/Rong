/**
 * Navigator module type definitions
 * Corresponds to: modules/rong_navigator
 */

// Extend the global Navigator interface with Rong-specific properties
declare global {
  interface Navigator {
    /** User agent string */
    readonly userAgent: string;

    /** Platform identifier (e.g., "macos", "linux", "windows") */
    readonly platform: string;

    /** CPU architecture (e.g., "x86_64", "aarch64") */
    readonly arch: string;
  }
}

export {};
