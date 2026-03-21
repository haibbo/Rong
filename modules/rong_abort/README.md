# rong_abort

Implements the Web AbortController and AbortSignal APIs for cancellation signaling.

## JS APIs

- `AbortController` — controls an abort signal
  - `signal` — the associated `AbortSignal`
  - `abort(reason?)` — abort the signal with an optional reason
- `AbortSignal` — represents a cancellation signal (extends `EventTarget`)
  - `aborted` — whether the signal has been aborted
  - `reason` — the abort reason, if any
  - `onabort` — abort event handler
  - `throwIfAborted()` — throws the abort reason if aborted
  - `AbortSignal.any(signals)` — returns a signal that aborts when any input signal aborts
  - `AbortSignal.abort(reason?)` — returns an already-aborted signal
  - `AbortSignal.timeout(ms)` — returns a signal that aborts after the given milliseconds
