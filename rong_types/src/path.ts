/**
 * Path module type definitions
 * Corresponds to: modules/rong_path
 */

export interface ParsedPath {
  /** Root directory (e.g., "/" on Unix, "C:\\" on Windows) */
  root: string;
  /** Directory path */
  dir: string;
  /** File name with extension */
  base: string;
  /** File extension (including the dot) */
  ext: string;
  /** File name without extension */
  name: string;
}

export interface PathModule {
  /**
   * Returns the last portion of a path
   * @param path - The path to process
   * @param suffix - Optional suffix to remove from the result
   */
  basename(path: string, suffix?: string): string;

  /**
   * Returns the directory name of a path
   * @param path - The path to process
   */
  dirname(path: string): string;

  /**
   * Returns the extension of a path (including the dot)
   * @param path - The path to process
   */
  extname(path: string): string;

  /**
   * Determines whether a path is absolute
   * @param path - The path to check
   */
  isAbsolute(path: string): boolean;

  /**
   * Joins path segments together
   * @param paths - Path segments to join
   */
  join(...paths: string[]): string;

  /**
   * Normalizes a path by resolving '..' and '.' segments
   * @param path - The path to normalize
   */
  normalize(path: string): string;

  /**
   * Resolves a sequence of paths to an absolute path
   * @param paths - Paths to resolve
   */
  resolve(...paths: string[]): string;

  /**
   * Parses a path into an object
   * @param path - The path to parse
   */
  parse(path: string): ParsedPath;

  /**
   * Formats a path object into a path string
   * @param pathObject - Path object to format
   */
  format(pathObject: Partial<ParsedPath>): string;

  /** Platform-specific path segment separator ("/" on Unix, "\\" on Windows) */
  readonly sep: string;

  /** Platform-specific PATH delimiter (":" on Unix, ";" on Windows) */
  readonly delimiter: string;
}

// Note: path is declared as a global in global.d.ts
export {};
