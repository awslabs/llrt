// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

export {};

declare global {
  /**
   * Defined when LLRT runs a Lambda function, as in the Node.js runtime.
   */
  namespace awslambda {
    /**
     * The `stream.Writable` passed to a streaming handler. What is written to
     * it is streamed to the caller.
     */
    interface ResponseStream extends QuickJS.WritableStream {
      /**
       * Sets the response's `Content-Type`, `application/octet-stream` by
       * default. Throws once the first chunk has been written.
       */
      setContentType(contentType: string): void;

      /**
       * Strings and `Uint8Array`s (including `Buffer`s) are written as they
       * are. Any other value is written as JSON, as in the Node.js runtime.
       *
       * @return `false` if the caller should wait for the `'drain'` event
       * before writing more.
       */
      write(chunk: unknown, callback?: (error?: Error | null) => void): boolean;
      write(
        chunk: unknown,
        encoding: string,
        callback?: (error?: Error | null) => void
      ): boolean;

      /**
       * Ends the response. Unlike `write`, `end` doesn't convert a chunk to
       * JSON.
       */
      end(callback?: () => void): this;
      end(chunk: string | Uint8Array, callback?: () => void): this;
      end(
        chunk: string | Uint8Array,
        encoding: string,
        callback?: () => void
      ): this;

      /**
       * Ends the response with an error, which is sent to Lambda in the
       * response's error trailers.
       */
      destroy(error?: Error): this;

      readonly destroyed: boolean;
      readonly writableHighWaterMark: number;
    }

    type StreamingHandler<TEvent = any, TContext = any> = (
      event: TEvent,
      responseStream: ResponseStream,
      context: TContext
    ) => Promise<void>;

    interface StreamifyResponseOptions {
      /**
       * The response stream's buffer size in bytes. Defaults to 16 KiB.
       */
      highWaterMark?: number;
    }

    /**
     * Marks a handler for Lambda response streaming. The handler gets a
     * `ResponseStream` as its second argument, and the invocation ends once
     * its promise has settled and the stream has ended.
     *
     * ```js
     * export const handler = awslambda.streamifyResponse(
     *   async (event, responseStream) => {
     *     responseStream.setContentType("text/plain");
     *     responseStream.write("Hello, ");
     *     responseStream.end("world!");
     *   }
     * );
     * ```
     */
    function streamifyResponse<T extends StreamingHandler>(
      handler: T,
      options?: StreamifyResponseOptions
    ): T;
  }
}
