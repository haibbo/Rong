/**
 * Rong JavaScript Runtime Type Definitions
 *
 * This package provides TypeScript type definitions for all Rust-driven
 * JavaScript APIs in the Rong runtime.
 *
 * ## Quick Start
 *
 * Add to your tsconfig.json:
 * ```json
 * {
 *   "compilerOptions": {
 *     "types": ["@lingxia/rong"]
 *   }
 * }
 * ```
 *
 * Then use the global APIs:
 * ```typescript
 * // File system
 * const text = await Rong.file('/path/to/file.txt').text();
 * await Rong.write('/output.txt', 'Hello World');
 *
 * // Process
 * console.log(process.pid);
 * console.log(process.env.PATH);
 *
 * // HTTP
 * const response = await fetch('https://api.example.com');
 *
 * // Child process
 * const child = child_process.spawn('ls', ['-la']);
 * ```
 *
 * For detailed API documentation, see individual module exports below.
 */

// Global API declarations - Import this for full IDE autocomplete
import './global';

// Core runtime modules
export * from './process';
export * from './child_process';
export * from './stream';
export * from './encoding';
export * from './storage';
export * from './http';

// File system
export * from './fs';

// Web APIs
export * from './url';
export * from './buffer';
export * from './event';
export * from './abort';
export * from './exception';

// Utility modules
export * from './navigator';
export * from './timer';
export * from './path';
export * from './assert';
export * from './console';

// Error types
export * from './error';
