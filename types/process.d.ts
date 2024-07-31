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
    (time?: [number, number]): [number, number];
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
  }
}
