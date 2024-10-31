declare module "os" {
  interface CpuInfo {
    model: string;
    speed: number;
    /** The number of milliseconds spent by the CPU in each mode cannot be obtained at this time. */
    times: {
      /** The number of milliseconds the CPU has spent in user mode. */
      user: number;
      /** The number of milliseconds the CPU has spent in nice mode. */
      nice: number;
      /** The number of milliseconds the CPU has spent in sys mode. */
      sys: number;
      /** The number of milliseconds the CPU has spent in idle mode. */
      idle: number;
      /** The number of milliseconds the CPU has spent in irq mode. */
      irq: number;
    };
  }
  interface NetworkInterfaceBase {
    address: string;
    netmask: string;
    mac: string;
    internal: boolean;
    cidr: string | null;
  }
  interface NetworkInterfaceInfoIPv4 extends NetworkInterfaceBase {
    family: "IPv4";
    scopeid?: undefined;
  }
  interface NetworkInterfaceInfoIPv6 extends NetworkInterfaceBase {
    family: "IPv6";
    scopeid: number;
  }
  interface UserInfo<T> {
    username: T;
    uid: number;
    gid: number;
    shell: T | null;
    homedir: T;
  }
  type NetworkInterfaceInfo =
    | NetworkInterfaceInfoIPv4
    | NetworkInterfaceInfoIPv6;
  /**
   * Returns an estimate of the default amount of parallelism a program should use.
   * Always returns a value greater than zero.
   */
  function availableParallelism(): number;
  /**
   * Returns the operating system CPU architecture for which the LLRT binary was compiled.
   * Possible values are 'arm64', 'x64'. The return value is equivalent to `process.arch`.
   */
  function arch(): string;
  /**
   * Returns an array of objects containing information about each logical CPU core.
   * The array will be empty if no CPU information is available, such as if the `/proc` file system is unavailable.
   *
   * The properties included on each object include:
   *
   * ```js
   * [
   *   {
   *     model: 'Intel(R) Core(TM) i7 CPU         860  @ 2.80GHz',
   *     speed: 2926,
   *     times: {
   *       user: 252020,
   *       nice: 0,
   *       sys: 30340,
   *       idle: 1070356870,
   *       irq: 0,
   *     },
   *   },
   *   {
   *     model: 'Intel(R) Core(TM) i7 CPU         860  @ 2.80GHz',
   *     speed: 2926,
   *     times: {
   *       user: 306960,
   *       nice: 0,
   *       sys: 26980,
   *       idle: 1071569080,
   *       irq: 0,
   *     },
   *   },
   *   {
   *     model: 'Intel(R) Core(TM) i7 CPU         860  @ 2.80GHz',
   *     speed: 2926,
   *     times: {
   *       user: 248450,
   *       nice: 0,
   *       sys: 21750,
   *       idle: 1070919370,
   *       irq: 0,
   *     },
   *   },
   *   {
   *     model: 'Intel(R) Core(TM) i7 CPU         860  @ 2.80GHz',
   *     speed: 2926,
   *     times: {
   *       user: 256880,
   *       nice: 0,
   *       sys: 19430,
   *       idle: 1070905480,
   *       irq: 20,
   *     },
   *   },
   * ]
   * ```
   *
   * `nice` values are POSIX-only. On Windows, the `nice` values of all processors
   * are always 0.
   *
   * `os.cpus().length` should not be used to calculate the amount of parallelism
   * available to an application. Use {@link availableParallelism} for this purpose.
   */
  function cpus(): CpuInfo[];
  const devNull: string;
  /**
   * Returns a string identifying the endianness of the CPU for which the Node.js
   * binary was compiled.
   *
   * Possible values are `'BE'` for big endian and `'LE'` for little endian.
   */
  function endianness(): "BE" | "LE";
  /**
   * The operating system-specific end-of-line marker.
   * * `\n` on POSIX
   * * `\r\n` on Windows
   */
  const EOL: string;
  /**
   * Returns the amount of free system memory in bytes as an integer.
   */
  function freemem(): number;
  /**
   * Returns the scheduling priority for the process specified by `pid`. If `pid` is
   * not provided or is `0`, the priority of the current process is returned.
   * @param [pid=0] The process ID to retrieve scheduling priority for.
   */
  function getPriority(pid?: number): number;
  /**
   * Returns the string path of the current user's home directory.
   *
   * On POSIX, it uses the `$HOME` environment variable if defined. Otherwise it
   * uses the [effective UID](https://en.wikipedia.org/wiki/User_identifier#Effective_user_ID) to look up the user's home directory.
   *
   * On Windows, it uses the `USERPROFILE` environment variable if defined.
   * Otherwise it uses the path to the profile directory of the current user.
   */
  function homedir(): string;
  /**
   * Returns the host name of the operating system as a string.
   */
  function hostname(): string;
  /**
   * Returns an array containing the 1, 5, and 15 minute load averages.
   *
   * The load average is a measure of system activity calculated by the operating
   * system and expressed as a fractional number.
   *
   * The load average is a Unix-specific concept. On Windows, the return value is
   * always `[0, 0, 0]`.
   */
  function loadavg(): number[];
  /**
   * Returns the machine type as a string, such as `arm64`, `aarch64`, `x86_64`.
   *
   * On POSIX systems, the machine type is determined by calling [`uname(3)`](https://linux.die.net/man/3/uname). On Windows, `RtlGetVersion()` is used, and if it is not
   * available, `GetVersionExW()` will be used. See [https://en.wikipedia.org/wiki/Uname#Examples](https://en.wikipedia.org/wiki/Uname#Examples) for more information.
   */
  function machine(): string;
  /**
   * Returns an object containing network interfaces that have been assigned a
   * network address.
   *
   * Each key on the returned object identifies a network interface. The associated
   * value is an array of objects that each describe an assigned network address.
   *
   * The properties available on the assigned network address object include:
   *
   * ```js
   * {
   *   lo: [
   *     {
   *       address: '127.0.0.1',
   *       netmask: '255.0.0.0',
   *       family: 'IPv4',
   *       mac: '00:00:00:00:00:00',
   *       internal: true,
   *       cidr: '127.0.0.1/8'
   *     },
   *     {
   *       address: '::1',
   *       netmask: 'ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff',
   *       family: 'IPv6',
   *       mac: '00:00:00:00:00:00',
   *       scopeid: 0,
   *       internal: true,
   *       cidr: '::1/128'
   *     }
   *   ],
   *   eth0: [
   *     {
   *       address: '192.168.1.108',
   *       netmask: '255.255.255.0',
   *       family: 'IPv4',
   *       mac: '01:02:03:0a:0b:0c',
   *       internal: false,
   *       cidr: '192.168.1.108/24'
   *     },
   *     {
   *       address: 'fe80::a00:27ff:fe4e:66a1',
   *       netmask: 'ffff:ffff:ffff:ffff::',
   *       family: 'IPv6',
   *       mac: '01:02:03:0a:0b:0c',
   *       scopeid: 1,
   *       internal: false,
   *       cidr: 'fe80::a00:27ff:fe4e:66a1/64'
   *     }
   *   ]
   * }
   * ```
   */
  function networkInterfaces(): NodeJS.Dict<NetworkInterfaceInfo[]>;
  /**
   * Returns a string identifying the operating system platform for which
   * the Node.js binary was compiled. The value is set at compile time.
   */
  function platform(): Platform;
  function release(): string;
  /**
   * Returns the operating system's default directory for temporary files as a
   * string.
   */
  /**
   * Attempts to set the scheduling priority for the process specified by `pid`. If `pid` is not provided or is `0`, the process ID of the current process is used.
   *
   * The `priority` input must be an integer between `-20` (high priority) and `19` (low priority). Due to differences between Unix priority levels and Windows
   * priority classes, `priority` is mapped to one of six priority constants in `os.constants.priority`. When retrieving a process priority level, this range
   * mapping may cause the return value to be slightly different on Windows. To avoid
   * confusion, set `priority` to one of the priority constants.
   *
   * On Windows, not currently supported on Windows.
   *
   * @param [pid=0] The process ID to set scheduling priority for.
   * @param priority The scheduling priority to assign to the process.
   */
  function setPriority(priority: number): void;
  function setPriority(pid: number, priority: number): void;
  /**
   * Returns the operating system release as a string.
   *
   * On POSIX systems, the operating system release is determined by calling [`uname(3)`](https://linux.die.net/man/3/uname). On Windows, `RtlGetVersion()` is used. See
   * [https://en.wikipedia.org/wiki/Uname#Examples](https://en.wikipedia.org/wiki/Uname#Examples) for more information.
   */
  function tmpdir(): string;
  /**
   * Returns the total amount of system memory in bytes as an integer.
   */
  function totalmem(): number;
  /**
   * Returns the operating system name as returned by [`uname(3)`](https://linux.die.net/man/3/uname). For example, it
   * returns `'Linux'` on Linux, `'Darwin'` on macOS, and `'Windows_NT'` on Windows.
   *
   * See [https://en.wikipedia.org/wiki/Uname#Examples](https://en.wikipedia.org/wiki/Uname#Examples) for additional information
   * about the output of running [`uname(3)`](https://linux.die.net/man/3/uname) on various operating systems.
   */
  function type(): string;
  /**
   * Returns the system uptime in number of seconds.
   */
  function uptime(): number;
  /**
   * Returns a string identifying the kernel version.
   *
   * On POSIX systems, the operating system release is determined by calling [`uname(3)`](https://linux.die.net/man/3/uname). On Windows, `RtlGetVersion()` is used.
   * See [https://en.wikipedia.org/wiki/Uname#Examples](https://en.wikipedia.org/wiki/Uname#Examples) for more information.
   */
  /**
   * Returns information about the currently effective user. On POSIX platforms,
   * this is typically a subset of the password file. The returned object includes
   * the `username`, `uid`, `gid`, `shell`, and `homedir`. On Windows, the `uid` and `gid` fields are `-1`, and `shell` is `null`.
   *
   * The value of `homedir` returned by `os.userInfo()` is provided by the operating
   * system. This differs from the result of `os.homedir()`, which queries
   * environment variables for the home directory before falling back to the
   * operating system response.
   *
   * Throws a [`SystemError`](https://nodejs.org/docs/latest-v22.x/api/errors.html#class-systemerror) if a user has no `username` or `homedir`.
   */
  //function userInfo(options: { encoding: "buffer" }): UserInfo<Buffer>;
  function userInfo(options?: { encoding: BufferEncoding }): UserInfo<string>;
  function version(): string;
}
