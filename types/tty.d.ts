declare module "tty" {
  /**
   * The `tty.isatty()` method returns `true` if the given `fd` is associated with
   * a TTY and `false` if it is not, including whenever `fd` is not a non-negative
   * integer.
   * @since v0.5.8
   * @param fd A numeric file descriptor
   */
  function isatty(fd: number): boolean;

  /**
   * Represents a writable stream connected to a TTY.
   * Instances are exposed as `process.stdout` and `process.stderr`.
   */
  class WriteStream {
    constructor(fd: number);
    readonly fd: number;
    /** Number of columns in the terminal window. Each access issues an ioctl; use getWindowSize() when both dimensions are needed. */
    readonly columns: number;
    /** Number of rows in the terminal window. Each access issues an ioctl; use getWindowSize() when both dimensions are needed. */
    readonly rows: number;
    /** `true` if the stream is connected to a TTY. */
    readonly isTTY: boolean;
    /** Write a string to the stream. Returns `true` on success. */
    write(data: string): boolean;
    /** Enable or disable raw mode. Returns `true` on success. Only supported for fd 0–2. */
    setRawMode(enable: boolean): boolean;
    /** Returns `[columns, rows]` from a single ioctl call. */
    getWindowSize(): [number, number];
  }

  /**
   * Represents a readable stream connected to a TTY.
   * Exposed as `process.stdin`.
   */
  class ReadStream {
    constructor(fd: number);
    readonly fd: number;
    /** `true` if the stream is connected to a TTY. */
    readonly isTTY: boolean;
    /** Enable or disable raw mode. Returns `true` on success. Only supported for fd 0–2. */
    setRawMode(enable: boolean): boolean;
  }
}
