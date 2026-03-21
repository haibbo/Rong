/**
 * Stream module type definitions
 * Corresponds to: modules/rong_stream
 *
 * Rong runtime provides the Web Streams API, which is a standard for handling
 * streaming data in JavaScript. These APIs are globally available.
 *
 * ## Available Stream Types
 *
 * ### ReadableStream<T>
 * Represents a readable stream of data.
 *
 * Usage:
 * ```typescript
 * // From file
 * const file = await Rong.file('/path/to/file.txt').open({ read: true });
 * const readable = file.readable;
 *
 * // Read from stream
 * const reader = readable.getReader();
 * while (true) {
 *   const { done, value } = await reader.read();
 *   if (done) break;
 *   console.log('Received:', value);
 * }
 * ```
 *
 * ### WritableStream<T>
 * Represents a writable stream of data.
 *
 * Usage:
 * ```typescript
 * // To file
 * const file = await Rong.file('/path/to/output.txt').open({ write: true, create: true });
 * const writable = file.writable;
 *
 * // Write to stream
 * const writer = writable.getWriter();
 * await writer.write(new TextEncoder().encode('Hello World'));
 * await writer.close();
 * ```
 *
 * ### TransformStream<I, O>
 * Represents a transform stream for processing data.
 *
 * Usage:
 * ```typescript
 * // Pipe with transformation
 * await readable
 *   .pipeThrough(new TransformStream({
 *     transform(chunk, controller) {
 *       // Process chunk
 *       controller.enqueue(processedChunk);
 *     }
 *   }))
 *   .pipeTo(writable);
 * ```
 *
 * ## Common Use Cases
 *
 * ### Pipe streams
 * ```typescript
 * // Pipe stdin to stdout
 * await process.stdin.pipeTo(process.stdout);
 *
 * // Pipe file to file
 * const source = await Rong.file('/source.txt').open({ read: true });
 * const dest = await Rong.file('/dest.txt').open({ write: true, create: true });
 * await source.readable.pipeTo(dest.writable);
 * ```
 *
 * ### Process data in chunks
 * ```typescript
 * const file = await Rong.file('/large-file.txt').open({ read: true });
 * const reader = file.readable.getReader();
 *
 * let totalBytes = 0;
 * while (true) {
 *   const { done, value } = await reader.read();
 *   if (done) break;
 *   totalBytes += value.byteLength;
 *   // Process chunk...
 * }
 * console.log(`Processed ${totalBytes} bytes`);
 * ```
 *
 * @see https://developer.mozilla.org/en-US/docs/Web/API/Streams_API
 */

// Stream types are provided globally by the DOM library
// When you include "DOM" in tsconfig.json lib array, these types are available:
// - ReadableStream<T>
// - WritableStream<T>
// - TransformStream<I, O>
// - ReadableStreamDefaultReader<T>
// - WritableStreamDefaultWriter<T>

export {};
