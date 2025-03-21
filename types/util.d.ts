/**
 * The `util` module supports the needs of LLRT internal APIs. Many of the
 * utilities are useful for application and module developers as well. To access
 * it:
 *
 * ```js
 * import util from 'util';
 * ```
 * @see [source](https://github.com/nodejs/node/blob/v22.x/lib/util.js)
 */
declare module "util" {
  /**
   * The `util.format()` method returns a formatted string using the first argument
   * as a `printf`-like format string which can contain zero or more format
   * specifiers. Each specifier is replaced with the converted value from the
   * corresponding argument. Supported specifiers are:
   *
   * If a specifier does not have a corresponding argument, it is not replaced:
   *
   * ```js
   * util.format('%s:%s', 'foo');
   * // Returns: 'foo:%s'
   * ```
   *
   * Values that are not part of the format string are formatted using `util.inspect()` if their type is not `string`.
   *
   * If there are more arguments passed to the `util.format()` method than the
   * number of specifiers, the extra arguments are concatenated to the returned
   * string, separated by spaces:
   *
   * ```js
   * util.format('%s:%s', 'foo', 'bar', 'baz');
   * // Returns: 'foo:bar baz'
   * ```
   *
   * If the first argument does not contain a valid format specifier, `util.format()` returns a string that is the concatenation of all arguments separated by spaces:
   *
   * ```js
   * util.format(1, 2, 3);
   * // Returns: '1 2 3'
   * ```
   *
   * If only one argument is passed to `util.format()`, it is returned as it is
   * without any formatting:
   *
   * ```js
   * util.format('%% %s');
   * // Returns: '%% %s'
   * ```
   *
   * `util.format()` is a synchronous method that is intended as a debugging tool.
   * Some input values can have a significant performance overhead that can block the
   * event loop. Use this function with care and never in a hot code path.
   * @param format A `printf`-like format string.
   */
  export function format(format?: any, ...param: any[]): string;
  /**
   * An implementation of the [WHATWG Encoding Standard](https://encoding.spec.whatwg.org/) `TextDecoder` API.
   *
   * ```js
   * const decoder = new TextDecoder();
   * const u8arr = new Uint8Array([72, 101, 108, 108, 111]);
   * console.log(decoder.decode(u8arr)); // Hello
   * ```
   */
  export class TextDecoder {
    /**
     * The encoding supported by the `TextDecoder` instance.
     */
    readonly encoding: string;
    /**
     * The value will be `true` if decoding errors result in a `TypeError` being
     * thrown.
     */
    readonly fatal: boolean;
    /**
     * The value will be `true` if the decoding result will include the byte order
     * mark.
     */
    readonly ignoreBOM: boolean;
    constructor(
      encoding?: string,
      options?: {
        fatal?: boolean | undefined;
        ignoreBOM?: boolean | undefined;
      }
    );
    /**
     * Decodes the `input` and returns a string. If `options.stream` is `true`, any
     * incomplete byte sequences occurring at the end of the `input` are buffered
     * internally and emitted after the next call to `textDecoder.decode()`.
     *
     * If `textDecoder.fatal` is `true`, decoding errors that occur will result in a `TypeError` being thrown.
     * @param input An `ArrayBuffer`, `DataView`, or `TypedArray` instance containing the encoded data.
     */
    decode(
      input?: QuickJS.ArrayBufferView | ArrayBuffer | null,
      options?: {
        stream?: boolean | undefined;
      }
    ): string;
  }
  export interface EncodeIntoResult {
    /**
     * The read Unicode code units of input.
     */
    read: number;
    /**
     * The written UTF-8 bytes of output.
     */
    written: number;
  }
  //// TextEncoder/Decoder
  /**
   * An implementation of the [WHATWG Encoding Standard](https://encoding.spec.whatwg.org/) `TextEncoder` API. All
   * instances of `TextEncoder` only support UTF-8 encoding.
   *
   * ```js
   * const encoder = new TextEncoder();
   * const uint8array = encoder.encode('this is some data');
   * ```
   *
   * The `TextEncoder` class is also available on the global object.
   */
  export class TextEncoder {
    /**
     * The encoding supported by the `TextEncoder` instance. Always set to `'utf-8'`.
     */
    readonly encoding: string;
    /**
     * UTF-8 encodes the `input` string and returns a `Uint8Array` containing the
     * encoded bytes.
     * @param [input='an empty string'] The text to encode.
     */
    encode(input?: string): Uint8Array;
    /**
     * UTF-8 encodes the `src` string to the `dest` Uint8Array and returns an object
     * containing the read Unicode code units and written UTF-8 bytes.
     *
     * ```js
     * const encoder = new TextEncoder();
     * const src = 'this is some data';
     * const dest = new Uint8Array(10);
     * const { read, written } = encoder.encodeInto(src, dest);
     * ```
     * @param src The text to encode.
     * @param dest The array to hold the encode result.
     */
    encodeInto(src: string, dest: Uint8Array): EncodeIntoResult;
  }
  import {
    TextDecoder as _TextDecoder,
    TextEncoder as _TextEncoder,
  } from "util";
  global {
    /**
     * `TextDecoder` class is a global reference for `import { TextDecoder } from 'util'`
     * https://nodejs.org/api/globals.html#textdecoder
     */
    var TextDecoder: typeof globalThis extends {
      onmessage: any;
      TextDecoder: infer TextDecoder;
    }
      ? TextDecoder
      : typeof _TextDecoder;
    /**
     * `TextEncoder` class is a global reference for `import { TextEncoder } from 'util'`
     * https://nodejs.org/api/globals.html#textencoder
     */
    var TextEncoder: typeof globalThis extends {
      onmessage: any;
      TextEncoder: infer TextEncoder;
    }
      ? TextEncoder
      : typeof _TextEncoder;
  }
}
declare module "util" {
  export * from "util";
}
