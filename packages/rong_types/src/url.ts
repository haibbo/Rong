/**
 * URL module type definitions
 * Corresponds to: modules/rong_url
 */

export interface URLSearchParams {
  /** Append a parameter */
  append(name: string, value: string): void;

  /** Delete all parameters with the given name */
  delete(name: string): void;

  /** Get the first value for a parameter */
  get(name: string): string | null;

  /** Get all values for a parameter */
  getAll(name: string): string[];

  /** Check if a parameter exists */
  has(name: string): boolean;

  /** Set a parameter value (replaces existing) */
  set(name: string, value: string): void;

  /** Sort parameters by name */
  sort(): void;

  /** Get all parameter entries */
  entries(): Array<[string, string]>;

  /** Get all parameter names */
  keys(): string[];

  /** Get all parameter values */
  values(): string[];

  /** Iterate over parameters */
  forEach(callback: (value: string, key: string) => void, thisArg?: any): void;

  /** Convert to query string */
  toString(): string;

  /** Number of parameters */
  readonly size: number;
}

export interface URLSearchParamsConstructor {
  new(): URLSearchParams;
  new(init: string): URLSearchParams;
  new(init: Array<[string, string]>): URLSearchParams;
  new(init: Record<string, string>): URLSearchParams;
  prototype: URLSearchParams;
}

export interface URL {
  /** Fragment identifier (e.g., "#section") */
  hash: string;

  /** Hostname with port (e.g., "example.com:8080") */
  host: string;

  /** Hostname only (e.g., "example.com") */
  hostname: string;

  /** Full URL string */
  href: string;

  /** Protocol + host (e.g., "https://example.com") */
  readonly origin: string;

  /** Password component */
  password: string;

  /** Path component (e.g., "/path/to/resource") */
  pathname: string;

  /** Port number as string */
  port: string;

  /** Protocol scheme (e.g., "https:") */
  protocol: string;

  /** Query string (e.g., "?key=value") */
  search: string;

  /** Username component */
  username: string;

  /** URL search parameters interface */
  readonly searchParams: URLSearchParams;

  /** Convert to string */
  toString(): string;

  /** Convert to JSON */
  toJSON(): string;
}

export interface URLConstructor {
  new(url: string, base?: string): URL;
  prototype: URL;
}

// Note: URL and URLSearchParams are provided by the global environment (Web API)
// These type definitions are for reference and extend the standard Web API
export {};
