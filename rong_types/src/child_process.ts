/**
 * Child Process module type definitions
 * Corresponds to: modules/rong_child_process
 */

import type { EventEmitter } from './event';

export interface SpawnOptions {
  /** Current working directory */
  cwd?: string;
  /** Environment variables */
  env?: Record<string, string>;
  /** Use shell to execute command */
  shell?: boolean;
  /** Timeout in milliseconds */
  timeout?: number;
}

export interface ExecOptions {
  /** Current working directory */
  cwd?: string;
  /** Environment variables */
  env?: Record<string, string>;
  /** Timeout in milliseconds */
  timeout?: number;
}

export interface ExecResult {
  /** Exit code (null if not yet exited) */
  code: number | null;
  /** Standard output */
  stdout: string;
  /** Standard error */
  stderr: string;
}

export interface ChildProcess extends EventEmitter {
  /** Process ID (null if process failed to spawn) */
  readonly pid: number | null;

  /** Exit code (available after process exits) */
  readonly exitCode: number | null;

  /** Standard input stream (if configured as 'piped') */
  readonly stdin: WritableStream<Uint8Array> | null;

  /** Standard output stream (if configured as 'piped') */
  readonly stdout: ReadableStream<Uint8Array> | null;

  /** Standard error stream (if configured as 'piped') */
  readonly stderr: ReadableStream<Uint8Array> | null;

  /**
   * Wait for process to exit
   * @returns Exit code (null if terminated by signal)
   */
  wait(): Promise<number | null>;

  /**
   * Kill the process with a signal
   * @param signal - Signal name (e.g., 'SIGTERM', 'SIGKILL') or number
   * @returns true if signal was sent successfully
   */
  kill(signal?: string): boolean;

  // Event emitter methods (Node.js style)
  on(event: 'exit', listener: (code: number | null) => void): this;
  once(event: 'exit', listener: (code: number | null) => void): this;
  off(event: 'exit', listener: (code: number | null) => void): this;
}

export interface ChildProcessModule {
  /**
   * Spawn a child process
   * @param command - Command to execute
   * @param args - Command arguments (optional)
   * @param options - Spawn options
   *
   * @example
   * ```typescript
   * const child = child_process.spawn('ls', ['-la'], { cwd: '/tmp' });
   * const code = await child.wait();
   * ```
   */
  spawn(command: string, args?: string[], options?: SpawnOptions): ChildProcess;

  /**
   * Execute a shell command and capture output
   * @param command - Shell command to execute
   * @param options - Execution options
   *
   * @example
   * ```typescript
   * const result = await child_process.exec('echo "Hello"', { timeout: 5000 });
   * console.log(result.stdout);
   * ```
   */
  exec(command: string, options?: ExecOptions): Promise<ExecResult>;

  /**
   * Execute a file directly without shell
   * @param file - File path to execute
   * @param args - Command arguments
   * @param options - Execution options
   *
   * @example
   * ```typescript
   * const result = await child_process.execFile('/usr/bin/node', ['--version']);
   * console.log(result.stdout);
   * ```
   */
  execFile(file: string, args?: string[], options?: ExecOptions): Promise<ExecResult>;
}

// Note: ChildProcess module is mounted as globalThis.child_process
export {};
