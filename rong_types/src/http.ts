/**
 * HTTP module type definitions
 * Corresponds to: modules/rong_http
 *
 * The HTTP module provides the standard `fetch` API for making HTTP requests.
 * It does NOT provide an `http` object or `download` function.
 */

export interface FetchOptions {
  method?: string;
  headers?: HeadersInit | Headers;
  body?: BodyInit | null;
  signal?: AbortSignal | null;
  redirect?: 'follow' | 'error' | 'manual';
}

export interface FetchResponse {
  /** HTTP status code */
  readonly status: number;

  /** HTTP status text */
  readonly statusText: string;

  /** Whether the response was successful (status in 200-299) */
  readonly ok: boolean;

  /** Response headers */
  readonly headers: Headers;

  /** Whether the response body has been read */
  readonly bodyUsed: boolean;

  /** Response type */
  readonly type: string;

  /** Whether the response was redirected */
  readonly redirected: boolean;

  /** Response URL */
  readonly url: string;

  /** Parse response body as text */
  text(): Promise<string>;

  /** Parse response body as JSON */
  json<T = any>(): Promise<T>;

  /** Get response body as ArrayBuffer */
  arrayBuffer(): Promise<ArrayBuffer>;

  /** Get response body as Blob */
  blob(): Promise<Blob>;

  /** Get response body as FormData */
  formData(): Promise<FormData>;

  /** Get response body as ReadableStream */
  readonly body: ReadableStream<Uint8Array> | null;
}

declare global {
  /**
   * Fetch API - Make HTTP requests
   *
   * @example
   * ```typescript
   * const response = await fetch('https://api.example.com/data', {
   *   method: 'POST',
   *   headers: { 'Content-Type': 'application/json' },
   *   body: JSON.stringify({ key: 'value' })
   * });
   * const data = await response.json();
   * ```
   */
  function fetch(url: RequestInfo | URL, options?: RequestInit): Promise<Response>;

}

export {};

export type BodyInit =
  | string
  | Blob
  | ArrayBuffer
  | ArrayBufferView
  | FormData
  | URLSearchParams
  | ReadableStream<Uint8Array>;

export interface Body {
  /** Consumes the body and returns a promise that resolves with a Blob */
  blob(): Promise<Blob>;
  /** Consumes the body and returns a promise that resolves with a FormData */
  formData(): Promise<FormData>;
  /** Consumes the body and returns a promise that resolves with the result of parsing the body text as JSON */
  json<T = any>(): Promise<T>;
  /** Consumes the body and returns a promise that resolves with the result of parsing the body text as a String */
  text(): Promise<string>;
  /** Consumes the body and returns a promise that resolves with an ArrayBuffer */
  arrayBuffer(): Promise<ArrayBuffer>;
  /** Returns a boolean indicating whether body has been consumed */
  readonly bodyUsed: boolean;
  /** The body content */
  readonly body: ReadableStream<Uint8Array> | null;
}

export interface RequestInit {
  method?: string;
  headers?: HeadersInit | Headers;
  body?: BodyInit | null;
  redirect?: 'follow' | 'error' | 'manual';
  signal?: AbortSignal | null;
}

export interface Request extends Body {
  readonly method: string;
  readonly headers: Headers;
  readonly redirect: string;
  readonly signal: AbortSignal | null;
  readonly url: string;
  clone(): Request;
}

export interface RequestConstructor {
  new(input: RequestInfo | string, init?: RequestInit): Request;
  prototype: Request;
}

export type RequestInfo = string | Request | URL;

export type HeadersInit = Record<string, string> | Array<[string, string]> | Headers;

export interface Headers {
  append(name: string, value: string): void;
  delete(name: string): void;
  get(name: string): string | null;
  has(name: string): boolean;
  set(name: string, value: string): void;
  forEach(callback: (value: string, name: string, self: Headers) => void, thisArg?: any): void;
  entries(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;

  /** Returns all Set-Cookie values (Rong extension) */
  getSetCookie(): string[];
}

export interface HeadersConstructor {
  new(init?: HeadersInit): Headers;
  prototype: Headers;
}
