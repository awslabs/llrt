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
  export const constants: {
    MAX_LENGTH: number;
    MAX_STRING_LENGTH: number;
  };
  export type BufferEncoding =
    | "hex"
    | "base64"
    | "utf-8"
    | "utf8"
    | "unicode-1-1-utf8"
    | "ucs2"
    | "ucs-2"
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
    /**
     * Returns `true` if `obj` is a `Buffer`, `false` otherwise.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * Buffer.isBuffer(Buffer.alloc(10)); // true
     * Buffer.isBuffer(Buffer.from('foo')); // true
     * Buffer.isBuffer('a string'); // false
     * Buffer.isBuffer([]); // false
     * Buffer.isBuffer(new Uint8Array(1024)); // false
     * ```
     */
    isBuffer(obj: any): obj is Buffer;
    /**
     * Returns `true` if `encoding` is the name of a supported character encoding,
     * or `false` otherwise.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * console.log(Buffer.isEncoding('utf8'));
     * // Prints: true
     *
     * console.log(Buffer.isEncoding('hex'));
     * // Prints: true
     *
     * console.log(Buffer.isEncoding('utf/8'));
     * // Prints: false
     *
     * console.log(Buffer.isEncoding(''));
     * // Prints: false
     * ```
     * @param encoding A character encoding name to check.
     */
    isEncoding(encoding: string): encoding is BufferEncoding;
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
     * Allocates a new `Buffer` of `size` bytes.
     *
     * The underlying memory for `Buffer` instances created in this way is _not_
     * _initialized_. The contents of the newly created `Buffer` are unknown and _may contain sensitive data_. Use `Buffer.alloc()` instead to initialize`Buffer` instances with zeroes.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(10);
     *
     * console.log(buf);
     * // Prints (contents may vary): <Buffer a0 8b 28 3f 01 00 00 00 50 32>
     *
     * buf.fill(0);
     *
     * console.log(buf);
     * // Prints: <Buffer 00 00 00 00 00 00 00 00 00 00>
     * ```
     *
     * A `TypeError` will be thrown if `size` is not a number.
     *
     * @param size The desired length of the new `Buffer`.
     */
    allocUnsafe(size: number): Buffer;
    /**
     * Allocates a new `Buffer` of `size` bytes.
     *
     * The underlying memory for `Buffer` instances created in this way is _not_
     * _initialized_. The contents of the newly created `Buffer` are unknown and _may contain sensitive data_. Use `buf.fill(0)` to initialize
     * such `Buffer` instances with zeroes.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * // Need to keep around a few small chunks of memory.
     * const store = [];
     *
     * socket.on('readable', () => {
     *   let data;
     *   while (null !== (data = readable.read())) {
     *     // Allocate for retained data.
     *     const sb = Buffer.allocUnsafeSlow(10);
     *
     *     // Copy the data into the new allocation.
     *     data.copy(sb, 0, 0, 10);
     *
     *     store.push(sb);
     *   }
     * });
     * ```
     *
     * A `TypeError` will be thrown if `size` is not a number.
     *
     * @param size The desired length of the new `Buffer`.
     */
    allocUnsafeSlow(size: number): Buffer;
  }
  interface Buffer extends Uint8Array {
    /**
     * Writes `string` to `buf` at `offset` according to the character encoding in`encoding`. The `length` parameter is the number of bytes to write. If `buf` did
     * not contain enough space to fit the entire string, only part of `string` will be
     * written. However, partially encoded characters will not be written.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.alloc(256);
     *
     * const len = buf.write('\u00bd + \u00bc = \u00be', 0);
     *
     * console.log(`${len} bytes: ${buf.toString('utf8', 0, len)}`);
     * // Prints: 12 bytes: ½ + ¼ = ¾
     *
     * const buffer = Buffer.alloc(10);
     *
     * const length = buffer.write('abcd', 8);
     *
     * console.log(`${length} bytes: ${buffer.toString('utf8', 8, 10)}`);
     * // Prints: 2 bytes : ab
     * ```
     * @param string String to write to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write `string`.
     * @param [length=buf.length - offset] Maximum number of bytes to write (written bytes will not exceed `buf.length - offset`).
     * @param [encoding='utf8'] The character encoding of `string`.
     * @return Number of bytes written.
     */
    write(string: string, encoding?: BufferEncoding): number;
    write(string: string, offset: number, encoding?: BufferEncoding): number;
    write(
      string: string,
      offset: number,
      length: number,
      encoding?: BufferEncoding
    ): number;
    /**
     * Decodes `buf` to a string according to the specified character encoding in`encoding`. `start` and `end` may be passed to decode only a subset of `buf`.
     *
     * If `encoding` is `'utf8'` and a byte sequence in the input is not valid UTF-8,
     * then each invalid byte is replaced with the replacement character `U+FFFD`.
     *
     * ```js
     * import { Buffer } from 'node:buffer';
     *
     * const buf1 = Buffer.allocUnsafe(26);
     *
     * for (let i = 0; i < 26; i++) {
     *   // 97 is the decimal ASCII value for 'a'.
     *   buf1[i] = i + 97;
     * }
     *
     * console.log(buf1.toString('utf8'));
     * // Prints: abcdefghijklmnopqrstuvwxyz
     * console.log(buf1.toString('utf8', 0, 5));
     * // Prints: abcde
     *
     * const buf2 = Buffer.from('tést');
     *
     * console.log(buf2.toString('hex'));
     * // Prints: 74c3a97374
     * console.log(buf2.toString('utf8', 0, 3));
     * // Prints: té
     * console.log(buf2.toString(undefined, 0, 3));
     * // Prints: té
     * ```
     * @param [encoding='utf8'] The character encoding to use.
     * @param [start=0] The byte offset to start decoding at.
     * @param [end=buf.length] The byte offset to stop decoding at (not inclusive).
     */
    toString(encoding?: BufferEncoding, start?: number, end?: number): string;
    /**
     * Copies data from a region of `buf` to a region in `target`, even if the `target`memory region overlaps with `buf`.
     *
     * [`TypedArray.prototype.set()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray/set) performs the same operation, and is available
     * for all TypedArrays, including `Buffer`s, although it takes
     * different function arguments.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * // Create two `Buffer` instances.
     * const buf1 = Buffer.allocUnsafe(26);
     * const buf2 = Buffer.allocUnsafe(26).fill('!');
     *
     * for (let i = 0; i < 26; i++) {
     *   // 97 is the decimal ASCII value for 'a'.
     *   buf1[i] = i + 97;
     * }
     *
     * // Copy `buf1` bytes 16 through 19 into `buf2` starting at byte 8 of `buf2`.
     * buf1.copy(buf2, 8, 16, 20);
     * // This is equivalent to:
     * // buf2.set(buf1.subarray(16, 20), 8);
     *
     * console.log(buf2.toString('ascii', 0, 25));
     * // Prints: !!!!!!!!qrst!!!!!!!!!!!!!
     * ```
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * // Create a `Buffer` and copy data from one region to an overlapping region
     * // within the same `Buffer`.
     *
     * const buf = Buffer.allocUnsafe(26);
     *
     * for (let i = 0; i < 26; i++) {
     *   // 97 is the decimal ASCII value for 'a'.
     *   buf[i] = i + 97;
     * }
     *
     * buf.copy(buf, 0, 4, 10);
     *
     * console.log(buf.toString());
     * // Prints: efghijghijklmnopqrstuvwxyz
     * ```
     * @param target A `Buffer` or {@link Uint8Array} to copy into.
     * @param [targetStart=0] The offset within `target` at which to begin writing.
     * @param [sourceStart=0] The offset within `buf` from which to begin copying.
     * @param [sourceEnd=buf.length] The offset within `buf` at which to stop copying (not inclusive).
     * @return The number of bytes copied.
     */
    copy(
      target: Uint8Array,
      targetStart?: number,
      sourceStart?: number,
      sourceEnd?: number
    ): number;
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
     * Writes `value` to `buf` at the specified `offset` as big-endian.
     *
     * `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(8);
     *
     * buf.writeBigInt64BE(0x0102030405060708n, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 01 02 03 04 05 06 07 08>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy: `0 <= offset <= buf.length - 8`.
     * @return `offset` plus the number of bytes written.
     */
    writeBigInt64BE(value: bigint, offset?: number): number;
    /**
     * @alias Buffer.writeBigUInt64BE
     */
    writeBigUint64BE(value: bigint, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian.
     *
     * `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(8);
     *
     * buf.writeBigInt64LE(0x0102030405060708n, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 08 07 06 05 04 03 02 01>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy: `0 <= offset <= buf.length - 8`.
     * @return `offset` plus the number of bytes written.
     */
    writeBigInt64LE(value: bigint, offset?: number): number;
    /**
     * @alias Buffer.writeBigUInt64LE
     */
    writeBigUint64LE(value: bigint, offset?: number): number;
    /**
     * Reads an unsigned, big-endian 64-bit integer from `buf` at the specified`offset`.
     *
     * This function is also available under the `readBigUint64BE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff]);
     *
     * console.log(buf.readBigUInt64BE(0));
     * // Prints: 4294967295n
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy: `0 <= offset <= buf.length - 8`.
     */
    readBigUInt64BE(offset?: number): bigint;
    /**
     * @alias Buffer.readBigUInt64BE
     */
    readBigUint64BE(offset?: number): bigint;
    /**
     * Reads an unsigned, little-endian 64-bit integer from `buf` at the specified`offset`.
     *
     * This function is also available under the `readBigUint64LE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff]);
     *
     * console.log(buf.readBigUInt64LE(0));
     * // Prints: 18446744069414584320n
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy: `0 <= offset <= buf.length - 8`.
     */
    readBigUInt64LE(offset?: number): bigint;
    /**
     * @alias Buffer.readBigUInt64LE
     */
    readBigUint64LE(offset?: number): bigint;
    /**
     * Reads a signed, big-endian 64-bit integer from `buf` at the specified `offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed
     * values.
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy: `0 <= offset <= buf.length - 8`.
     */
    readBigInt64BE(offset?: number): bigint;
    /**
     * Reads a signed, little-endian 64-bit integer from `buf` at the specified`offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed
     * values.
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy: `0 <= offset <= buf.length - 8`.
     */
    readBigInt64LE(offset?: number): bigint;
    /**
     * Reads an unsigned 8-bit integer from `buf` at the specified `offset`.
     *
     * This function is also available under the `readUint8` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([1, -2]);
     *
     * console.log(buf.readUInt8(0));
     * // Prints: 1
     * console.log(buf.readUInt8(1));
     * // Prints: 254
     * console.log(buf.readUInt8(2));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 1`.
     */
    readUInt8(offset?: number): number;
    /**
     * @alias Buffer.readUInt8
     */
    readUint8(offset?: number): number;
    /**
     * Reads an unsigned, little-endian 16-bit integer from `buf` at the specified `offset`.
     *
     * This function is also available under the `readUint16LE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0x12, 0x34, 0x56]);
     *
     * console.log(buf.readUInt16LE(0).toString(16));
     * // Prints: 3412
     * console.log(buf.readUInt16LE(1).toString(16));
     * // Prints: 5634
     * console.log(buf.readUInt16LE(2).toString(16));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 2`.
     */
    readUInt16LE(offset?: number): number;
    /**
     * @alias Buffer.readUInt16LE
     */
    readUint16LE(offset?: number): number;
    /**
     * Reads an unsigned, big-endian 16-bit integer from `buf` at the specified`offset`.
     *
     * This function is also available under the `readUint16BE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0x12, 0x34, 0x56]);
     *
     * console.log(buf.readUInt16BE(0).toString(16));
     * // Prints: 1234
     * console.log(buf.readUInt16BE(1).toString(16));
     * // Prints: 3456
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 2`.
     */
    readUInt16BE(offset?: number): number;
    /**
     * @alias Buffer.readUInt16BE
     */
    readUint16BE(offset?: number): number;
    /**
     * Reads an unsigned, little-endian 32-bit integer from `buf` at the specified`offset`.
     *
     * This function is also available under the `readUint32LE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0x12, 0x34, 0x56, 0x78]);
     *
     * console.log(buf.readUInt32LE(0).toString(16));
     * // Prints: 78563412
     * console.log(buf.readUInt32LE(1).toString(16));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 4`.
     */
    readUInt32LE(offset?: number): number;
    /**
     * @alias Buffer.readUInt32LE
     */
    readUint32LE(offset?: number): number;
    /**
     * Reads a signed 8-bit integer from `buf` at the specified `offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed values.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([-1, 5]);
     *
     * console.log(buf.readInt8(0));
     * // Prints: -1
     * console.log(buf.readInt8(1));
     * // Prints: 5
     * console.log(buf.readInt8(2));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 1`.
     */
    readInt8(offset?: number): number;
    /**
     * Reads a signed, little-endian 16-bit integer from `buf` at the specified`offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed values.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0, 5]);
     *
     * console.log(buf.readInt16LE(0));
     * // Prints: 1280
     * console.log(buf.readInt16LE(1));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 2`.
     */
    readInt16LE(offset?: number): number;
    /**
     * Reads a signed, big-endian 16-bit integer from `buf` at the specified `offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed values.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0, 5]);
     *
     * console.log(buf.readInt16BE(0));
     * // Prints: 5
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 2`.
     */
    readInt16BE(offset?: number): number;
    /**
     * Reads a signed, little-endian 32-bit integer from `buf` at the specified`offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed values.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0, 0, 0, 5]);
     *
     * console.log(buf.readInt32LE(0));
     * // Prints: 83886080
     * console.log(buf.readInt32LE(1));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 4`.
     */
    readInt32LE(offset?: number): number;
    /**
     * Reads a signed, big-endian 32-bit integer from `buf` at the specified `offset`.
     *
     * Integers read from a `Buffer` are interpreted as two's complement signed values.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([0, 0, 0, 5]);
     *
     * console.log(buf.readInt32BE(0));
     * // Prints: 5
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 4`.
     */
    readInt32BE(offset?: number): number;
    /**
     * Reads a 32-bit, little-endian float from `buf` at the specified `offset`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([1, 2, 3, 4]);
     *
     * console.log(buf.readFloatLE(0));
     * // Prints: 1.539989614439558e-36
     * console.log(buf.readFloatLE(1));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 4`.
     */
    readFloatLE(offset?: number): number;
    /**
     * Reads a 32-bit, big-endian float from `buf` at the specified `offset`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([1, 2, 3, 4]);
     *
     * console.log(buf.readFloatBE(0));
     * // Prints: 2.387939260590663e-38
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 4`.
     */
    readFloatBE(offset?: number): number;
    /**
     * Reads a 64-bit, little-endian double from `buf` at the specified `offset`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]);
     *
     * console.log(buf.readDoubleLE(0));
     * // Prints: 5.447603722011605e-270
     * console.log(buf.readDoubleLE(1));
     * // Throws ERR_OUT_OF_RANGE.
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 8`.
     */
    readDoubleLE(offset?: number): number;
    /**
     * Reads a 64-bit, big-endian double from `buf` at the specified `offset`.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]);
     *
     * console.log(buf.readDoubleBE(0));
     * // Prints: 8.20788039913184e-304
     * ```
     * @param [offset=0] Number of bytes to skip before starting to read. Must satisfy `0 <= offset <= buf.length - 8`.
     */
    readDoubleBE(offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset`. `value` must be a
     * valid unsigned 8-bit integer. Behavior is undefined when `value` is anything
     * other than an unsigned 8-bit integer.
     *
     * This function is also available under the `writeUint8` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeUInt8(0x3, 0);
     * buf.writeUInt8(0x4, 1);
     * buf.writeUInt8(0x23, 2);
     * buf.writeUInt8(0x42, 3);
     *
     * console.log(buf);
     * // Prints: <Buffer 03 04 23 42>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 1`.
     * @return `offset` plus the number of bytes written.
     */
    writeUInt8(value: number, offset?: number): number;
    /**
     * @alias Buffer.writeUInt8
     */
    writeUint8(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian. The `value` must be a valid unsigned 16-bit integer. Behavior is undefined when `value` is
     * anything other than an unsigned 16-bit integer.
     *
     * This function is also available under the `writeUint16LE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeUInt16LE(0xdead, 0);
     * buf.writeUInt16LE(0xbeef, 2);
     *
     * console.log(buf);
     * // Prints: <Buffer ad de ef be>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 2`.
     * @return `offset` plus the number of bytes written.
     */
    writeUInt16LE(value: number, offset?: number): number;
    /**
     * @alias Buffer.writeUInt16LE
     */
    writeUint16LE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian. The `value` must be a valid unsigned 16-bit integer. Behavior is undefined when `value`is anything other than an
     * unsigned 16-bit integer.
     *
     * This function is also available under the `writeUint16BE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeUInt16BE(0xdead, 0);
     * buf.writeUInt16BE(0xbeef, 2);
     *
     * console.log(buf);
     * // Prints: <Buffer de ad be ef>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 2`.
     * @return `offset` plus the number of bytes written.
     */
    writeUInt16BE(value: number, offset?: number): number;
    /**
     * @alias Buffer.writeUInt16BE
     */
    writeUint16BE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian. The `value` must be a valid unsigned 32-bit integer. Behavior is undefined when `value` is
     * anything other than an unsigned 32-bit integer.
     *
     * This function is also available under the `writeUint32LE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeUInt32LE(0xfeedface, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer ce fa ed fe>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeUInt32LE(value: number, offset?: number): number;
    /**
     * @alias Buffer.writeUInt32LE
     */
    writeUint32LE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian. The `value` must be a valid unsigned 32-bit integer. Behavior is undefined when `value`is anything other than an
     * unsigned 32-bit integer.
     *
     * This function is also available under the `writeUint32BE` alias.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeUInt32BE(0xfeedface, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer fe ed fa ce>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeUInt32BE(value: number, offset?: number): number;
    /**
     * @alias Buffer.writeUInt32BE
     */
    writeUint32BE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset`. `value` must be a valid
     * signed 8-bit integer. Behavior is undefined when `value` is anything other than
     * a signed 8-bit integer.
     *
     * `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(2);
     *
     * buf.writeInt8(2, 0);
     * buf.writeInt8(-2, 1);
     *
     * console.log(buf);
     * // Prints: <Buffer 02 fe>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 1`.
     * @return `offset` plus the number of bytes written.
     */
    writeInt8(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian.  The `value` must be a valid signed 16-bit integer. Behavior is undefined when `value` is
     * anything other than a signed 16-bit integer.
     *
     * The `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(2);
     *
     * buf.writeInt16LE(0x0304, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 04 03>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 2`.
     * @return `offset` plus the number of bytes written.
     */
    writeInt16LE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian.  The `value` must be a valid signed 16-bit integer. Behavior is undefined when `value` is
     * anything other than a signed 16-bit integer.
     *
     * The `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(2);
     *
     * buf.writeInt16BE(0x0102, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 01 02>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 2`.
     * @return `offset` plus the number of bytes written.
     */
    writeInt16BE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian. The `value` must be a valid signed 32-bit integer. Behavior is undefined when `value` is
     * anything other than a signed 32-bit integer.
     *
     * The `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeInt32LE(0x05060708, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 08 07 06 05>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeInt32LE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian. The `value` must be a valid signed 32-bit integer. Behavior is undefined when `value` is
     * anything other than a signed 32-bit integer.
     *
     * The `value` is interpreted and written as a two's complement signed integer.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeInt32BE(0x01020304, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 01 02 03 04>
     * ```
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeInt32BE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian. Behavior is
     * undefined when `value` is anything other than a JavaScript number.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeFloatLE(0xcafebabe, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer bb fe 4a 4f>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeFloatLE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian. Behavior is
     * undefined when `value` is anything other than a JavaScript number.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(4);
     *
     * buf.writeFloatBE(0xcafebabe, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 4f 4a fe bb>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 4`.
     * @return `offset` plus the number of bytes written.
     */
    writeFloatBE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as little-endian. The `value` must be a JavaScript number. Behavior is undefined when `value` is anything
     * other than a JavaScript number.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(8);
     *
     * buf.writeDoubleLE(123.456, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 77 be 9f 1a 2f dd 5e 40>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 8`.
     * @return `offset` plus the number of bytes written.
     */
    writeDoubleLE(value: number, offset?: number): number;
    /**
     * Writes `value` to `buf` at the specified `offset` as big-endian. The `value` must be a JavaScript number. Behavior is undefined when `value` is anything
     * other than a JavaScript number.
     *
     * ```js
     * import { Buffer } from 'buffer';
     *
     * const buf = Buffer.allocUnsafe(8);
     *
     * buf.writeDoubleBE(123.456, 0);
     *
     * console.log(buf);
     * // Prints: <Buffer 40 5e dd 2f 1a 9f be 77>
     * ```
     * @param value Number to be written to `buf`.
     * @param [offset=0] Number of bytes to skip before starting to write. Must satisfy `0 <= offset <= buf.length - 8`.
     * @return `offset` plus the number of bytes written.
     */
    writeDoubleBE(value: number, offset?: number): number;
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
