/**
 * The `fs` module enables interacting with the file system in a
 * way modeled on standard POSIX functions.
 *
 * To use the promise-based APIs:
 *
 * ```js
 * import * as fs from 'fs/promises';
 * ```
 *
 * To use the sync APIs:
 *
 * ```js
 * import * as fs from 'fs';
 * ```
 *
 * All file system operations have synchronous and promise-based forms.
 */
declare module "fs" {
  import * as promises from "fs/promises";
  import { Buffer, BufferEncoding } from "buffer";
  export { promises };

  /**
   * Valid types for path values in "fs".
   */
  export type PathLike = string;
  export type Mode = number;

  export interface StatsBase<T> {
    isFile(): boolean;
    isDirectory(): boolean;
    isBlockDevice(): boolean;
    isCharacterDevice(): boolean;
    isSymbolicLink(): boolean;
    isFIFO(): boolean;
    isSocket(): boolean;
    dev: T;
    ino: T;
    mode: T;
    nlink: T;
    uid: T;
    gid: T;
    rdev: T;
    size: T;
    blksize: T;
    blocks: T;
    atimeMs: T;
    mtimeMs: T;
    ctimeMs: T;
    birthtimeMs: T;
    atime: Date;
    mtime: Date;
    ctime: Date;
    birthtime: Date;
  }
  export interface Stats extends StatsBase<number> {}
  /**
   * A `fs.Stats` object provides information about a file.
   *
   * `Stat` objects are not to be created directly using the `new` keyword.
   *
   * ```console
   * Stats {
   *   dev: 2114,
   *   ino: 48064969,
   *   mode: 33188,
   *   nlink: 1,
   *   uid: 85,
   *   gid: 100,
   *   rdev: 0,
   *   size: 527,
   *   blksize: 4096,
   *   blocks: 8,
   *   atimeMs: 1318289051000.1,
   *   mtimeMs: 1318289051000.1,
   *   ctimeMs: 1318289051000.1,
   *   birthtimeMs: 1318289051000.1,
   *   atime: Mon, 10 Oct 2011 23:24:11 GMT,
   *   mtime: Mon, 10 Oct 2011 23:24:11 GMT,
   *   ctime: Mon, 10 Oct 2011 23:24:11 GMT,
   *   birthtime: Mon, 10 Oct 2011 23:24:11 GMT }
   * ```
   */
  export class Stats {}

  /**
   * A representation of a directory entry, which can be a file or a subdirectory
   * within the directory. A directory entry is a combination of the file name and file type pairs.
   *
   * Additionally, when {@link promises.readdir} or {@link readdirSync} is called with
   * the `withFileTypes` option set to `true`, the resulting array is filled with `fs.Dirent` objects, rather than strings.
   */
  export class Dirent {
    /**
     * Returns `true` if the `fs.Dirent` object describes a regular file.
     */
    isFile(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a file system
     * directory.
     */
    isDirectory(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a block device.
     */
    isBlockDevice(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a character device.
     */
    isCharacterDevice(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a symbolic link.
     */
    isSymbolicLink(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a first-in-first-out
     * (FIFO) pipe.
     */
    isFIFO(): boolean;
    /**
     * Returns `true` if the `fs.Dirent` object describes a socket.
     */
    isSocket(): boolean;
    /**
     * The file name that this `fs.Dirent` object refers to.
     */
    name: string;
    /**
     * The base path that this `fs.Dirent` object refers to.
     */
    parentPath: string;
  }

  export interface StatSyncFn extends Function {
    (path: PathLike): Stats;
  }
  /**
   * Synchronous stat - Get file status.
   * @param path A path to a file.
   */
  export const statSync: StatSyncFn;

  export interface RmDirOptions {
    /**
     * @deprecated Use `fs.rm(path, { recursive: true, force: true })` instead.
     *
     * If `true`, perform a recursive directory removal. In
     * recursive mode, operations are retried on failure.
     * @default false
     */
    recursive?: boolean | undefined;
  }

  /**
   * Synchronous [`rmdir(2)`](http://man7.org/linux/man-pages/man2/rmdir.2.html). Returns `undefined`.
   *
   * Using `fs.rmdirSync()` on a file (not a directory) results in an `ENOENT` error
   * on Windows and an `ENOTDIR` error on POSIX.
   *
   * To get a behavior similar to the `rm -rf` Unix command, use {@link rmSync} with options `{ recursive: true, force: true }`.
   */
  export function rmdirSync(path: PathLike, options?: RmDirOptions): void;

  export interface RmOptions {
    /**
     * When `true`, exceptions will be ignored if `path` does not exist.
     * @default false
     */
    force?: boolean | undefined;
    /**
     * If `true`, perform a recursive directory removal. In
     * recursive mode, operations are retried on failure.
     * @default false
     */
    recursive?: boolean | undefined;
  }

  /**
   * Synchronously removes files and directories (modeled on the standard POSIX `rm` utility). Returns `undefined`.
   */
  export function rmSync(path: PathLike, options?: RmOptions): void;

  export interface MakeDirectoryOptions {
    /**
     * Indicates whether parent folders should be created.
     * If a folder was created, the path to the first created folder will be returned.
     * @default false
     */
    recursive?: boolean | undefined;
    /**
     * A file mode. If not specified
     * @default 0o777
     */
    mode?: Mode | undefined;
  }

  /**
   * Synchronously creates a directory. Returns the `path`.
   *
   * See the POSIX [`mkdir(2)`](http://man7.org/linux/man-pages/man2/mkdir.2.html) documentation for more details.
   */
  export function mkdirSync(
    path: PathLike,
    options?: MakeDirectoryOptions
  ): string;

  /**
   * Returns the created directory path.
   *
   * For detailed information, see the documentation of the asynchronous version of
   * this API: {@link promises.mkdtemp}.
   */
  export function mkdtempSync(prefix: string): string;

  /**
   * Reads the contents of the directory.
   *
   * See the POSIX [`readdir(3)`](http://man7.org/linux/man-pages/man3/readdir.3.html) documentation for more details.
   *
   * If `options.withFileTypes` is set to `true`, the result will contain `fs.Dirent` objects.
   */
  export function readdirSync(
    path: PathLike,
    options?: {
      withFileTypes?: false | undefined;
      recursive?: boolean | undefined;
    }
  ): string[];

  /**
   * Synchronous readdir (2) - read a directory.
   * @param path A path to a file. If a URL is provided, it must use the `file:` protocol.
   * @param options If called with `withFileTypes: true` the result data will be an array of Dirent.
   */
  export function readdirSync(
    path: PathLike,
    options: {
      withFileTypes: true;
      recursive?: boolean | undefined;
    }
  ): Dirent[];

  /**
   * Returns the contents of the `path`.
   *
   * For detailed information, see the documentation of the asynchronous version of
   * this API: {@link promises.readFile}.
   *
   * If the `encoding` option is specified then this function returns a
   * string. Otherwise it returns a buffer.
   *
   * @param path A path to a file.
   */
  export function readFileSync(
    path: PathLike,
    options?: {
      encoding?: null | undefined;
    } | null
  ): Buffer;

  /**
   * Synchronously reads the entire contents of a file.
   * @param path A path to a file.
   * @param options Either the encoding for the result, or an object that contains the encoding.
   */
  export function readFileSync(
    path: PathLike,
    options:
      | {
          encoding: BufferEncoding;
        }
      | BufferEncoding
  ): string;

  /**
   * Returns `undefined`.
   *
   * For detailed information, see the documentation of the asynchronous version of
   * this API: {@link promises.writeFile}.
   * @param file A path to a file.
   */
  export function writeFileSync(
    file: PathLike,
    data:
      | string
      | Buffer
      | QuickJS.ArrayBufferView
      | ArrayBuffer
      | SharedArrayBuffer
  ): void;

  export namespace constants {
    // File Access Constants
    /** Constant for fs.access(). File is visible to the calling process. */
    const F_OK: number;
    /** Constant for fs.access(). File can be read by the calling process. */
    const R_OK: number;
    /** Constant for fs.access(). File can be written by the calling process. */
    const W_OK: number;
    /** Constant for fs.access(). File can be executed by the calling process. */
    const X_OK: number;
  }

  /**
   * Synchronously tests a user's permissions for the file or directory specified
   * by `path`. The `mode` argument is an optional integer that specifies the
   * accessibility checks to be performed. `mode` should be either the value `fs.constants.F_OK` or a mask consisting of the bitwise OR of any of `fs.constants.R_OK`, `fs.constants.W_OK`, and
   * `fs.constants.X_OK` (e.g.`fs.constants.W_OK | fs.constants.R_OK`). Check `File access constants` for
   * possible values of `mode`.
   *
   * If any of the accessibility checks fail, an `Error` will be thrown. Otherwise,
   * the method will return `undefined`.
   *
   * ```js
   * import { accessSync, constants } from 'fs';
   *
   * try {
   *   accessSync('etc/passwd', constants.R_OK | constants.W_OK);
   *   console.log('can read/write');
   * } catch (err) {
   *   console.error('no access!');
   * }
   * ```
   * @param [mode=fs.constants.F_OK]
   */
  export function accessSync(path: PathLike, mode?: Mode): void;
}

declare module "node:fs" {
  export * from "fs";
}
