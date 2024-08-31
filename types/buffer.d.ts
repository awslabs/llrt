/**
 * `Buffer` objects are used to represent a fixed-length sequence of bytes. Many
 * LLRT APIs support `Buffer`s.
 *
 * The `Buffer` class is a subclass of JavaScript's [`Uint8Array`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array) class and
 * extends it with methods that cover additional use cases. LLRT APIs accept
 * plain [`Uint8Array`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array) s wherever `Buffer`s are supported as well.
 *
 * While the `Buffer` class is available within the global scope, it is still
 * recommended to explicitly reference it via an import or require statement.
 *
 * ```js
 * import { Buffer } from 'buffer';
 *
 * // Creates a zero-filled Buffer of length 10.
 * const buf1 = Buffer.alloc(10);
 *
 * // Creates a Buffer of length 10,
 * // filled with bytes which all have the value `1`.
 * const buf2 = Buffer.alloc(10, 1);
 *
 * // Creates a Buffer containing the bytes [1, 2, 3].
 * const buf4 = Buffer.from([1, 2, 3]);
 *
 * // Creates a Buffer containing the bytes [1, 1, 1, 1] – the entries
 * // are all truncated using `(value &#x26; 255)` to fit into the range 0–255.
 * const buf5 = Buffer.from([257, 257.5, -255, '1']);
 *
 * // Creates a Buffer containing the UTF-8-encoded bytes for the string 'tést':
 * // [0x74, 0xc3, 0xa9, 0x73, 0x74] (in hexadecimal notation)
 * // [116, 195, 169, 115, 116] (in decimal notation)
 * const buf6 = Buffer.from('tést');
 *
 * // Creates a Buffer containing the Latin-1 bytes [0x74, 0xe9, 0x73, 0x74].
 * const buf7 = Buffer.from('tést', 'latin1');
 * ```
 */
declare module "buffer" {
  export type BufferEncoding =
    | "hex"
    | "base64"
    | "utf-8"
    | "utf8"
    | "unicode-1-1-utf8"
    | "utf-16le"
    | "utf16le"
    | "utf-16"
    | "utf16"
    | "utf-16be"
    | "utf16be"
    | "windows-1252"
    | "ansi_x3.4-1968"
    | "ascii"
    | "cp1252"
    | "cp819"
    | "csisolatin1"
    | "ibm819"
    | "iso-8859-1"
    | "iso-ir-100"
    | "iso8859-1"
    | "iso88591"
    | "iso_8859-1"
    | "iso_8859-1:1987"
    | "l1"
    | "latin1"
    | "us-ascii"
    | "x-cp1252";
  type WithImplicitCoercion<T> =
    | T
    | {
        valueOf(): T;
      };
  interface BufferConstructor {
    /**
     * Allocates a new `Buffer` of `size` bytes. If `fill` is `undefined`, the `Buffer` will be zero-filled.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.alloc(5);
     *
     * console.log(buf);
     * // Prints: <Buffer 00 00 00 00 00>
     * ```
     *
     * If `fill` is specified, the allocated `Buffer` will be initialized by calling `Buffer.alloc(size, fill)`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.alloc(5, 'a');
     *
     * console.log(buf);
     * // Prints: <Buffer 61 61 61 61 61>
     * ```
     *
     * If both `fill` and `encoding` are specified, the allocated `Buffer` will be
     * initialized by calling `Buffer.aloc(size, fill, encoding)`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.alloc(11, 'aGVsbG8gd29ybGQ=', 'base64');
     *
     * console.log(buf);
     * // Prints: <Buffer 68 65 6c 6c 6f 20 77 6f 72 6c 64>
     * ```
     *
     * @param size The desired length of the new `Buffer`.
     * @param [fill=0] A value to pre-fill the new `Buffer` with.
     * @param [encoding='utf8'] If `fill` is a string, this is its encoding.
     */
    alloc(
      size: number,
      fill?: string | Uint8Array | number,
      encoding?: BufferEncoding
    ): Buffer;
    /**
     * Returns the byte length of a string when encoded using `encoding`.
     * This is not the same as [`String.prototype.length`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/length), which does not account
     * for the encoding that is used to convert the string into bytes.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const str = '\u00bd + \u00bc = \u00be';
     *
     * console.log(`${str}: ${str.length} characters, ` +
     *             `${Buffer.byteLength(str, 'utf8')} bytes`);
     * // Prints: ½ + ¼ = ¾: 9 characters, 12 bytes
     * ```
     *
     * When `string` is a
     * `Buffer`/[`DataView`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/DataView)/[`TypedArray`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/-
     * Reference/Global_Objects/TypedArray)/[`ArrayBuffer`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer)/[`SharedArrayBuffer`](https://develop-
     * er.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer), the byte length as reported by `.byteLength`is returned.
     * @param string A value to calculate the length of.
     * @param [encoding='utf8'] If `string` is a string, this is its encoding.
     * @return The number of bytes contained within `string`.
     */
    byteLength(
      string:
        | string
        | Buffer
        | QuickJS.ArrayBufferView
        | ArrayBuffer
        | SharedArrayBuffer,
      encoding?: BufferEncoding
    ): number;
    /**
     * Returns a new `Buffer` which is the result of concatenating all the `Buffer` instances in the `list` together.
     *
     * If the list has no items, or if the `totalLength` is 0, then a new zero-length `Buffer` is returned.
     *
     * If `totalLength` is not provided, it is calculated from the `Buffer` instances
     * in `list` by adding their lengths.
     *
     * If `totalLength` is provided, it is coerced to an unsigned integer. If the
     * combined length of the `Buffer`s in `list` exceeds `totalLength`, the result is
     * truncated to `totalLength`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * // Create a single `Buffer` from a list of three `Buffer` instances.
     *
     * const buf1 = Buffer.alloc(10);
     * const buf2 = Buffer.alloc(14);
     * const buf3 = Buffer.alloc(18);
     * const totalLength = buf1.length + buf2.length + buf3.length;
     *
     * console.log(totalLength);
     * // Prints: 42
     *
     * const bufA = Buffer.concat([buf1, buf2, buf3], totalLength);
     *
     * console.log(bufA);
     * // Prints: <Buffer 00 00 00 00 ...>
     * console.log(bufA.length);
     * // Prints: 42
     * ```
     *
     * @param list List of `Buffer` or {@link Uint8Array} instances to concatenate.
     * @param totalLength Total length of the `Buffer` instances in `list` when concatenated.
     */
    concat(list: readonly Uint8Array[], totalLength?: number): Buffer;
    /**
     * Allocates a new `Buffer` using an `array` of bytes in the range `0` – `255`.
     */
    from(
      arrayBuffer: WithImplicitCoercion<ArrayBuffer | SharedArrayBuffer>,
      byteOffset?: number,
      length?: number
    ): Buffer;
    /**
     * Creates a new Buffer using the passed {data}
     * @param data data to create a new Buffer
     */
    from(data: Uint8Array | readonly number[]): Buffer;
    from(
      data: WithImplicitCoercion<Uint8Array | readonly number[] | string>
    ): Buffer;
    /**
     * Creates a new Buffer containing the given JavaScript string {str}.
     * If provided, the {encoding} parameter identifies the character encoding.
     * If not provided, {encoding} defaults to 'utf8'.
     */
    from(
      str:
        | WithImplicitCoercion<string>
        | {
            [Symbol.toPrimitive](hint: "string"): string;
          },
      encoding?: BufferEncoding
    ): Buffer;
  }
  interface Buffer extends Uint8Array {
    /**
     * Returns a new `Buffer` that references the same memory as the original, but
     * offset and cropped by the `start` and `end` indices.
     *
     * Specifying `end` greater than `buf.length` will return the same result as
     * that of `end` equal to `buf.length`.
     *
     * This method is inherited from [`TypedArray.prototype.subarray()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray/subarray).
     *
     * Modifying the new `Buffer` slice will modify the memory in the original `Buffer`because the allocated memory of the two objects overlap.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * // Create a `Buffer` with the ASCII alphabet, take a slice, and modify one byte
     * // from the original `Buffer`.
     *
     * const buf1 = Buffer.alloc(26);
     *
     * for (let i = 0; i < 26; i++) {
     *   // 97 is the decimal ASCII value for 'a'.
     *   buf1[i] = i + 97;
     * }
     *
     * const buf2 = buf1.subarray(0, 3);
     *
     * console.log(buf2.toString('ascii', 0, buf2.length));
     * // Prints: abc
     *
     * buf1[0] = 33;
     *
     * console.log(buf2.toString('ascii', 0, buf2.length));
     * // Prints: !bc
     * ```
     *
     * Specifying negative indexes causes the slice to be generated relative to the
     * end of `buf` rather than the beginning.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from('buffer');
     *
     * console.log(buf.subarray(-6, -1).toString());
     * // Prints: buffe
     * // (Equivalent to buf.subarray(0, 5).)
     *
     * console.log(buf.subarray(-6, -2).toString());
     * // Prints: buff
     * // (Equivalent to buf.subarray(0, 4).)
     *
     * console.log(buf.subarray(-5, -2).toString());
     * // Prints: uff
     * // (Equivalent to buf.subarray(1, 4).)
     * ```
     * @param [start=0] Where the new `Buffer` will start.
     * @param [end=buf.length] Where the new `Buffer` will end (not inclusive).
     */
    subarray(start?: number, end?: number): Buffer;
    /**
     * Decodes `buf` to a string according to the specified character encoding in`encoding`.
     *
     * If `encoding` is `'utf8'` and a byte sequence in the input is not valid UTF-8,
     * then each invalid byte is replaced with the replacement character `U+FFFD`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf1 = Buffer.alloc(26);
     *
     * for (let i = 0; i < 26; i++) {
     *   // 97 is the decimal ASCII value for 'a'.
     *   buf1[i] = i + 97;
     * }
     *
     * console.log(buf1.toString('utf8'));
     * // Prints: abcdefghijklmnopqrstuvwxyz
     *
     * const buf2 = Buffer.from('tést');
     *
     * console.log(buf2.toString('hex'));
     * // Prints: 74c3a97374
     * ```
     * @param [encoding='utf8'] The character encoding to use.
     */
    toString(encoding?: BufferEncoding): string;
  }
  var Buffer: BufferConstructor;
  /**
   * Decodes a string of Base64-encoded data into bytes, and encodes those bytes
   * into a string using UTF-8.
   *
   * The `data` may be any JavaScript-value that can be coerced into a string.
   *
   * @legacy Use `Buffer.from(data, 'base64')` instead.
   * @param data The Base64-encoded input string.
   */
  function atob(data: string): string;
  /**
   * Decodes a string into bytes using UTF-8, and encodes those bytes
   * into a string using Base64.
   *
   * The `data` may be any JavaScript-value that can be coerced into a string.
   *
   * @legacy Use `buf.toString('base64')` instead.
   * @param data An ASCII (Latin1) string.
   */
  function btoa(data: string): string;

  export { Buffer, atob, btoa };
}
