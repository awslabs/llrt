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

  /**
   * A convenient alias for `crypto.webcrypto.subtle`.
   * @since v17.4.0
   */
  const subtle: webcrypto.SubtleCrypto;
  /**
   * An implementation of the Web Crypto API standard.
   *
   * See the {@link https://nodejs.org/docs/latest/api/webcrypto.html Web Crypto API documentation} for details.
   */
  namespace webcrypto {
    type BufferSource = ArrayBufferView | ArrayBuffer;
    type KeyFormat = "jwk" | "pkcs8" | "raw" | "spki";
    type KeyType = "private" | "public" | "secret";
    type KeyUsage =
      | "decrypt"
      | "deriveBits"
      | "deriveKey"
      | "encrypt"
      | "sign"
      | "unwrapKey"
      | "verify"
      | "wrapKey";
    type AlgorithmIdentifier = Algorithm | string;
    type HashAlgorithmIdentifier = AlgorithmIdentifier;
    type NamedCurve = string;
    type BigInteger = Uint8Array;
    interface AesCbcParams extends Algorithm {
      iv: BufferSource;
    }
    interface AesCtrParams extends Algorithm {
      counter: BufferSource;
      length: number;
    }
    interface AesDerivedKeyParams extends Algorithm {
      length: number;
    }
    interface AesGcmParams extends Algorithm {
      additionalData?: BufferSource;
      iv: BufferSource;
      tagLength?: number;
    }
    interface AesKeyAlgorithm extends KeyAlgorithm {
      length: number;
    }
    interface AesKeyGenParams extends Algorithm {
      length: number;
    }
    interface Algorithm {
      name: string;
    }
    interface EcKeyAlgorithm extends KeyAlgorithm {
      namedCurve: NamedCurve;
    }
    interface EcKeyGenParams extends Algorithm {
      namedCurve: NamedCurve;
    }
    interface EcKeyImportParams extends Algorithm {
      namedCurve: NamedCurve;
    }
    interface EcdhKeyDeriveParams extends Algorithm {
      public: CryptoKey;
    }
    interface EcdsaParams extends Algorithm {
      hash: HashAlgorithmIdentifier;
    }
    interface Ed448Params extends Algorithm {
      context?: BufferSource;
    }
    interface HkdfParams extends Algorithm {
      hash: HashAlgorithmIdentifier;
      info: BufferSource;
      salt: BufferSource;
    }
    interface HmacImportParams extends Algorithm {
      hash: HashAlgorithmIdentifier;
      length?: number;
    }
    interface HmacKeyAlgorithm extends KeyAlgorithm {
      hash: KeyAlgorithm;
      length: number;
    }
    interface HmacKeyGenParams extends Algorithm {
      hash: HashAlgorithmIdentifier;
      length?: number;
    }
    interface JsonWebKey {
      alg?: string;
      crv?: string;
      d?: string;
      dp?: string;
      dq?: string;
      e?: string;
      ext?: boolean;
      k?: string;
      key_ops?: string[];
      kty?: string;
      n?: string;
      oth?: RsaOtherPrimesInfo[];
      p?: string;
      q?: string;
      qi?: string;
      use?: string;
      x?: string;
      y?: string;
    }
    interface KeyAlgorithm {
      name: string;
    }
    interface Pbkdf2Params extends Algorithm {
      hash: HashAlgorithmIdentifier;
      iterations: number;
      salt: BufferSource;
    }
    interface RsaHashedImportParams extends Algorithm {
      hash: HashAlgorithmIdentifier;
    }
    interface RsaHashedKeyAlgorithm extends RsaKeyAlgorithm {
      hash: KeyAlgorithm;
    }
    interface RsaHashedKeyGenParams extends RsaKeyGenParams {
      hash: HashAlgorithmIdentifier;
    }
    interface RsaKeyAlgorithm extends KeyAlgorithm {
      modulusLength: number;
      publicExponent: BigInteger;
    }
    interface RsaKeyGenParams extends Algorithm {
      modulusLength: number;
      publicExponent: BigInteger;
    }
    interface RsaOaepParams extends Algorithm {
      label?: BufferSource;
    }
    interface RsaOtherPrimesInfo {
      d?: string;
      r?: string;
      t?: string;
    }
    interface RsaPssParams extends Algorithm {
      saltLength: number;
    }

    interface CryptoKey {
      /**
       * An object detailing the algorithm for which the key can be used along with additional algorithm-specific parameters.
       */
      readonly algorithm: KeyAlgorithm;
      /**
       * When `true`, the {@link CryptoKey} can be extracted using either `subtleCrypto.exportKey()` or `subtleCrypto.wrapKey()`.
       */
      readonly extractable: boolean;
      /**
       * A string identifying whether the key is a symmetric (`'secret'`) or asymmetric (`'private'` or `'public'`) key.
       */
      readonly type: KeyType;
      /**
       * An array of strings identifying the operations for which the key may be used.
       *
       * The possible usages are:
       * - `'encrypt'` - The key may be used to encrypt data.
       * - `'decrypt'` - The key may be used to decrypt data.
       * - `'sign'` - The key may be used to generate digital signatures.
       * - `'verify'` - The key may be used to verify digital signatures.
       * - `'deriveKey'` - The key may be used to derive a new key.
       * - `'deriveBits'` - The key may be used to derive bits.
       * - `'wrapKey'` - The key may be used to wrap another key.
       * - `'unwrapKey'` - The key may be used to unwrap another key.
       *
       * Valid key usages depend on the key algorithm (identified by `cryptokey.algorithm.name`).
       * @since v15.0.0
       */
      readonly usages: KeyUsage[];
    }
    /**
     * The `CryptoKeyPair` is a simple dictionary object with `publicKey` and `privateKey` properties, representing an asymmetric key pair.
     */
    interface CryptoKeyPair {
      /**
       * A {@link CryptoKey} whose type will be `'private'`.
       */
      privateKey: CryptoKey;
      /**
       * A {@link CryptoKey} whose type will be `'public'`.
       */
      publicKey: CryptoKey;
    }

    interface SubtleCrypto {
      /**
       * Using the method and parameters specified in `algorithm` and the keying material provided by `key`,
       * `subtle.decrypt()` attempts to decipher the provided `data`. If successful,
       * the returned promise will be resolved with an `<ArrayBuffer>` containing the plaintext result.
       *
       * The algorithms currently supported include:
       *
       * - `'RSA-OAEP'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       */
      decrypt(
        algorithm:
          | AlgorithmIdentifier
          | RsaOaepParams
          | AesCtrParams
          | AesCbcParams
          | AesGcmParams,
        key: CryptoKey,
        data: BufferSource
      ): Promise<ArrayBuffer>;
      /**
       * Using the method and parameters specified in `algorithm` and the keying material provided by `baseKey`,
       * `subtle.deriveBits()` attempts to generate `length` bits.
       * The LLRT implementation requires that when `length` is a number it must be multiple of `8`.
       * When `length` is `null` the maximum number of bits for a given algorithm is generated. This is allowed
       * for the `'ECDH'`, `'X25519'`, and `'X448'` algorithms.
       * If successful, the returned promise will be resolved with an `<ArrayBuffer>` containing the generated data.
       *
       * The algorithms currently supported include:
       *
       * - `'ECDH'`
       * - `'X25519'`
       * - `'X448'`
       * - `'HKDF'`
       * - `'PBKDF2'`
       */
      deriveBits(
        algorithm: EcdhKeyDeriveParams,
        baseKey: CryptoKey,
        length: number | null
      ): Promise<ArrayBuffer>;
      deriveBits(
        algorithm: AlgorithmIdentifier | HkdfParams | Pbkdf2Params,
        baseKey: CryptoKey,
        length: number
      ): Promise<ArrayBuffer>;
      /**
       * Using the method and parameters specified in `algorithm`, and the keying material provided by `baseKey`,
       * `subtle.deriveKey()` attempts to generate a new <CryptoKey>` based on the method and parameters in `derivedKeyAlgorithm`.
       *
       * Calling `subtle.deriveKey()` is equivalent to calling `subtle.deriveBits()` to generate raw keying material,
       * then passing the result into the `subtle.importKey()` method using the `deriveKeyAlgorithm`, `extractable`, and `keyUsages` parameters as input.
       *
       * The algorithms currently supported include:
       *
       * - `'ECDH'`
       * - `'X25519'`
       * - `'X448'`
       * - `'HKDF'`
       * - `'PBKDF2'`
       * @param keyUsages See {@link https://nodejs.org/docs/latest/api/webcrypto.html#cryptokeyusages Key usages}.
       */
      deriveKey(
        algorithm:
          | AlgorithmIdentifier
          | EcdhKeyDeriveParams
          | HkdfParams
          | Pbkdf2Params,
        baseKey: CryptoKey,
        derivedKeyAlgorithm:
          | AlgorithmIdentifier
          | AesDerivedKeyParams
          | HmacImportParams
          | HkdfParams
          | Pbkdf2Params,
        extractable: boolean,
        keyUsages: readonly KeyUsage[]
      ): Promise<CryptoKey>;
      /**
       * Using the method identified by `algorithm`, `subtle.digest()` attempts to generate a digest of `data`.
       * If successful, the returned promise is resolved with an `<ArrayBuffer>` containing the computed digest.
       *
       * If `algorithm` is provided as a `<string>`, it must be one of:
       *
       * - `'SHA-1'`
       * - `'SHA-256'`
       * - `'SHA-384'`
       * - `'SHA-512'`
       *
       * If `algorithm` is provided as an `<Object>`, it must have a `name` property whose value is one of the above.
       */
      digest(
        algorithm: AlgorithmIdentifier,
        data: BufferSource
      ): Promise<ArrayBuffer>;
      /**
       * Using the method and parameters specified by `algorithm` and the keying material provided by `key`,
       * `subtle.encrypt()` attempts to encipher `data`. If successful,
       * the returned promise is resolved with an `<ArrayBuffer>` containing the encrypted result.
       *
       * The algorithms currently supported include:
       *
       * - `'RSA-OAEP'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       */
      encrypt(
        algorithm:
          | AlgorithmIdentifier
          | RsaOaepParams
          | AesCtrParams
          | AesCbcParams
          | AesGcmParams,
        key: CryptoKey,
        data: BufferSource
      ): Promise<ArrayBuffer>;
      /**
       * Exports the given key into the specified format, if supported.
       *
       * If the `<CryptoKey>` is not extractable, the returned promise will reject.
       *
       * When `format` is either `'pkcs8'` or `'spki'` and the export is successful,
       * the returned promise will be resolved with an `<ArrayBuffer>` containing the exported key data.
       *
       * When `format` is `'jwk'` and the export is successful, the returned promise will be resolved with a
       * JavaScript object conforming to the {@link https://tools.ietf.org/html/rfc7517 JSON Web Key} specification.
       * @param format Must be one of `'raw'`, `'pkcs8'`, `'spki'`, or `'jwk'`.
       * @returns `<Promise>` containing `<ArrayBuffer>`.
       */
      exportKey(format: "jwk", key: CryptoKey): Promise<JsonWebKey>;
      exportKey(
        format: Exclude<KeyFormat, "jwk">,
        key: CryptoKey
      ): Promise<ArrayBuffer>;
      /**
       * Using the method and parameters provided in `algorithm`,
       * `subtle.generateKey()` attempts to generate new keying material.
       * Depending the method used, the method may generate either a single `<CryptoKey>` or a `<CryptoKeyPair>`.
       *
       * The `<CryptoKeyPair>` (public and private key) generating algorithms supported include:
       *
       * - `'RSASSA-PKCS1-v1_5'`
       * - `'RSA-PSS'`
       * - `'RSA-OAEP'`
       * - `'ECDSA'`
       * - `'ECDH'`
       * The `<CryptoKey>` (secret key) generating algorithms supported include:
       *
       * - `'HMAC'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       * - `'AES-KW'`
       * @param keyUsages See {@link https://nodejs.org/docs/latest/api/webcrypto.html#cryptokeyusages Key usages}.
       */
      generateKey(
        algorithm: RsaHashedKeyGenParams | EcKeyGenParams,
        extractable: boolean,
        keyUsages: readonly KeyUsage[]
      ): Promise<CryptoKeyPair>;
      generateKey(
        algorithm: AesKeyGenParams | HmacKeyGenParams | Pbkdf2Params,
        extractable: boolean,
        keyUsages: readonly KeyUsage[]
      ): Promise<CryptoKey>;
      generateKey(
        algorithm: AlgorithmIdentifier,
        extractable: boolean,
        keyUsages: KeyUsage[]
      ): Promise<CryptoKeyPair | CryptoKey>;
      /**
       * The `subtle.importKey()` method attempts to interpret the provided `keyData` as the given `format`
       * to create a `<CryptoKey>` instance using the provided `algorithm`, `extractable`, and `keyUsages` arguments.
       * If the import is successful, the returned promise will be resolved with the created `<CryptoKey>`.
       *
       * If importing a `'PBKDF2'` key, `extractable` must be `false`.
       * @param format Must be one of `'raw'`, `'pkcs8'`, `'spki'`, or `'jwk'`.
       * @param keyUsages See {@link https://nodejs.org/docs/latest/api/webcrypto.html#cryptokeyusages Key usages}.
       */
      importKey(
        format: "jwk",
        keyData: JsonWebKey,
        algorithm:
          | AlgorithmIdentifier
          | RsaHashedImportParams
          | EcKeyImportParams
          | HmacImportParams
          | AesKeyAlgorithm,
        extractable: boolean,
        keyUsages: readonly KeyUsage[]
      ): Promise<CryptoKey>;
      importKey(
        format: Exclude<KeyFormat, "jwk">,
        keyData: BufferSource,
        algorithm:
          | AlgorithmIdentifier
          | RsaHashedImportParams
          | EcKeyImportParams
          | HmacImportParams
          | AesKeyAlgorithm,
        extractable: boolean,
        keyUsages: KeyUsage[]
      ): Promise<CryptoKey>;
      /**
       * Using the method and parameters given by `algorithm` and the keying material provided by `key`,
       * `subtle.sign()` attempts to generate a cryptographic signature of `data`. If successful,
       * the returned promise is resolved with an `<ArrayBuffer>` containing the generated signature.
       *
       * The algorithms currently supported include:
       *
       * - `'RSASSA-PKCS1-v1_5'`
       * - `'RSA-PSS'`
       * - `'ECDSA'`
       * - `'HMAC'`
       */
      sign(
        algorithm:
          | AlgorithmIdentifier
          | RsaPssParams
          | EcdsaParams
          | Ed448Params,
        key: CryptoKey,
        data: BufferSource
      ): Promise<ArrayBuffer>;
      /**
       * In cryptography, "wrapping a key" refers to exporting and then encrypting the keying material.
       * The `subtle.unwrapKey()` method attempts to decrypt a wrapped key and create a `<CryptoKey>` instance.
       * It is equivalent to calling `subtle.decrypt()` first on the encrypted key data (using the `wrappedKey`, `unwrapAlgo`, and `unwrappingKey` arguments as input)
       * then passing the results in to the `subtle.importKey()` method using the `unwrappedKeyAlgo`, `extractable`, and `keyUsages` arguments as inputs.
       * If successful, the returned promise is resolved with a `<CryptoKey>` object.
       *
       * The wrapping algorithms currently supported include:
       *
       * - `'RSA-OAEP'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       * - `'AES-KW'`
       *
       * The unwrapped key algorithms supported include:
       *
       * - `'RSASSA-PKCS1-v1_5'`
       * - `'RSA-PSS'`
       * - `'RSA-OAEP'`
       * - `'ECDSA'`
       * - `'ECDH'`
       * - `'HMAC'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       * - `'AES-KW'`
       * @param format Must be one of `'raw'`, `'pkcs8'`, `'spki'`, or `'jwk'`.
       * @param keyUsages See {@link https://nodejs.org/docs/latest/api/webcrypto.html#cryptokeyusages Key usages}.
       */
      unwrapKey(
        format: KeyFormat,
        wrappedKey: BufferSource,
        unwrappingKey: CryptoKey,
        unwrapAlgorithm:
          | AlgorithmIdentifier
          | RsaOaepParams
          | AesCtrParams
          | AesCbcParams
          | AesGcmParams,
        unwrappedKeyAlgorithm:
          | AlgorithmIdentifier
          | RsaHashedImportParams
          | EcKeyImportParams
          | HmacImportParams
          | AesKeyAlgorithm,
        extractable: boolean,
        keyUsages: KeyUsage[]
      ): Promise<CryptoKey>;
      /**
       * Using the method and parameters given in `algorithm` and the keying material provided by `key`,
       * `subtle.verify()` attempts to verify that `signature` is a valid cryptographic signature of `data`.
       * The returned promise is resolved with either `true` or `false`.
       *
       * The algorithms currently supported include:
       *
       * - `'RSASSA-PKCS1-v1_5'`
       * - `'RSA-PSS'`
       * - `'ECDSA'`
       * - `'HMAC'`
       */
      verify(
        algorithm:
          | AlgorithmIdentifier
          | RsaPssParams
          | EcdsaParams
          | Ed448Params,
        key: CryptoKey,
        signature: BufferSource,
        data: BufferSource
      ): Promise<boolean>;
      /**
       * In cryptography, "wrapping a key" refers to exporting and then encrypting the keying material.
       * The `subtle.wrapKey()` method exports the keying material into the format identified by `format`,
       * then encrypts it using the method and parameters specified by `wrapAlgo` and the keying material provided by `wrappingKey`.
       * It is the equivalent to calling `subtle.exportKey()` using `format` and `key` as the arguments,
       * then passing the result to the `subtle.encrypt()` method using `wrappingKey` and `wrapAlgo` as inputs.
       * If successful, the returned promise will be resolved with an `<ArrayBuffer>` containing the encrypted key data.
       *
       * The wrapping algorithms currently supported include:
       *
       * - `'RSA-OAEP'`
       * - `'AES-CTR'`
       * - `'AES-CBC'`
       * - `'AES-GCM'`
       * - `'AES-KW'`
       * @param format Must be one of `'raw'`, `'pkcs8'`, `'spki'`, or `'jwk'`.
       */
      wrapKey(
        format: KeyFormat,
        key: CryptoKey,
        wrappingKey: CryptoKey,
        wrapAlgorithm:
          | AlgorithmIdentifier
          | RsaOaepParams
          | AesCtrParams
          | AesCbcParams
          | AesGcmParams
      ): Promise<ArrayBuffer>;
    }
  }
}
