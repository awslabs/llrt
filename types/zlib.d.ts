/**
 * The `zlib` module provides compression functionality implemented using
 * Gzip, Deflate/Inflate, Brotli and Zstandard.
 *
 * To access it:
 *
 * ```js
 * import * as zlib from 'zlib';
 * ```
 *
 * It is possible to compress or decompress data in a single step:
 *
 * ```js
 * mport * as zlib from 'zlib';
 *
 * const input = '.................................';
 * zlib.deflate(input, (err, buffer) => {
 *   if (err) {
 *     console.error('An error occurred:', err);
 *     process.exitCode = 1;
 *   }
 *   console.log(buffer.toString('base64'));
 * });
 *
 * const buffer = Buffer.from('CwWASGVsbG8gV29ybGQD', 'base64');
 * zlib.brotliDecompress(buffer, (err, buffer) => {
 *   if (err) {
 *     console.error('An error occurred:', err);
 *     process.exitCode = 1;
 *   }
 *   console.log(buffer.toString());
 * });
 *
 * ```
 */
declare module "zlib" {
  import { Buffer } from "buffer";

  interface ZlibOptions {
    level?: number | undefined; // compression only
  }
  interface ZstdOptions {
    level?: number | undefined; // compression only
  }
  type InputType = string | ArrayBuffer | QuickJS.ArrayBufferView;
  type CompressCallback = (error: Error | null, result: Buffer) => void;

  /**
   * Compress a chunk of data with `Deflate`.
   */
  function deflate(buf: InputType, callback: CompressCallback): void;
  function deflate(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Compress a chunk of data with `Deflate`.
   */
  function deflateSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Compress a chunk of data with `DeflateRaw`.
   */
  function deflateRaw(buf: InputType, callback: CompressCallback): void;
  function deflateRaw(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Compress a chunk of data with `DeflateRaw`.
   */
  function deflateRawSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Compress a chunk of data with `Gzip`.
   */
  function gzip(buf: InputType, callback: CompressCallback): void;
  function gzip(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Compress a chunk of data with `Gzip`.
   */
  function gzipSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Decompress a chunk of data with `Inflate`.
   */
  function inflate(buf: InputType, callback: CompressCallback): void;
  function inflate(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Decompress a chunk of data with `Inflate`.
   */
  function inflateSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Decompress a chunk of data with `InflateRaw`.
   */
  function inflateRaw(buf: InputType, callback: CompressCallback): void;
  function inflateRaw(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Decompress a chunk of data with `InflateRaw`.
   */
  function inflateRawSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Decompress a chunk of data with `Gunzip`.
   */
  function gunzip(buf: InputType, callback: CompressCallback): void;
  function gunzip(
    buf: InputType,
    options: ZlibOptions,
    callback: CompressCallback
  ): void;
  /**
   * Decompress a chunk of data with `Gunzip`.
   */
  function gunzipSync(buf: InputType, options?: ZlibOptions): Buffer;

  /**
   * Compress a chunk of data with `BrotliCompress`.
   */
  function brotliCompress(buf: InputType, callback: CompressCallback): void;
  /**
   * Compress a chunk of data with `BrotliCompress`.
   */
  function brotliCompressSync(buf: InputType): Buffer;

  /**
   * Decompress a chunk of data with `BrotliDecompress`.
   */
  function brotliDecompress(buf: InputType, callback: CompressCallback): void;
  /**
   * Decompress a chunk of data with `BrotliDecompress`.
   */
  function brotliDecompressSync(buf: InputType): Buffer;

  /**
   * Compress a chunk of data with `ZstdCompress`.
   */
  function zstdCompress(buf: InputType, callback: CompressCallback): void;
  function zstdCompress(
    buf: InputType,
    options: ZstdOptions,
    callback: CompressCallback
  ): void;
  /**
   * Compress a chunk of data with `ZstdCompress`.
   */
  function zstdCompressSync(buf: InputType, options?: ZstdOptions): Buffer;

  /**
   * Decompress a chunk of data with `ZstdDecompress`.
   */
  function zstdDecompress(buf: InputType, callback: CompressCallback): void;
  function zstdDecompress(
    buf: InputType,
    options: ZstdOptions,
    callback: CompressCallback
  ): void;
  /**
   * Decompress a chunk of data with `ZstdDecompress`.
   */
  function zstdDecompressSync(buf: InputType, options?: ZstdOptions): Buffer;
}
