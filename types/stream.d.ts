declare module "stream" {
  import { EventEmitter } from "events";
  import { Buffer } from "buffer";

  class ReadableStreamInner
    extends EventEmitter
    implements QuickJS.ReadableStream
  {
    /**
     * The `readable.read()` method reads data out of the internal buffer and
     * returns it. If no data is available to be read, `null` is returned. By default,
     * the data is returned as a `Buffer` object unless an encoding has been
     * specified using the `readable.setEncoding()` method or the stream is operating
     * in object mode.
     *
     * The optional `size` argument specifies a specific number of bytes to read. If
     * `size` bytes are not available to be read, `null` will be returned _unless_ the
     * stream has ended, in which case all of the data remaining in the internal buffer
     * will be returned.
     *
     * If the `size` argument is not specified, all of the data contained in the
     * internal buffer will be returned.
     *
     * The `size` argument must be less than or equal to 1 GiB.
     *
     * The `readable.read()` method should only be called on `Readable` streams
     * operating in paused mode. In flowing mode, `readable.read()` is called
     * automatically until the internal buffer is fully drained.
     *
     * ```js
     * const readable = getReadableStreamSomehow();
     *
     * // 'readable' may be triggered multiple times as data is buffered in
     * readable.on('readable', () => {
     *   let chunk;
     *   console.log('Stream is readable (new data received in buffer)');
     *   // Use a loop to make sure we read all currently available data
     *   while (null !== (chunk = readable.read())) {
     *     console.log(`Read ${chunk.length} bytes of data...`);
     *   }
     * });
     *
     * // 'end' will be triggered once when there is no more data available
     * readable.on('end', () => {
     *   console.log('Reached end of stream.');
     * });
     * ```
     *
     * Each call to `readable.read()` returns a chunk of data, or `null`. The chunks
     * are not concatenated. A `while` loop is necessary to consume all data
     * currently in the buffer. When reading a large file `.read()` may return `null`,
     * having consumed all buffered content so far, but there is still more data to
     * come not yet buffered. In this case a new `'readable'` event will be emitted
     * when there is more data in the buffer. Finally the `'end'` event will be
     * emitted when there is no more data to come.
     *
     * Therefore to read a file's whole contents from a `readable`, it is necessary
     * to collect chunks across multiple `'readable'` events:
     *
     * ```js
     * const chunks = [];
     *
     * readable.on('readable', () => {
     *   let chunk;
     *   while (null !== (chunk = readable.read())) {
     *     chunks.push(chunk);
     *   }
     * });
     *
     * readable.on('end', () => {
     *   const content = chunks.join('');
     * });
     * ```
     *
     * A `Readable` stream in object mode will always return a single item from
     * a call to `readable.read(size)`, regardless of the value of the `size` argument.
     *
     * If the `readable.read()` method returns a chunk of data, a `'data'` event will
     * also be emitted.
     *
     * Calling {@link read} after the `'end'` event has
     * been emitted will return `null`. No runtime error will be raised.
     * @param size Optional argument to specify how much data to read.
     */
    read(size?: number): Buffer | null;

    /**
     * Destroy the stream. Optionally emit an `'error'` event, and emit a `'close'` event. After this call, the readable
     * stream will release any internal resources and subsequent calls to `push()` will be ignored.
     *
     * Once `destroy()` has been called any further calls will be a no-op and no
     * further errors except from `_destroy()` may be emitted as `'error'`.
     *
     * Implementors should not override this method, but instead implement `readable._destroy()`.
     * @param error Error which will be passed as payload in `'error'` event
     */
    destroy(error?: Error): this;

    /**
     * Event emitter
     * The defined events on documents including:
     * 1. close
     * 2. data
     * 3. end
     * 4. error
     * 5. readable
     */
    addListener(event: EventKey, listener: (...args: any[]) => void): this;
    addListener(event: "close", listener: () => void): this;
    addListener(event: "data", listener: (chunk: Buffer) => void): this;
    addListener(event: "end", listener: () => void): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    addListener(event: "readable", listener: () => void): this;
    emit(event: EventKey, ...args: any[]): boolean;
    emit(event: "close"): boolean;
    emit(event: "data", chunk: Buffer): boolean;
    emit(event: "end"): boolean;
    emit(event: "error", err: Error): boolean;
    emit(event: "readable"): boolean;
    on(event: EventKey, listener: (...args: any[]) => void): this;
    on(event: "close", listener: () => void): this;
    on(event: "data", listener: (chunk: Buffer) => void): this;
    on(event: "end", listener: () => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(event: "readable", listener: () => void): this;
    once(event: EventKey, listener: (...args: any[]) => void): this;
    once(event: "close", listener: () => void): this;
    once(event: "data", listener: (chunk: Buffer) => void): this;
    once(event: "end", listener: () => void): this;
    once(event: "error", listener: (err: Error) => void): this;
    once(event: "readable", listener: () => void): this;
    prependListener(event: EventKey, listener: (...args: any[]) => void): this;
    prependListener(event: "close", listener: () => void): this;
    prependListener(event: "data", listener: (chunk: Buffer) => void): this;
    prependListener(event: "end", listener: () => void): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependListener(event: "readable", listener: () => void): this;
    prependOnceListener(
      event: EventKey,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(event: "close", listener: () => void): this;
    prependOnceListener(event: "data", listener: (chunk: Buffer) => void): this;
    prependOnceListener(event: "end", listener: () => void): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(event: "readable", listener: () => void): this;
    removeListener(event: EventKey, listener: (...args: any[]) => void): this;
    removeListener(event: "close", listener: () => void): this;
    removeListener(event: "data", listener: (chunk: Buffer) => void): this;
    removeListener(event: "end", listener: () => void): this;
    removeListener(event: "error", listener: (err: Error) => void): this;
    removeListener(event: "readable", listener: () => void): this;

    /**
     * Calls `readable.destroy()`.
     */
    [Symbol.dispose](): void;
  }

  class WritableStreamInner
    extends EventEmitter
    implements QuickJS.WritableStream
  {
    /**
     * The `writable.write()` method writes some data to the stream, and calls the
     * supplied `callback` once the data has been fully handled. If an error
     * occurs, the `callback` will be called with the error as its
     * first argument. The `callback` is usually called asynchronously and before `'error'`
     * is emitted.
     *
     * ```js
     * function write(data, cb) {
     *   if (!stream.write(data)) {
     *     stream.once('drain', cb);
     *   } else {
     *     process.nextTick(cb);
     *   }
     * }
     *
     * // Wait for cb to be called before doing any other write.
     * write('hello', () => {
     *   console.log('Write completed, do more writes now.');
     * });
     * ```
     *
     * A `Writable` stream in object mode will always ignore the `encoding` argument.
     * @since v0.9.4
     * @param chunk Optional data to write. `chunk` must be a {string}, {Buffer}, {TypedArray} or {DataView}.
     * @param [encoding='utf8'] The encoding, if `chunk` is a string.
     * @param callback Callback for when this chunk of data is flushed.
     * @return `false` if the stream wishes for the calling code to wait for the `'drain'` event to be emitted before continuing to write additional data; otherwise `true`.
     */
    write(
      chunk:
        | string
        | Buffer
        | QuickJS.ArrayBufferView
        | ArrayBuffer
        | SharedArrayBuffer,
      callback?: (error?: Error | null) => void
    ): void;

    /**
     * Calling the `writable.end()` method signals that no more data will be written
     * to the `Writable`.
     *
     * Calling the {@link write} method after calling {@link end} will raise an error.
     */
    end(): this;

    /**
     * Event emitter
     * The defined events on documents including:
     * 1. close
     * 2. error
     * 3. finish
     */
    addListener(event: EventKey, listener: (...args: any[]) => void): this;
    addListener(event: "close", listener: () => void): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    addListener(event: "finish", listener: () => void): this;
    emit(event: EventKey, ...args: any[]): boolean;
    emit(event: "close"): boolean;
    emit(event: "error", err: Error): boolean;
    emit(event: "finish"): boolean;
    on(event: EventKey, listener: (...args: any[]) => void): this;
    on(event: "close", listener: () => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(event: "finish", listener: () => void): this;
    once(event: EventKey, listener: (...args: any[]) => void): this;
    once(event: "close", listener: () => void): this;
    once(event: "error", listener: (err: Error) => void): this;
    once(event: "finish", listener: () => void): this;
    prependListener(event: EventKey, listener: (...args: any[]) => void): this;
    prependListener(event: "close", listener: () => void): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependListener(event: "finish", listener: () => void): this;
    prependOnceListener(
      event: EventKey,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(event: "close", listener: () => void): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(event: "finish", listener: () => void): this;
    removeListener(event: EventKey, listener: (...args: any[]) => void): this;
    removeListener(event: "close", listener: () => void): this;
    removeListener(event: "error", listener: (err: Error) => void): this;
    removeListener(event: "finish", listener: () => void): this;
  }

  class DefaultReadableStream extends ReadableStreamInner {}

  class DefaultWritableStream extends WritableStreamInner {}

  global {
    namespace QuickJS {
      interface ReadableStream extends EventEmitter {
        read(size?: number): Buffer | null;
      }

      interface WritableStream extends EventEmitter {
        write(
          chunk:
            | string
            | Buffer
            | QuickJS.ArrayBufferView
            | ArrayBuffer
            | SharedArrayBuffer,
          callback?: (err?: Error | null) => void
        ): void;
        end(): this;
      }
    }
  }
}
