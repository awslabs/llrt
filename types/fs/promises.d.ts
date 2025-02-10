/**
 * The `fs/promises` API provides asynchronous file system methods that return
 * promises.
 */
declare module "fs/promises" {
  import { Buffer, BufferEncoding } from "buffer";
  import {
    constants as fsConstants,
    Dirent,
    MakeDirectoryOptions,
    Mode,
    PathLike,
    RmDirOptions,
    RmOptions,
    Stats,
  } from "fs";

  export type FileSystemFlags =
    | "a"
    | "ax"
    | "a+"
    | "r"
    | "r+"
    | "w"
    | "wx"
    | "w+"
    | "wx+";

  interface FileReadResult<T extends QuickJS.ArrayBufferView> {
    bytesRead: number;
    buffer: T;
  }

  interface FileReadOptions<T extends QuickJS.ArrayBufferView = Buffer> {
    /**
     * @default `Buffer.alloc(16384)`
     */
    buffer?: T;
    /**
     * @default 0
     */
    offset?: number | null;
    /**
     * @default `buffer.byteLength - offset`
     */
    length?: number | null;
    position?: number | null;
  }

  class FileHandle {
    /**
     * The numeric file descriptor managed by the {FileHandle} object.
     */
    readonly fd: number;

    /**
     * Changes the ownership of the file. A wrapper for [`chown(2)`](http://man7.org/linux/man-pages/man2/chown.2.html).
     * @param uid The file's new owner's user id.
     * @param gid The file's new group's group id.
     * @return Fulfills with `undefined` upon success.
     */
    chown(uid: number, gid: number): Promise<void>;

    /**
     * Modifies the permissions on the file. See [`chmod(2)`](http://man7.org/linux/man-pages/man2/chmod.2.html).
     * @param mode the file mode bit mask.
     * @return Fulfills with `undefined` upon success.
     */
    chmod(mode: Mode): Promise<void>;

    /**
     * Forces all currently queued I/O operations associated with the file to the
     * operating system's synchronized I/O completion state. Refer to the POSIX [`fdatasync(2)`](http://man7.org/linux/man-pages/man2/fdatasync.2.html) documentation for details.
     *
     * Unlike `filehandle.sync` this method does not flush modified metadata.
     * @return Fulfills with `undefined` upon success.
     */
    datasync(): Promise<void>;

    /**
     * Request that all data for the open file descriptor is flushed to the storage
     * device. The specific implementation is operating system and device specific.
     * Refer to the POSIX [`fsync(2)`](http://man7.org/linux/man-pages/man2/fsync.2.html) documentation for more detail.
     * @return Fulfills with `undefined` upon success.
     */
    sync(): Promise<void>;

    /**
     * Reads data from the file and stores that in the given buffer.
     *
     * If the file is not modified concurrently, the end-of-file is reached when the
     * number of bytes read is zero.
     * @param buffer A buffer that will be filled with the file data read.
     * @param offset The location in the buffer at which to start filling.
     * @param length The number of bytes to read.
     * @param position The location where to begin reading data from the file. If `null`, data will be read from the current file position, and the position will be updated. If `position` is an
     * integer, the current file position will remain unchanged.
     * @return Fulfills upon success with an object with two properties: bytesRead and buffer
     */
    read<T extends QuickJS.ArrayBufferView>(
      buffer: T,
      offset?: number | null,
      length?: number | null,
      position?: number | null
    ): Promise<FileReadResult<T>>;
    read<T extends QuickJS.ArrayBufferView = Buffer>(
      buffer: T,
      options?: FileReadOptions<T>
    ): Promise<FileReadResult<T>>;
    read<T extends QuickJS.ArrayBufferView = Buffer>(
      options?: FileReadOptions<T>
    ): Promise<FileReadResult<T>>;

    /**
     * Asynchronously reads the entire contents of a file.
     *
     * If `options` is a string, then it specifies the `encoding`.
     *
     * The `FileHandle` has to support reading.
     *
     * If one or more `filehandle.read()` calls are made on a file handle and then a `filehandle.readFile()` call is made, the data will be read from the current
     * position till the end of the file. It doesn't always read from the beginning
     * of the file.
     * @return Fulfills upon a successful read with the contents of the file. If no encoding is specified (using `options.encoding`), the data is returned as a {Buffer} object. Otherwise, the
     * data will be a string.
     */
    readFile(
      options?: {
        encoding?: null | undefined;
      } | null
    ): Promise<Buffer>;
    readFile(
      options:
        | {
            encoding: BufferEncoding;
          }
        | BufferEncoding
    ): Promise<string>;

    /**
     * Get {FileHandle} status.
     * @return Fulfills with the {fs.Stats} object.
     */
    stat(): Promise<Stats>;

    /**
     * Truncates the file.
     *
     * If the file was larger than `len` bytes, only the first `len` bytes will be
     * retained in the file.
     *
     * The following example retains only the first four bytes of the file:
     *
     * ```js
     * import { open } from 'fs/promises';
     *
     * let filehandle = null;
     * try {
     *   filehandle = await open('temp.txt', 'r+');
     *   await filehandle.truncate(4);
     * } finally {
     *   await filehandle?.close();
     * }
     * ```
     *
     * If the file previously was shorter than `len` bytes, it is extended, and the
     * extended part is filled with null bytes (`'\0'`):
     *
     * @param [len=0]
     * @return Fulfills with `undefined` upon success.
     */
    truncate(len?: number): Promise<void>;

    /**
     * Asynchronously writes data to a file, replacing the file if it already exists.
     *
     * If `options` is a string, then it specifies the `encoding`.
     *
     * The `FileHandle` has to support writing.
     *
     * It is unsafe to use `filehandle.writeFile()` multiple times on the same file
     * without waiting for the promise to be fulfilled (or rejected).
     *
     * If one or more `filehandle.write()` calls are made on a file handle and then a`filehandle.writeFile()` call is made, the data will be written from the
     * current position till the end of the file. It doesn't always write from the beginning of the file.
     */
    writeFile(
      data: string | QuickJS.ArrayBufferView,
      options?:
        | {
            encoding?: BufferEncoding | null;
          }
        | BufferEncoding
        | null
    ): Promise<void>;

    /**
     * Write `buffer` to the file.
     *
     * The promise is fulfilled with an object containing two properties:
     *
     * It is unsafe to use `filehandle.write()` multiple times on the same file
     * without waiting for the promise to be fulfilled (or rejected). For this
     * scenario, use `filehandle.createWriteStream()`.
     *
     * On Linux, positional writes do not work when the file is opened in append mode.
     * The kernel ignores the position argument and always appends the data to
     * the end of the file.
     * @param offset The start position from within `buffer` where the data to write begins.
     * @param [length=buffer.byteLength - offset] The number of bytes from `buffer` to write.
     * @param [position='null'] The offset from the beginning of the file where the data from `buffer` should be written. If `position` is not a `number`, the data will be written at the current
     * position. See the POSIX pwrite(2) documentation for more detail.
     */
    write<TBuffer extends QuickJS.ArrayBufferView>(
      buffer: TBuffer,
      offset?: number | null,
      length?: number | null,
      position?: number | null
    ): Promise<{
      bytesWritten: number;
      buffer: TBuffer;
    }>;
    write<TBuffer extends QuickJS.ArrayBufferView>(
      buffer: TBuffer,
      options?: {
        offset?: number | null;
        length?: number | null;
        position?: number | null;
      } | null
    ): Promise<{
      bytesWritten: number;
      buffer: TBuffer;
    }>;
    write(
      data: string,
      position?: number | null,
      encoding?: BufferEncoding | null
    ): Promise<{
      bytesWritten: number;
      buffer: string;
    }>;

    /**
     * Closes the file handle after waiting for any pending operation on the handle to
     * complete.
     *
     * ```js
     * import { open } from 'fs/promises';
     *
     * let filehandle;
     * try {
     *   filehandle = await open('thefile.txt', 'r');
     * } finally {
     *   await filehandle?.close();
     * }
     * ```
     * @return Fulfills with `undefined` upon success.
     */
    close(): Promise<void>;
  }

  const constants: typeof fsConstants;

  /**
   * Tests a user's permissions for the file or directory specified by `path`.
   * The `mode` argument is an optional integer that specifies the accessibility
   * checks to be performed. `mode` should be either the value `fs.constants.F_OK` or a mask consisting of the bitwise OR of any of `fs.constants.R_OK`, `fs.constants.W_OK`, and `fs.constants.X_OK`
   * (e.g.`fs.constants.W_OK | fs.constants.R_OK`). Check `File access constants` for
   * possible values of `mode`.
   *
   * If the accessibility check is successful, the promise is fulfilled with no
   * value. If any of the accessibility checks fail, the promise is rejected
   * with an [Error](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Error) object. The following example checks if the file`/etc/passwd` can be read and
   * written by the current process.
   *
   * ```js
   * import { access, constants } from 'fs/promises';
   *
   * try {
   *   await access('/etc/passwd', constants.R_OK | constants.W_OK);
   *   console.log('can access');
   * } catch {
   *   console.error('cannot access');
   * }
   * ```
   *
   * Using `fsPromises.access()` to check for the accessibility of a file before
   * calling `fsPromises.open()` is not recommended. Doing so introduces a race
   * condition, since other processes may change the file's state between the two
   * calls. Instead, user code should open/read/write the file directly and handle
   * the error raised if the file is not accessible.
   * @param [mode=fs.constants.F_OK]
   * @return Fulfills with `undefined` upon success.
   */
  function access(path: PathLike, mode?: Mode): Promise<void>;

  /**
   * Opens a `FileHandle`.
   *
   * Refer to the POSIX [`open(2)`](http://man7.org/linux/man-pages/man2/open.2.html) documentation for more detail.
   *
   * Some characters (`< > : " / \ | ? *`) are reserved under Windows as documented
   * by [Naming Files, Paths, and Namespaces](https://docs.microsoft.com/en-us/windows/desktop/FileIO/naming-a-file).
   * @param [flags='r'] See {FileSystemFlags}.
   * @param [mode=0o666] Sets the file mode if the file is created (UNIX).
   * @return Fulfills with a {FileHandle} object.
   */
  function open(
    path: PathLike,
    flags?: FileSystemFlags,
    mode?: Mode
  ): Promise<FileHandle>;

  /**
   * Removes the directory identified by `path`.
   *
   * Using `fsPromises.rmdir()` on a file (not a directory) results in the
   * promise being rejected with an `ENOENT` error on Windows and an `ENOTDIR` error on POSIX.
   *
   * To get a behavior similar to the `rm -rf` Unix command, use `fsPromises.rm()` with options `{ recursive: true, force: true }`.
   * @return Fulfills with `undefined` upon success.
   */
  function rmdir(path: PathLike, options?: RmDirOptions): Promise<void>;

  /**
   * Removes files and directories (modeled on the standard POSIX `rm` utility).
   * @return Fulfills with `undefined` upon success.
   */
  function rm(path: PathLike, options?: RmOptions): Promise<void>;

  /**
   * Asynchronously creates a directory.
   *
   * The optional `options` argument can be an object with a `mode` property and a `recursive` property indicating whether parent directories should be created.
   * Calling `fsPromises.mkdir()` when `path` is a directory that exists results in a rejection only when `recursive` is false.
   *
   * ```js
   * import { mkdir } from 'fs/promises';
   *
   * try {
   *   const projectFolder = './test/project/123';
   *   const createDir = await mkdir(projectFolder, { recursive: true });
   *
   *   console.log(`created ${createDir}`);
   * } catch (err) {
   *   console.error(err.message);
   * }
   * ```
   * @return Upon success, fulfills with `undefined` if `recursive` is `false`, or the first directory path created if `recursive` is `true`.
   */
  function mkdir(
    path: PathLike,
    options?: MakeDirectoryOptions
  ): Promise<string>;

  /**
   * Reads the contents of a directory.
   *
   * If `options.withFileTypes` is set to `true`, the returned array will contain `fs.Dirent` objects.
   *
   * ```js
   * import { readdir } from 'fs/promises';
   *
   * try {
   *   const files = await readdir(path);
   *   for (const file of files)
   *     console.log(file);
   * } catch (err) {
   *   console.error(err);
   * }
   * ```
   * @return Fulfills with an array of the names of the files in the directory excluding `'.'` and `'..'`.
   */
  function readdir(
    path: PathLike,
    options?: {
      withFileTypes?: false | undefined;
      recursive?: boolean | undefined;
    }
  ): Promise<string[]>;

  /**
   * Asynchronous readdir(2) - read a directory.
   * @param path A path to a file. If a URL is provided, it must use the `file:` protocol.
   * @param options If called with `withFileTypes: true` the result data will be an array of Dirent.
   */
  function readdir(
    path: PathLike,
    options: {
      withFileTypes: true;
      recursive?: boolean | undefined;
    }
  ): Promise<Dirent[]>;

  /**
   * Asynchronous stat - Get file status.
   * @param path A path to a file.
   * @return Fulfills with the {fs.Stats} object for the given `path`.
   */
  function stat(path: PathLike): Promise<Stats>;

  /**
   * Creates a unique temporary directory. A unique directory name is generated by
   * appending six random characters to the end of the provided `prefix`. Due to
   * platform inconsistencies, avoid trailing `X` characters in `prefix`. Some
   * platforms, notably the BSDs, can return more than six random characters, and
   * replace trailing `X` characters in `prefix` with random characters.
   *
   * ```js
   * import { mkdtemp } from 'fs/promises';
   * import { join } from 'path';
   * import { tmpdir } from 'os';
   *
   * try {
   *   await mkdtemp(join(tmpdir(), 'foo-'));
   * } catch (err) {
   *   console.error(err);
   * }
   * ```
   *
   * The `fsPromises.mkdtemp()` method will append the six randomly selected
   * characters directly to the `prefix` string. For instance, given a directory `/tmp`, if the intention is to create a temporary directory _within_ `/tmp`, the `prefix` must end with a trailing
   * platform-specific path separator
   * (`require('path').sep`).
   * @return Fulfills with a string containing the file system path of the newly created temporary directory.
   */
  function mkdtemp(prefix: string): Promise<string>;

  /**
   * Asynchronously writes data to a file, replacing the file if it already exists.
   *
   * The `encoding` option is ignored if `data` is a buffer.
   *
   * It is unsafe to use `fsPromises.writeFile()` multiple times on the same file
   * without waiting for the promise to be settled.
   *
   * Similarly to `fsPromises.readFile` \- `fsPromises.writeFile` is a convenience
   * method that performs multiple `write` calls internally to write the buffer
   * passed to it.
   *
   * @param file filename or `FileHandle`
   * @return Fulfills with `undefined` upon success.
   */
  function writeFile(
    file: PathLike,
    data:
      | string
      | Buffer
      | QuickJS.ArrayBufferView
      | ArrayBuffer
      | SharedArrayBuffer
  ): Promise<void>;

  /**
   * Asynchronously reads the entire contents of a file.
   *
   * If no encoding is specified (using `options.encoding`), the data is returned
   * as a `Buffer` object. Otherwise, the data will be a string.
   *
   * If `options` is a string, then it specifies the encoding.
   *
   * When the `path` is a directory, the behavior of `fsPromises.readFile()` is
   * platform-specific. On macOS, Linux, and Windows, the promise will be rejected
   * with an error. On FreeBSD, a representation of the directory's contents will be
   * returned.
   *
   * An example of reading a `package.json` file.
   *
   * ```js
   * import { readFile } from 'fs/promises';
   * try {
   *   const filePath = './package.json';
   *   const contents = await readFile(filePath, { encoding: 'utf8' });
   *   console.log(contents);
   * } catch (err) {
   *   console.error(err.message);
   * }
   * ```
   *
   * @param path filename or `FileHandle`
   * @return Fulfills with the contents of the file.
   */
  function readFile(
    path: PathLike,
    options?: {
      encoding?: null | undefined;
    } | null
  ): Promise<Buffer>;

  /**
   * Asynchronously reads the entire contents of a file.
   * @param path A path to a file. If a URL is provided, it must use the `file:` protocol.
   * If a `FileHandle` is provided, the underlying file will _not_ be closed automatically.
   * @param options An object that may contain an optional flag.
   * If a flag is not provided, it defaults to `'r'`.
   */
  function readFile(
    path: PathLike,
    options:
      | {
          encoding: BufferEncoding;
        }
      | BufferEncoding
  ): Promise<string>;

  /**
   * Changes the permissions of a file.
   * @return Fulfills with `undefined` upon success.
   */
  function chmod(path: PathLike, mode: Mode): Promise<void>;
}
