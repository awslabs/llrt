declare module "process" {
  import { EventEmitter } from "events";

  global {
    var process: Process;

    namespace QuickJS {
      type Signals =
        | "SIGABRT"
        | "SIGALRM"
        | "SIGFPE"
        | "SIGHUP"
        | "SIGILL"
        | "SIGINT"
        | "SIGKILL"
        | "SIGPIPE"
        | "SIGQUIT"
        | "SIGSEGV"
        | "SIGTERM";
    }
    type Platform = "darwin" | "linux" | "win32";
    type Architecture = "arm64" | "x64";
  }
  interface Dict<T> {
    [key: string]: T | undefined;
  }
  // Alias for compatibility
  interface ProcessEnv extends Dict<string> {}
  interface HRTime {
    (): [number, number];

    /**
     * The `bigint` version of the `{@link process.hrtime()}` method returning the current high-resolution real time in nanoseconds as a `bigint`.
     */
    bigint(): bigint;
  }
  interface ProcessRelease {
    name: string;
  }
  interface ProcessVersions extends Dict<string> {
    llrt: string;
  }
  interface Process extends EventEmitter {
    hrtime: HRTime;
    /**
     * The `process.cwd()` method returns the current working directory of the llrt
     * process.
     *
     * ```js
     * import { cwd } from 'process';
     *
     * console.log(`Current directory: ${cwd()}`);
     * ```
     */
    cwd(): string;
    /**
     * The `process.argv0` property stores a read-only copy of the original value of`argv[0]` passed when llrt starts.
     *
     * ```console
     * $ ./llrt -e 'console.log(process.argv[0])'
     * ./llrt
     * $ ./llrt -e 'console.log(process.argv0)'
     * ./llrt
     * ```
     */
    argv0: string;
    /**
     * Returns the OS-assigned process identifier associated with this process.
     */
    id: number;
    /**
     * The `process.argv` property returns an array containing the command-line
     * arguments passed when the llrt process was launched. The first element will
     * be {@link execPath}. See `process.argv0` if access to the original value
     * of `argv[0]` is needed. The second element will be the path to the JavaScript
     * file being executed. The remaining elements will be any additional command-line
     * arguments.
     *
     * For example, assuming the following script for `process-args.js`:
     *
     * ```js
     * import { argv } from 'process';
     *
     * // print process.argv
     * argv.forEach((val, index) => {
     *   console.log(`${index}: ${val}`);
     * });
     * ```
     * ```
     */
    argv: string[];
    /**
     * The operating system CPU architecture for which the llrt binary was compiled.
     * Possible values are: `'arm64'` and `'x64'`.
     *
     * ```js
     * import { arch } from 'process';
     *
     * console.log(`This processor architecture is ${arch}`);
     * ```
     */
    readonly arch: Architecture;
    /**
     * The `process.platform` property returns a string identifying the operating
     * system platform for which the llrt binary was compiled.
     *
     * Currently possible values are:
     *
     * * `'darwin'`
     * * `'linux'`
     * * `'win32'`
     *
     * ```js
     * import { platform } from 'process';
     *
     * console.log(`This platform is ${platform}`);
     * ```
     */
    readonly platform: Platform;

    /**
     * The `process.release` property returns an `Object` containing metadata related
     * to the current release, including URLs for the source tarball and headers-only
     * tarball.
     *
     * `process.release` contains the following properties:
     *
     * ```js
     * {
     *   name: 'llrt',
     * }
     * ```
     *
     * In custom builds from non-release versions of the source tree, only the `name` property may be present. The additional properties should not be
     * relied upon to exist.
     */
    readonly release: ProcessRelease;
    /**
     * The `process.version` property contains the llrt version string.
     *
     * ```js
     * import { version } from 'process';
     *
     * console.log(`Version: ${version}`);
     * // Version: 0.1.15
     * ```
     */
    readonly version: string;
    /**
     * The `process.versions` property returns an object listing the version strings of
     * llrt and its dependencies. `process.versions.modules` indicates the current
     * ABI version, which is increased whenever a C++ API changes. llrt will refuse
     * to load modules that were compiled against a different module ABI version.
     *
     * ```js
     * import { versions } from 'process';
     *
     * console.log(versions);
     * ```
     *
     * Will generate an object similar to:
     *
     * ```console
     * {
     *   llrt:'0.1.15'
     * }
     * ```
     */
    readonly versions: ProcessVersions;
    /**
     * The `process.exit()` method instructs llrt to terminate the process
     * synchronously with an exit status of `code`. If `code` is omitted, exit uses
     * either the 'success' code `0` or the value of `process.exitCode` if it has been
     * set. llrt will not terminate until all the `'exit'` event listeners are
     * called.
     *
     * To exit with a 'failure' code:
     *
     * ```js
     * import { exit } from 'process';
     *
     * exit(1);
     * ```
     */
    exit(code?: number | string | null | undefined): never;

    /**
     * The `process.kill()` method sends the `signal` to the process identified by `pid`.
     *
     * Signal names are strings such as 'SIGINT' or 'SIGHUP'. See Signal Events and kill(2)
     * for more information.
     *
     * This method will throw an error if the target pid does not exist. As a special case,
     * a signal of 0 can be used to test for the existence of a process. Windows platforms
     * will throw an error if the pid is used to kill a process group.
     *
     * Even though the name of this function is process.kill(), it is really just a signal
     * sender, like the kill system call. The signal sent may do something other than kill
     * the target process.
     *
     * ```js
     * import { kill } from 'process';
     *
     * process.on('SIGHUP', () => {
     *   console.log('Got SIGHUP signal.');
     * });
     *
     * setTimeout(() => {
     *   console.log('Exiting.');
     *   process.exit(0);
     * }, 100);
     *
     * kill(process.pid, 'SIGHUP');
     * ```
     *
     * When `SIGUSR1` is received by a Node.js process, Node.js will start the debugger.
     * See the debugger documentation for more information.
     *
     * @since v0.1.31
     * @param pid A process ID
     * @param signal The signal to send, either as a string or number. Default: 'SIGTERM'.
     */
    kill(pid: number, signal?: QuickJS.Signals | number): void;

    /**
     * The `process.exitCode` property indicates the exit code that will be used
     * when the llrt process eventually exits. If it is not specified, the default
     * exit code is `undefined` and will be 0 on exit.
     *
     * ```js
     * import { exitCode, exit } from 'process';
     *
     * exitCode = 42;
     * exit();
     * ```
     *
     * This will cause the llrt process to exit with the exit code `42`.
     */
    exitCode: number | null;

    /**
     * The `process.getgid()` method returns the numerical group identity of the
     * process. (See [`getgid(2)`](http://man7.org/linux/man-pages/man2/getgid.2.html).)
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getgid) {
     *   console.log(`Current gid: ${process.getgid()}`);
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * @since v0.1.31
     */
    getgid?: () => number;
    /**
     * The `process.setgid()` method sets the group identity of the process. (See [`setgid(2)`](http://man7.org/linux/man-pages/man2/setgid.2.html).) The `id` can be passed as either a
     * numeric ID or a group name
     * string. If a group name is specified, this method blocks while resolving the
     * associated numeric ID.
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getgid &#x26;&#x26; process.setgid) {
     *   console.log(`Current gid: ${process.getgid()}`);
     *   try {
     *     process.setgid(501);
     *     console.log(`New gid: ${process.getgid()}`);
     *   } catch (err) {
     *     console.log(`Failed to set gid: ${err}`);
     *   }
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * This feature is not available in `Worker` threads.
     * @since v0.1.31
     * @param id The group name or ID
     */
    setgid?: (id: number) => void;
    /**
     * The `process.getuid()` method returns the numeric user identity of the process.
     * (See [`getuid(2)`](http://man7.org/linux/man-pages/man2/getuid.2.html).)
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getuid) {
     *   console.log(`Current uid: ${process.getuid()}`);
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * @since v0.1.28
     */
    getuid?: () => number;
    /**
     * The `process.setuid(id)` method sets the user identity of the process. (See [`setuid(2)`](http://man7.org/linux/man-pages/man2/setuid.2.html).) The `id` can be passed as either a
     * numeric ID or a username string.
     * If a username is specified, the method blocks while resolving the associated
     * numeric ID.
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getuid &#x26;&#x26; process.setuid) {
     *   console.log(`Current uid: ${process.getuid()}`);
     *   try {
     *     process.setuid(501);
     *     console.log(`New uid: ${process.getuid()}`);
     *   } catch (err) {
     *     console.log(`Failed to set uid: ${err}`);
     *   }
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * This feature is not available in `Worker` threads.
     * @since v0.1.28
     */
    setuid?: (id: number) => void;
    /**
     * The `process.geteuid()` method returns the numerical effective user identity of
     * the process. (See [`geteuid(2)`](http://man7.org/linux/man-pages/man2/geteuid.2.html).)
     *
     * ```js
     * import process from 'process';
     *
     * if (process.geteuid) {
     *   console.log(`Current uid: ${process.geteuid()}`);
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * @since v2.0.0
     */
    geteuid?: () => number;
    /**
     * The `process.seteuid()` method sets the effective user identity of the process.
     * (See [`seteuid(2)`](http://man7.org/linux/man-pages/man2/seteuid.2.html).) The `id` can be passed as either a numeric ID or a username
     * string. If a username is specified, the method blocks while resolving the
     * associated numeric ID.
     *
     * ```js
     * import process from 'process';
     *
     * if (process.geteuid &#x26;&#x26; process.seteuid) {
     *   console.log(`Current uid: ${process.geteuid()}`);
     *   try {
     *     process.seteuid(501);
     *     console.log(`New uid: ${process.geteuid()}`);
     *   } catch (err) {
     *     console.log(`Failed to set uid: ${err}`);
     *   }
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * This feature is not available in `Worker` threads.
     * @since v2.0.0
     * @param id A user name or ID
     */
    seteuid?: (id: number) => void;
    /**
     * The `process.getegid()` method returns the numerical effective group identity
     * of the Node.js process. (See [`getegid(2)`](http://man7.org/linux/man-pages/man2/getegid.2.html).)
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getegid) {
     *   console.log(`Current gid: ${process.getegid()}`);
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * @since v2.0.0
     */
    getegid?: () => number;
    /**
     * The `process.setegid()` method sets the effective group identity of the process.
     * (See [`setegid(2)`](http://man7.org/linux/man-pages/man2/setegid.2.html).) The `id` can be passed as either a numeric ID or a group
     * name string. If a group name is specified, this method blocks while resolving
     * the associated a numeric ID.
     *
     * ```js
     * import process from 'process';
     *
     * if (process.getegid &#x26;&#x26; process.setegid) {
     *   console.log(`Current gid: ${process.getegid()}`);
     *   try {
     *     process.setegid(501);
     *     console.log(`New gid: ${process.getegid()}`);
     *   } catch (err) {
     *     console.log(`Failed to set gid: ${err}`);
     *   }
     * }
     * ```
     *
     * This function is only available on POSIX platforms (i.e. not Windows or
     * Android).
     * This feature is not available in `Worker` threads.
     * @since v2.0.0
     * @param id A group name or ID
     */
    setegid?: (id: number) => void;
    /**
     * The `process.env` property returns an object containing the user environment.
     * See [`environ(7)`](http://man7.org/linux/man-pages/man7/environ.7.html).
     *
     * An example of this object looks like:
     *
     * ```js
     * {
     *   TERM: 'xterm-256color',
     *   SHELL: '/usr/local/bin/bash',
     *   USER: 'maciej',
     *   PATH: '~/.bin/:/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin',
     *   PWD: '/Users/maciej',
     *   EDITOR: 'vim',
     *   SHLVL: '1',
     *   HOME: '/Users/maciej',
     *   LOGNAME: 'maciej',
     *   _: '/usr/local/bin/node'
     * }
     * ```
     *
     * It is possible to modify this object, but such modifications will not be
     * reflected outside the Node.js process, or (unless explicitly requested)
     * to other `Worker` threads.
     * In other words, the following example would not work:
     *
     * ```bash
     * node -e 'process.env.foo = "bar"' &#x26;&#x26; echo $foo
     * ```
     *
     * While the following will:
     *
     * ```js
     * import { env } from 'node:process';
     *
     * env.foo = 'bar';
     * console.log(env.foo);
     * ```
     *
     * Assigning a property on `process.env` will implicitly convert the value
     * to a string. **This behavior is deprecated.** Future versions of Node.js may
     * throw an error when the value is not a string, number, or boolean.
     *
     * ```js
     * import { env } from 'node:process';
     *
     * env.test = null;
     * console.log(env.test);
     * // => 'null'
     * env.test = undefined;
     * console.log(env.test);
     * // => 'undefined'
     * ```
     *
     * Use `delete` to delete a property from `process.env`.
     *
     * ```js
     * import { env } from 'node:process';
     *
     * env.TEST = 1;
     * delete env.TEST;
     * console.log(env.TEST);
     * // => undefined
     * ```
     *
     * On Windows operating systems, environment variables are case-insensitive.
     *
     * ```js
     * import { env } from 'node:process';
     *
     * env.TEST = 1;
     * console.log(env.test);
     * // => 1
     * ```
     *
     * Unless explicitly specified when creating a `Worker` instance,
     * each `Worker` thread has its own copy of `process.env`, based on its
     * parent thread's `process.env`, or whatever was specified as the `env` option
     * to the `Worker` constructor. Changes to `process.env` will not be visible
     * across `Worker` threads, and only the main thread can make changes that
     * are visible to the operating system or to native add-ons. On Windows, a copy of `process.env` on a `Worker` instance operates in a case-sensitive manner
     * unlike the main thread.
     * @since v0.1.27
     */
    env: ProcessEnv;
  }
}
