declare module "child_process" {
  import { EventEmitter } from "events";
  import {
    DefaultReadableStream as Readable,
    DefaultWritableStream as Writable,
  } from "stream";

  /**
   * Instances of the `ChildProcess` represent spawned child processes.
   *
   * Instances of `ChildProcess` are not intended to be created directly. Rather,
   * use the {@link spawn} method to create instances of `ChildProcess`.
   */
  class ChildProcess extends EventEmitter {
    /**
     * A `Writable Stream` that represents the child process's `stdin`.
     *
     * If a child process waits to read all of its input, the child will not continue
     * until this stream has been closed via `end()`.
     *
     * If the child was spawned with `stdio[0]` set to anything other than `'pipe'`,
     * then this will be `null`.
     *
     * `subprocess.stdin` is an alias for `subprocess.stdio[0]`. Both properties will
     * refer to the same value.
     *
     * The `subprocess.stdin` property can be `null` or `undefined` if the child process could not be successfully spawned.
     */
    stdin: Writable | null;

    /**
     * A `Readable Stream` that represents the child process's `stdout`.
     *
     * If the child was spawned with `stdio[1]` set to anything other than `'pipe'`,
     * then this will be `null`.
     *
     * `subprocess.stdout` is an alias for `subprocess.stdio[1]`. Both properties will
     * refer to the same value.
     *
     * ```js
     * const { spawn } = require('child_process');
     *
     * const subprocess = spawn('ls');
     *
     * subprocess.stdout.on('data', (data) => {
     *   console.log(`Received chunk ${data}`);
     * });
     * ```
     *
     * The `subprocess.stdout` property can be `null` or `undefined` if the child process could not be successfully spawned.
     */
    stdout: Readable | null;

    /**
     * A `Readable Stream` that represents the child process's `stderr`.
     *
     * If the child was spawned with `stdio[2]` set to anything other than `'pipe'`,
     * then this will be `null`.
     *
     * `subprocess.stderr` is an alias for `subprocess.stdio[2]`. Both properties will
     * refer to the same value.
     *
     * The `subprocess.stderr` property can be `null` or `undefined` if the child process could not be successfully spawned.
     */
    stderr: Readable | null;

    /**
     * Returns the process identifier (PID) of the child process. If the child process
     * fails to spawn due to errors, then the value is `undefined` and `error` is
     * emitted.
     *
     * ```js
     * const { spawn } = require('child_process');
     * const grep = spawn('grep', ['ssh']);
     *
     * console.log(`Spawned child pid: ${grep.pid}`);
     * grep.stdin.end();
     * ```
     */
    readonly pid?: number | undefined;

    /**
     * The `subprocess.kill()` method sends a signal to the child process. If no
     * argument is given, the process will be sent the `'SIGTERM'` signal. See [`signal(7)`](http://man7.org/linux/man-pages/man7/signal.7.html) for a list of available signals. This function
     * returns `true` if [`kill(2)`](http://man7.org/linux/man-pages/man2/kill.2.html) succeeds, and `false` otherwise.
     *
     * ```js
     * const { spawn } = require('child_process');
     * const grep = spawn('grep', ['ssh']);
     *
     * grep.on('close', (code, signal) => {
     *   console.log(
     *     `child process terminated due to receipt of signal ${signal}`);
     * });
     *
     * // Send SIGHUP to process.
     * grep.kill('SIGHUP');
     * ```
     *
     * The `ChildProcess` object may emit an `'error'` event if the signal
     * cannot be delivered. Sending a signal to a child process that has already exited
     * is not an error but may have unforeseen consequences. Specifically, if the
     * process identifier (PID) has been reassigned to another process, the signal will
     * be delivered to that process instead which can have unexpected results.
     *
     * While the function is called `kill`, the signal delivered to the child process
     * may not actually terminate the process.
     *
     * See [`kill(2)`](http://man7.org/linux/man-pages/man2/kill.2.html) for reference.
     *
     * On Windows, where POSIX signals do not exist, the `signal` argument will be
     * ignored, and the process will be killed forcefully and abruptly (similar to `'SIGKILL'`).
     * See `Signal Events` for more details.
     *
     * On Linux, child processes of child processes will not be terminated
     * when attempting to kill their parent. This is likely to happen when running a
     * new process in a shell or with the use of the `shell` option of `ChildProcess`:
     *
     * ```js
     * 'use strict';
     * const { spawn } = require('child_process');
     *
     * const subprocess = spawn(
     *   'sh',
     *   [
     *     '-c',
     *     `node -e "setInterval(() => {
     *       console.log(process.pid, 'is alive')
     *     }, 500);"`,
     *   ], {
     *     stdio: ['inherit', 'inherit', 'inherit'],
     *   },
     * );
     *
     * setTimeout(() => {
     *   subprocess.kill(); // Does not terminate the Node.js process in the shell.
     * }, 2000);
     * ```
     */
    kill(signal?: QuickJS.Signals | number): boolean;

    /**
     * Calls {@link ChildProcess.kill} with `'SIGTERM'`.
     */
    [Symbol.dispose](): void;

    /**
     * events.EventEmitter
     * 1. close
     * 2. error
     * 3. exit
     */
    addListener(event: string, listener: (...args: any[]) => void): this;
    addListener(
      event: "close",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    addListener(event: "error", listener: (err: Error) => void): this;
    addListener(
      event: "exit",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    emit(event: string | symbol, ...args: any[]): boolean;
    emit(
      event: "close",
      code: number | null,
      signal: QuickJS.Signals | null
    ): boolean;
    emit(event: "error", err: Error): boolean;
    emit(
      event: "exit",
      code: number | null,
      signal: QuickJS.Signals | null
    ): boolean;
    on(event: string, listener: (...args: any[]) => void): this;
    on(
      event: "close",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(
      event: "exit",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    once(event: string, listener: (...args: any[]) => void): this;
    once(
      event: "close",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    once(event: "error", listener: (err: Error) => void): this;
    once(
      event: "exit",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    prependListener(event: string, listener: (...args: any[]) => void): this;
    prependListener(
      event: "close",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    prependListener(event: "error", listener: (err: Error) => void): this;
    prependListener(
      event: "exit",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    prependOnceListener(
      event: string,
      listener: (...args: any[]) => void
    ): this;
    prependOnceListener(
      event: "close",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
    prependOnceListener(event: "error", listener: (err: Error) => void): this;
    prependOnceListener(
      event: "exit",
      listener: (code: number | null, signal: QuickJS.Signals | null) => void
    ): this;
  }
  // return this object when stdio option is undefined or not specified
  interface ChildProcessWithoutNullStreams extends ChildProcess {
    stdin: Writable;
    stdout: Readable;
    stderr: Readable;
  }
  // return this object when stdio option is a tuple of 3
  interface ChildProcessByStdio<
    I extends null | Writable,
    O extends null | Readable,
    E extends null | Readable,
  > extends ChildProcess {
    stdin: I;
    stdout: O;
    stderr: E;
  }

  type IOType = "pipe" | "ignore" | "inherit";
  type StdioOptions = IOType | Array<IOType | number | null | undefined>;

  interface ProcessEnvOptions {
    uid?: number | undefined;
    gid?: number | undefined;
    cwd?: string | undefined;
  }
  interface SpawnOptions extends ProcessEnvOptions {
    /**
     * Can be set to 'pipe', 'inherit', or 'ignore', or an array of these strings.
     * If passed as an array, the first element is used for `stdin`, the second for
     * `stdout`, and the third for `stderr`.
     *
     * @default 'pipe'
     */
    stdio?: StdioOptions | undefined;
    shell?: boolean | string | undefined;
    windowsVerbatimArguments?: boolean | undefined;
  }
  interface SpawnOptionsWithoutStdio extends SpawnOptions {
    stdio?: StdioPipeNamed | StdioPipe[] | undefined;
  }
  type StdioNull = "inherit" | "ignore";
  type StdioPipeNamed = "pipe";
  type StdioPipe = undefined | null | StdioPipeNamed;
  interface SpawnOptionsWithStdioTuple<
    Stdin extends StdioNull | StdioPipe,
    Stdout extends StdioNull | StdioPipe,
    Stderr extends StdioNull | StdioPipe,
  > extends SpawnOptions {
    stdio: [Stdin, Stdout, Stderr];
  }

  /**
   * The `child_process.spawn()` method spawns a new process using the given `command`, with command-line arguments in `args`.
   * If omitted, `args` defaults to an empty array.
   *
   * **If the `shell` option is enabled, do not pass unsanitized user input to this**
   * **function. Any input containing shell metacharacters may be used to trigger**
   * **arbitrary command execution.**
   *
   * A third argument may be used to specify additional options.
   *
   * Use `cwd` to specify the working directory from which the process is spawned.
   * If not given, the default is to inherit the current working directory. If given,
   * but the path does not exist, the child process emits an `ENOENT` error
   * and exits immediately. `ENOENT` is also emitted when the command
   * does not exist.
   *
   * Example of running `ls -lh /usr`, capturing `stdout`, `stderr`, and the
   * exit code:
   *
   * ```js
   * const { spawn } = require('child_process');
   * const ls = spawn('ls', ['-lh', '/usr']);
   *
   * ls.stdout.on('data', (data) => {
   *   console.log(`stdout: ${data}`);
   * });
   *
   * ls.stderr.on('data', (data) => {
   *   console.error(`stderr: ${data}`);
   * });
   *
   * ls.on('close', (code) => {
   *   console.log(`child process exited with code ${code}`);
   * });
   * ```
   *
   * Example: A very elaborate way to run `ps ax | grep ssh`
   *
   * ```js
   * const { spawn } = require('child_process');
   * const ps = spawn('ps', ['ax']);
   * const grep = spawn('grep', ['ssh']);
   *
   * ps.stdout.on('data', (data) => {
   *   grep.stdin.write(data);
   * });
   *
   * ps.stderr.on('data', (data) => {
   *   console.error(`ps stderr: ${data}`);
   * });
   *
   * ps.on('close', (code) => {
   *   if (code !== 0) {
   *     console.log(`ps process exited with code ${code}`);
   *   }
   *   grep.stdin.end();
   * });
   *
   * grep.stdout.on('data', (data) => {
   *   console.log(data.toString());
   * });
   *
   * grep.stderr.on('data', (data) => {
   *   console.error(`grep stderr: ${data}`);
   * });
   *
   * grep.on('close', (code) => {
   *   if (code !== 0) {
   *     console.log(`grep process exited with code ${code}`);
   *   }
   * });
   * ```
   *
   * Example of checking for failed `spawn`:
   *
   * ```js
   * const { spawn } = require('child_process');
   * const subprocess = spawn('bad_command');
   *
   * subprocess.on('error', (err) => {
   *   console.error('Failed to start subprocess.');
   * });
   * ```
   *
   * Certain platforms (macOS, Linux) will use the value of `argv[0]` for the process
   * title while others (Windows, SunOS) will use `command`.
   * @param command The command to run.
   * @param args List of string arguments.
   */
  function spawn(
    command: string,
    options?: SpawnOptionsWithoutStdio
  ): ChildProcessWithoutNullStreams;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioPipe, StdioPipe>
  ): ChildProcessByStdio<Writable, Readable, Readable>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioPipe, StdioNull>
  ): ChildProcessByStdio<Writable, Readable, null>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioNull, StdioPipe>
  ): ChildProcessByStdio<Writable, null, Readable>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioPipe, StdioPipe>
  ): ChildProcessByStdio<null, Readable, Readable>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioNull, StdioNull>
  ): ChildProcessByStdio<Writable, null, null>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioPipe, StdioNull>
  ): ChildProcessByStdio<null, Readable, null>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioNull, StdioPipe>
  ): ChildProcessByStdio<null, null, Readable>;
  function spawn(
    command: string,
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioNull, StdioNull>
  ): ChildProcessByStdio<null, null, null>;
  function spawn(command: string, options: SpawnOptions): ChildProcess;
  // overloads of spawn with 'args'
  function spawn(
    command: string,
    args?: readonly string[],
    options?: SpawnOptionsWithoutStdio
  ): ChildProcessWithoutNullStreams;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioPipe, StdioPipe>
  ): ChildProcessByStdio<Writable, Readable, Readable>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioPipe, StdioNull>
  ): ChildProcessByStdio<Writable, Readable, null>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioNull, StdioPipe>
  ): ChildProcessByStdio<Writable, null, Readable>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioPipe, StdioPipe>
  ): ChildProcessByStdio<null, Readable, Readable>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioPipe, StdioNull, StdioNull>
  ): ChildProcessByStdio<Writable, null, null>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioPipe, StdioNull>
  ): ChildProcessByStdio<null, Readable, null>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioNull, StdioPipe>
  ): ChildProcessByStdio<null, null, Readable>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptionsWithStdioTuple<StdioNull, StdioNull, StdioNull>
  ): ChildProcessByStdio<null, null, null>;
  function spawn(
    command: string,
    args: readonly string[],
    options: SpawnOptions
  ): ChildProcess;
}
