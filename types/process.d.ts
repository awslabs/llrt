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

    type Platform =
      | "aix"
      | "android"
      | "darwin"
      | "freebsd"
      | "haiku"
      | "linux"
      | "openbsd"
      | "sunos"
      | "win32"
      | "cygwin"
      | "netbsd";

    type Architecture =
      | "arm"
      | "arm64"
      | "ia32"
      | "loong64"
      | "mips"
      | "mipsel"
      | "ppc"
      | "ppc64"
      | "riscv64"
      | "s390"
      | "s390x"
      | "x64";
  }

  interface Dict<T> {
    [key: string]: T | undefined;
  }

  // Alias for compatibility
  interface ProcessEnv extends Dict<string> {}
  interface HRTime {
    (time?: [number, number]): [number, number];
  }
  interface ProcessRelease {
    name: string;
    sourceUrl?: string | undefined;
    headersUrl?: string | undefined;
    libUrl?: string | undefined;
    lts?: string | undefined;
  }
  interface ProcessVersions extends Dict<string> {
    http_parser: string;
    node: string;
    v8: string;
    ares: string;
    uv: string;
    zlib: string;
    modules: string;
    openssl: string;
  }
  interface Process extends EventEmitter {
    hrtime: HRTime;

    /**
     * The `process.cwd()` method returns the current working directory of the Node.js
     * process.
     *
     * ```js
     * import { cwd } from 'node:process';
     *
     * console.log(`Current directory: ${cwd()}`);
     * ```
     */
    cwd(): string;
    /**
     * The `process.argv0` property stores a read-only copy of the original value of`argv[0]` passed when Node.js starts.
     *
     * ```console
     * $ bash -c 'exec -a customArgv0 ./node'
     * > process.argv[0]
     * '/Volumes/code/external/node/out/Release/node'
     * > process.argv0
     * 'customArgv0'
     * ```
     */
    argv0: string;
    /**
     * Returns the OS-assigned process identifier associated with this process.
     */
    id: number;
    /**
     * The `process.argv` property returns an array containing the command-line
     * arguments passed when the Node.js process was launched. The first element will
     * be {@link execPath}. See `process.argv0` if access to the original value
     * of `argv[0]` is needed. The second element will be the path to the JavaScript
     * file being executed. The remaining elements will be any additional command-line
     * arguments.
     *
     * For example, assuming the following script for `process-args.js`:
     *
     * ```js
     * import { argv } from 'node:process';
     *
     * // print process.argv
     * argv.forEach((val, index) => {
     *   console.log(`${index}: ${val}`);
     * });
     * ```
     *
     * Launching the Node.js process as:
     *
     * ```bash
     * node process-args.js one two=three four
     * ```
     *
     * Would generate the output:
     *
     * ```text
     * 0: /usr/local/bin/node
     * 1: /Users/mjr/work/node/process-args.js
     * 2: one
     * 3: two=three
     * 4: four
     * ```
     */
    argv: string[];
    /**
     * The operating system CPU architecture for which the Node.js binary was compiled.
     * Possible values are: `'arm'`, `'arm64'`, `'ia32'`, `'loong64'`, `'mips'`, `'mipsel'`, `'ppc'`, `'ppc64'`, `'riscv64'`, `'s390'`, `'s390x'`, and `'x64'`.
     *
     * ```js
     * import { arch } from 'node:process';
     *
     * console.log(`This processor architecture is ${arch}`);
     * ```
     * @since v0.5.0
     */
    readonly arch: Architecture;
    /**
     * The `process.platform` property returns a string identifying the operating
     * system platform for which the Node.js binary was compiled.
     *
     * Currently possible values are:
     *
     * * `'aix'`
     * * `'darwin'`
     * * `'freebsd'`
     * * `'linux'`
     * * `'openbsd'`
     * * `'sunos'`
     * * `'win32'`
     *
     * ```js
     * import { platform } from 'node:process';
     *
     * console.log(`This platform is ${platform}`);
     * ```
     *
     * The value `'android'` may also be returned if the Node.js is built on the
     * Android operating system. However, Android support in Node.js [is experimental](https://github.com/nodejs/node/blob/HEAD/BUILDING.md#androidandroid-based-devices-eg-firefox-os).
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
     *   name: 'node',
     *   lts: 'Hydrogen',
     *   sourceUrl: 'https://nodejs.org/download/release/v18.12.0/node-v18.12.0.tar.gz',
     *   headersUrl: 'https://nodejs.org/download/release/v18.12.0/node-v18.12.0-headers.tar.gz',
     *   libUrl: 'https://nodejs.org/download/release/v18.12.0/win-x64/node.lib'
     * }
     * ```
     *
     * In custom builds from non-release versions of the source tree, only the `name` property may be present. The additional properties should not be
     * relied upon to exist.
     */
    readonly release: ProcessRelease;
    /**
     * The `process.version` property contains the Node.js version string.
     *
     * ```js
     * import { version } from 'node:process';
     *
     * console.log(`Version: ${version}`);
     * // Version: v14.8.0
     * ```
     *
     * To get the version string without the prepended _v_, use`process.versions.node`.
     */
    readonly version: string;
    /**
     * The `process.versions` property returns an object listing the version strings of
     * Node.js and its dependencies. `process.versions.modules` indicates the current
     * ABI version, which is increased whenever a C++ API changes. Node.js will refuse
     * to load modules that were compiled against a different module ABI version.
     *
     * ```js
     * import { versions } from 'node:process';
     *
     * console.log(versions);
     * ```
     *
     * Will generate an object similar to:
     *
     * ```console
     * { node: '20.2.0',
     *   acorn: '8.8.2',
     *   ada: '2.4.0',
     *   ares: '1.19.0',
     *   base64: '0.5.0',
     *   brotli: '1.0.9',
     *   cjs_module_lexer: '1.2.2',
     *   cldr: '43.0',
     *   icu: '73.1',
     *   llhttp: '8.1.0',
     *   modules: '115',
     *   napi: '8',
     *   nghttp2: '1.52.0',
     *   nghttp3: '0.7.0',
     *   ngtcp2: '0.8.1',
     *   openssl: '3.0.8+quic',
     *   simdutf: '3.2.9',
     *   tz: '2023c',
     *   undici: '5.22.0',
     *   unicode: '15.0',
     *   uv: '1.44.2',
     *   uvwasi: '0.0.16',
     *   v8: '11.3.244.8-node.9',
     *   zlib: '1.2.13' }
     * ```
     */
    readonly versions: ProcessVersions;
    /**
     * The `process.exit()` method instructs Node.js to terminate the process
     * synchronously with an exit status of `code`. If `code` is omitted, exit uses
     * either the 'success' code `0` or the value of `process.exitCode` if it has been
     * set. Node.js will not terminate until all the `'exit'` event listeners are
     * called.
     *
     * To exit with a 'failure' code:
     *
     * ```js
     * import { exit } from 'node:process';
     *
     * exit(1);
     * ```
     *
     * The shell that executed Node.js should see the exit code as `1`.
     *
     * Calling `process.exit()` will force the process to exit as quickly as possible
     * even if there are still asynchronous operations pending that have not yet
     * completed fully, including I/O operations to `process.stdout` and `process.stderr`.
     *
     * In most situations, it is not actually necessary to call `process.exit()` explicitly. The Node.js process will exit on its own _if there is no additional_
     * _work pending_ in the event loop. The `process.exitCode` property can be set to
     * tell the process which exit code to use when the process exits gracefully.
     *
     * For instance, the following example illustrates a _misuse_ of the `process.exit()` method that could lead to data printed to stdout being
     * truncated and lost:
     *
     * ```js
     * import { exit } from 'node:process';
     *
     * // This is an example of what *not* to do:
     * if (someConditionNotMet()) {
     *   printUsageToStdout();
     *   exit(1);
     * }
     * ```
     *
     * The reason this is problematic is because writes to `process.stdout` in Node.js
     * are sometimes _asynchronous_ and may occur over multiple ticks of the Node.js
     * event loop. Calling `process.exit()`, however, forces the process to exit _before_ those additional writes to `stdout` can be performed.
     *
     * Rather than calling `process.exit()` directly, the code _should_ set the `process.exitCode` and allow the process to exit naturally by avoiding
     * scheduling any additional work for the event loop:
     *
     * ```js
     * import process from 'node:process';
     *
     * // How to properly set the exit code while letting
     * // the process exit gracefully.
     * if (someConditionNotMet()) {
     *   printUsageToStdout();
     *   process.exitCode = 1;
     * }
     * ```
     *
     * If it is necessary to terminate the Node.js process due to an error condition,
     * throwing an _uncaught_ error and allowing the process to terminate accordingly
     * is safer than calling `process.exit()`.
     *
     * In `Worker` threads, this function stops the current thread rather
     * than the current process.
     * @since v0.1.13
     * @param [code=0] The exit code. For string type, only integer strings (e.g.,'1') are allowed.
     */
    exit(code?: number | string | null | undefined): never;
  }
}
