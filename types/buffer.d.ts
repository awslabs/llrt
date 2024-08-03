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
     * If `fill` is specified, the allocated `Buffer` will be initialized by calling `buf.fill(fill)`.
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
     * initialized by calling `buf.fill(fill, encoding)`.
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
  }
  interface Buffer extends Uint8Array {}
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
