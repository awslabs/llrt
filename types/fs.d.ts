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
  export { promises };

  /**
   * Valid types for path values in "fs".
   */
  export type PathLike = string;
  export type Mode = number;

  /**
   * A representation of a directory entry, which can be a file or a subdirectory
   * within the directory. A directory entry is a combination of the file name and file type pairs.
   *
   * Additionally, when {@link readdir} or {@link readdirSync} is called with
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
   * this API: {@link mkdtemp}.
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
  export function accessSync(path: PathLike, mode?: number): void;
}
