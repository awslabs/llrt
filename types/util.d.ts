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
   * Usage of `util.inherits()` is discouraged. Please use the ES6 `class` and `extends` keywords to get language level inheritance support. Also note
   * that the two styles are [semantically incompatible](https://github.com/nodejs/node/issues/4179).
   *
   * Inherit the prototype methods from one [constructor](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/constructor) into another. The
   * prototype of `constructor` will be set to a new object created from `superConstructor`.
   *
   * This mainly adds some input validation on top of`Object.setPrototypeOf(constructor.prototype, superConstructor.prototype)`.
   * As an additional convenience, `superConstructor` will be accessible
   * through the `constructor.super_` property.
   *
   * ```js
   * import util from 'util';
   * import EventEmitter from 'events';
   *
   * function MyStream() {
   *   EventEmitter.call(this);
   * }
   *
   * util.inherits(MyStream, EventEmitter);
   *
   * MyStream.prototype.write = function(data) {
   *   this.emit('data', data);
   * };
   *
   * const stream = new MyStream();
   *
   * console.log(stream instanceof EventEmitter); // true
   * console.log(MyStream.super_ === EventEmitter); // true
   *
   * stream.on('data', (data) => {
   *   console.log(`Received data: "${data}"`);
   * });
   * stream.write('It works!'); // Received data: "It works!"
   * ```
   *
   * ES6 example using `class` and `extends`:
   *
   * ```js
   * import EventEmitter from 'events';
   *
   * class MyStream extends EventEmitter {
   *   write(data) {
   *     this.emit('data', data);
   *   }
   * }
   *
   * const stream = new MyStream();
   *
   * stream.on('data', (data) => {
   *   console.log(`Received data: "${data}"`);
   * });
   * stream.write('With ES6');
   * ```
   * @legacy Use ES2015 class syntax and `extends` keyword instead.
   */
  export function inherits(
    constructor: unknown,
    superConstructor: unknown
  ): void;

  /**
   * Options for `util.inspect()`.
   */
  export interface InspectOptions {
    /**
     * If `true`, includes non-enumerable symbols and properties.
     * @default false
     */
    showHidden?: boolean | undefined;
    /**
     * Specifies the number of times to recurse while formatting object.
     * `null` or `Infinity` means recurse up to maximum call stack size.
     * @default 2
     */
    depth?: number | null | undefined;
    /**
     * If `true`, the output is styled with ANSI color codes.
     * @default false
     */
    colors?: boolean | undefined;
    /**
     * If `false`, `[util.inspect.custom](depth, opts, inspect)` functions are not invoked.
     * @default true
     */
    customInspect?: boolean | undefined;
    /**
     * Specifies the maximum number of `Array`, `TypedArray`, `Map`, `Set`, `WeakMap`,
     * and `WeakSet` elements to include when formatting.
     * @default 100
     */
    maxArrayLength?: number | null | undefined;
    /**
     * Specifies the maximum number of characters to include when formatting strings.
     * @default 10000
     */
    maxStringLength?: number | null | undefined;
    /**
     * The length at which input values are split across multiple lines.
     * @default 80
     */
    breakLength?: number | undefined;
    /**
     * If set to `true` or a function, all properties of an object, and `Set` and `Map`
     * entries are sorted in the resulting string.
     * @default false
     */
    sorted?: boolean | ((a: string, b: string) => number) | undefined;
    /**
     * Setting this to a number specifies how many layers deep the object will be formatted.
     * Setting this to `false` causes every layer to be formatted on a new line.
     * @default 3
     */
    compact?: boolean | number | undefined;
  }

  /**
   * The `util.inspect()` method returns a string representation of `object` that is
   * intended for debugging. The output of `util.inspect` may change at any time
   * and should not be depended upon programmatically.
   *
   * ```js
   * import util from 'util';
   *
   * console.log(util.inspect({ a: 1, b: { c: 2 } }));
   * // { a: 1, b: { c: 2 } }
   *
   * console.log(util.inspect({ a: 1, b: { c: 2 } }, { depth: 0 }));
   * // { a: 1, b: [Object] }
   *
   * console.log(util.inspect({ a: 1, b: { c: 2 } }, { colors: true }));
   * // { a: 1, b: { c: 2 } } (with colors)
   * ```
   * @param object Any JavaScript primitive or Object.
   * @param options Options to control the formatting.
   */
  export function inspect(object: unknown, options?: InspectOptions): string;

  /**
   * The `util.inspect()` method returns a string representation of `object` that is
   * intended for debugging.
   * @param object Any JavaScript primitive or Object.
   * @param showHidden If `true`, includes non-enumerable symbols and properties.
   * @param depth Specifies the number of times to recurse.
   * @param colors If `true`, the output is styled with ANSI color codes.
   */
  export function inspect(
    object: unknown,
    showHidden?: boolean,
    depth?: number | null,
    colors?: boolean
  ): string;

  export namespace inspect {
    /**
     * A Symbol that can be used to declare custom inspect functions.
     * This is the same as `Symbol.for('nodejs.util.inspect.custom')`.
     */
    const custom: unique symbol;

    /**
     * The default options for `util.inspect`. These can be modified to change
     * the default behavior of `util.inspect`.
     */
    let defaultOptions: InspectOptions;
  }
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
