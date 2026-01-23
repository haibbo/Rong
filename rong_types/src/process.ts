/**
 * Process module type definitions
 * Corresponds to: modules/rong_process
 */

import type { EventEmitter } from './event';

export interface ProcessEnv {
  [key: string]: string | undefined;
}

export interface ProcessStdin extends ReadableStream<Uint8Array> {
  readonly isTTY: boolean;
}

export interface ProcessStdout {
  write(data: string): boolean;
  readonly isTTY: boolean;
}

export interface ProcessStderr {
  write(data: string): boolean;
  readonly isTTY: boolean;
}

export interface Process extends EventEmitter {
  /** Process ID */
  readonly pid: number;

  /** Current working directory */
  cwd(): string;

  /** Change working directory */
  chdir(directory: string): void;

  /** Environment variables */
  readonly env: ProcessEnv;

  /** Platform (e.g., 'darwin', 'linux', 'win32') */
  readonly platform: string;

  /** CPU architecture (e.g., 'x64', 'arm64') */
  readonly arch: string;

  /** Process version */
  readonly version: string;

  /** Command line arguments */
  readonly argv: string[];

  /** Exit the process */
  exit(code?: number): never;

  /** Process uptime in seconds */
  uptime(): number;

  /** High-resolution real time - returns [seconds, nanoseconds] since arbitrary point */
  hrtime(prev?: [number, number]): [number, number];

  /** Schedule callback for next tick (microtask) */
  nextTick(callback: (...args: any[]) => void, ...args: any[]): void;

  /** Standard input stream */
  readonly stdin: ProcessStdin;

  /** Standard output stream */
  readonly stdout: ProcessStdout;

  /** Standard error stream */
  readonly stderr: ProcessStderr;
}

// Note: process is declared as a global in global.d.ts
export {};
