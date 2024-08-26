/**
 * The `crypto` module provides cryptographic functionality that includes a
 * set of wrappers for OpenSSL's hash, HMAC.
 *
 * ```js
 * import { createHmac } from 'crypto';
 *
 * const secret = 'abcdefg';
 * const hash = createHmac('sha256', secret)
 *                .update('I love cupcakes')
 *                .digest('hex');
 * console.log(hash);
 * // Prints:
 * //   c0fa1bc00531bd78ef38c628449c5102aeabd49b5dc3a2a516ea6ea959d6658e
 * ```
 */
declare module "crypto" {
  import { Buffer } from "buffer";
  type BinaryLike = string | QuickJS.ArrayBufferView;
  type BinaryToTextEncoding = "base64" | "hex";
  type CharacterEncoding = "utf8" | "utf-8" | "utf16le" | "utf-16le" | "latin1";
  type LegacyCharacterEncoding = "ascii";
  type Encoding =
    | BinaryToTextEncoding
    | CharacterEncoding
    | LegacyCharacterEncoding;
  /**
   * Creates and returns a `Hash` object that can be used to generate hash digests
   * using the given `algorithm`.
   *
   * The `algorithm` is supported by `'sha1'`, `'sha256'`,`'sha384'` and `'sha512'`.
   */
  function createHash(algorithm: string): Hash;
  /**
   * Creates and returns an `Hmac` object that uses the given `algorithm` and `key`.
   *
   * The `algorithm` is supported by `'sha1'`, `'sha256'`,`'sha384'` and `'sha512'`.
   *
   * The `key` is the HMAC key used to generate the cryptographic HMAC hash.
   * If it is a string, please consider `caveats when using strings as inputs to cryptographic APIs`.
   * If it was obtained from a cryptographically secure source of entropy, such as {@link randomBytes}
   * or {@link generateKey}, its length should not exceed the block size of `algorithm`
   * (e.g., 512 bits for SHA-256).
   */
  function createHmac(algorithm: string, key: BinaryLike): Hmac;
  /**
   * The `Hash` class is a utility for creating hash digests of data.
   *
   * Using the `hash.update()` and `hash.digest()` methods to produce the
   * computed hash.
   *
   * The {@link createHash} method is used to create `Hash` instances.
   * `Hash`objects are not to be created directly using the `new` keyword.
   *
   * Example: Using the `hash.update()` and `hash.digest()` methods:
   *
   * ```js
   * import { createHash } from 'crypto';
   *
   * const hash = createHash('sha256');
   *
   * hash.update('some data to hash');
   * console.log(hash.digest('hex'));
   * // Prints:
   * //   6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50
   * ```
   */
  class Hash {
    private constructor();
    /**
     * Updates the hash content with the given `data`, the encoding of which
     * is given in `inputEncoding`.
     * If `encoding` is not provided, and the `data` is a string, an
     * encoding of `'utf8'` is enforced. If `data` is a `Buffer`, `TypedArray`, or`DataView`,
     * then `inputEncoding` is ignored.
     *
     * This can be called many times with new data as it is streamed.
     * @param inputEncoding The `encoding` of the `data` string.
     */
    update(data: BinaryLike): Hash;
    update(data: string, inputEncoding: Encoding): Hash;
    /**
     * Calculates the digest of all of the data passed to be hashed (using the `hash.update()` method).
     * If `encoding` is provided a string will be returned; otherwise
     * a `Buffer` is returned.
     *
     * The `Hash` object can not be used again after `hash.digest()` method has been
     * called. Multiple calls will cause an error to be thrown.
     * @param encoding The `encoding` of the return value.
     */
    digest(): Buffer;
    digest(encoding: BinaryToTextEncoding): string;
  }
  /**
   * The `Hmac` class is a utility for creating cryptographic HMAC digests.
   *
   * Using the `hmac.update()` and `hmac.digest()` methods to produce the
   * computed HMAC digest.
   *
   * The {@link createHmac} method is used to create `Hmac` instances.
   * `Hmac`objects are not to be created directly using the `new` keyword.
   *
   * Example: Using the `hmac.update()` and `hmac.digest()` methods:
   *
   * ```js
   * import { createHmac } from 'crypto';
   *
   * const hmac = createHmac('sha256', 'a secret');
   *
   * hmac.update('some data to hash');
   * console.log(hmac.digest('hex'));
   * // Prints:
   * //   7fd04df92f636fd450bc841c9418e5825c17f33ad9c87c518115a45971f7f77e
   * ```
   */
  class Hmac {
    private constructor();
    /**
     * Updates the `Hmac` content with the given `data`, the encoding of which
     * is given in `inputEncoding`.
     * If `encoding` is not provided, and the `data` is a string, an
     * encoding of `'utf8'` is enforced. If `data` is a `Buffer`, `TypedArray`, or`DataView`,
     * then `inputEncoding` is ignored.
     *
     * This can be called many times with new data as it is streamed.
     * @param inputEncoding The `encoding` of the `data` string.
     */
    update(data: BinaryLike): Hmac;
    update(data: string, inputEncoding: Encoding): Hmac;
    /**
     * Calculates the HMAC digest of all of the data passed using `hmac.update()`.
     * If `encoding` is
     * provided a string is returned; otherwise a `Buffer` is returned;
     *
     * The `Hmac` object can not be used again after `hmac.digest()` has been
     * called. Multiple calls to `hmac.digest()` will result in an error being thrown.
     * @param encoding The `encoding` of the return value.
     */
    digest(): Buffer;
    digest(encoding: BinaryToTextEncoding): string;
  }
  /**
   * Generates cryptographically strong pseudorandom data. The `size` argument
   * is a number indicating the number of bytes to generate.
   *
   * the random bytes are generated synchronously and returned as a `Buffer`.
   * An error will be thrown if there is a problem generating the bytes.
   *
   * ```js
   * // Synchronous
   * import { randomBytes } from 'crypto';
   *
   * const buf = randomBytes(256);
   * console.log(
   *   `${buf.length} bytes of random data: ${buf.toString('hex')}`);
   * ```
   *
   * The `crypto.randomBytes()` method will not complete until there is
   * sufficient entropy available.
   * This should normally never take longer than a few milliseconds. The only time
   * when generating the random bytes may conceivably block for a longer period of
   * time is right after boot, when the whole system is still low on entropy.
   *
   * @param size The number of bytes to generate. The `size` must not be larger than `2**31 - 1`.
   */
  function randomBytes(size: number): Buffer;
  /**
   * Return a random integer `n` such that `min <= n < max`.  This
   * implementation avoids [modulo bias](https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle#Modulo_bias).
   *
   * The range (`max - min`) must be less than 2**48. `min` and `max` must
   * be [safe integers](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/isSafeInteger).
   *
   * ```js
   * // Synchronous
   * import { randomInt } from 'crypto';
   *
   * const n = randomInt(3);
   * console.log(`Random number chosen from (0, 1, 2): ${n}`);
   * ```
   *
   * ```js
   * // With `min` argument
   * import { randomInt } from 'crypto';
   *
   * const n = randomInt(1, 7);
   * console.log(`The dice rolled: ${n}`);
   * ```
   * @param [min=0] Start of random range (inclusive).
   * @param max End of random range (exclusive).
   */
  function randomInt(max: number): number;
  function randomInt(min: number, max: number): number;
  /**
   * Synchronous version of {@link randomFill}.
   *
   * ```js
   * import { Buffer } from 'buffer';
   * import { randomFillSync } from 'crypto';
   *
   * const buf = Buffer.alloc(10);
   * console.log(randomFillSync(buf).toString('hex'));
   *
   * randomFillSync(buf, 5);
   * console.log(buf.toString('hex'));
   *
   * // The above is equivalent to the following:
   * randomFillSync(buf, 5, 5);
   * console.log(buf.toString('hex'));
   * ```
   *
   * Any `ArrayBuffer`, `TypedArray` or `DataView` instance may be passed as`buffer`.
   *
   * ```js
   * import { Buffer } from 'buffer';
   * import { randomFillSync } from 'crypto';
   *
   * const a = new Uint32Array(10);
   * console.log(Buffer.from(randomFillSync(a).buffer,
   *                         a.byteOffset, a.byteLength).toString('hex'));
   *
   * const b = new DataView(new ArrayBuffer(10));
   * console.log(Buffer.from(randomFillSync(b).buffer,
   *                         b.byteOffset, b.byteLength).toString('hex'));
   *
   * const c = new ArrayBuffer(10);
   * console.log(Buffer.from(randomFillSync(c)).toString('hex'));
   * ```
   * @param buffer Must be supplied. The size of the provided `buffer` must not be larger than `2**31 - 1`.
   * @param [offset=0]
   * @param [size=buffer.length - offset]
   * @return The object passed as `buffer` argument.
   */
  function randomFillSync<T extends QuickJS.ArrayBufferView>(
    buffer: T,
    offset?: number,
    size?: number
  ): T;
  /**
   * This function is similar to {@link randomBytes} but requires the first
   * argument to be a `Buffer` that will be filled. It also
   * requires that a callback is passed in.
   *
   * If the `callback` function is not provided, an error will be thrown.
   *
   * ```js
   * import { Buffer } from 'buffer';
   * import { randomFill } from 'crypto';
   *
   * const buf = Buffer.alloc(10);
   * randomFill(buf, (err, buf) => {
   *   if (err) throw err;
   *   console.log(buf.toString('hex'));
   * });
   *
   * randomFill(buf, 5, (err, buf) => {
   *   if (err) throw err;
   *   console.log(buf.toString('hex'));
   * });
   *
   * // The above is equivalent to the following:
   * randomFill(buf, 5, 5, (err, buf) => {
   *   if (err) throw err;
   *   console.log(buf.toString('hex'));
   * });
   * ```
   *
   * Any `ArrayBuffer`, `TypedArray`, or `DataView` instance may be passed as `buffer`.
   *
   * While this includes instances of `Float32Array` and `Float64Array`, this
   * function should not be used to generate random floating-point numbers. The
   * result may contain `+Infinity`, `-Infinity`, and `NaN`, and even if the array
   * contains finite numbers only, they are not drawn from a uniform random
   * distribution and have no meaningful lower or upper bounds.
   *
   * ```js
   * import { Buffer } from 'buffer';
   * import { randomFill } from 'crypto';
   *
   * const a = new Uint32Array(10);
   * randomFill(a, (err, buf) => {
   *   if (err) throw err;
   *   console.log(Buffer.from(buf.buffer, buf.byteOffset, buf.byteLength)
   *     .toString('hex'));
   * });
   *
   * const b = new DataView(new ArrayBuffer(10));
   * randomFill(b, (err, buf) => {
   *   if (err) throw err;
   *   console.log(Buffer.from(buf.buffer, buf.byteOffset, buf.byteLength)
   *     .toString('hex'));
   * });
   *
   * const c = new ArrayBuffer(10);
   * randomFill(c, (err, buf) => {
   *   if (err) throw err;
   *   console.log(Buffer.from(buf).toString('hex'));
   * });
   * ```
   * @param buffer Must be supplied. The size of the provided `buffer` must not be larger than `2**31 - 1`.
   * @param [offset=0]
   * @param [size=buffer.length - offset]
   * @param callback `function(err, buf) {}`.
   */
  function randomFill<T extends QuickJS.ArrayBufferView>(
    buffer: T,
    callback: (err: Error | null, buf: T) => void
  ): void;
  function randomFill<T extends QuickJS.ArrayBufferView>(
    buffer: T,
    offset: number,
    callback: (err: Error | null, buf: T) => void
  ): void;
  function randomFill<T extends QuickJS.ArrayBufferView>(
    buffer: T,
    offset: number,
    size: number,
    callback: (err: Error | null, buf: T) => void
  ): void;
  type UUID = `${string}-${string}-${string}-${string}-${string}`;
  /**
   * A convenient alias for {@link webcrypto.getRandomValues}. This
   * implementation is not compliant with the Web Crypto spec, to write
   * web-compatible code use {@link webcrypto.getRandomValues} instead.
   * @return Returns `typedArray`.
   */
  function getRandomValues<T extends QuickJS.ArrayBufferView>(typedArray: T): T;
  /**
   * Generates a random {@link https://www.rfc-editor.org/rfc/rfc4122.txt RFC 4122} version 4 UUID.
   * The UUID is generated using a cryptographic pseudorandom number generator.
   */
  function randomUUID(): UUID;
}
