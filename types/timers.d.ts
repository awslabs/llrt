declare module "timers" {
  export import setTimeout = globalThis.setTimeout;
  export import clearTimeout = globalThis.clearTimeout;
  export import setInterval = globalThis.setInterval;
  export import clearInterval = globalThis.clearInterval;
  export import setImmediate = globalThis.setImmediate;

  global {
    /**
     * This object is created internally and is returned from `setTimeout()` and `setInterval()`. It can be passed to either `clearTimeout()` or `clearInterval()` in order to cancel the
     * scheduled actions.
     */
    class Timeout {}

    /**
     * Schedules execution of a one-time `callback` after `delay` milliseconds.
     *
     * The `callback` will likely not be invoked in precisely `delay` milliseconds.
     * LLRT makes no guarantees about the exact timing of when callbacks will fire,
     * nor of their ordering. The callback will be called as close as possible to the
     * time specified. The precision is limited to 4ms.
     *
     * @param callback The function to call when the timer elapses.
     * @param [delay=4] The number of milliseconds to wait before calling the `callback`.
     * @return for use with {@link clearTimeout}
     */
    function setTimeout<TArgs extends any[]>(
      callback: (...args: TArgs) => void,
      ms?: number
    ): Timeout;

    /**
     * Cancels a `Timeout` object created by `setTimeout()`.
     * @param timeout A `Timeout` object as returned by {@link setTimeout}.
     */
    function clearTimeout(timeout: Timeout): void;

    /**
     * Schedules repeated execution of `callback` every `delay` milliseconds.
     *
     * The `callback` will likely not be invoked at precisely `delay` milliseconds.
     * LLRT makes no guarantees about the exact timing of when callbacks will fire,
     * nor of their ordering. The callback will be called as close as possible to the
     * time specified. The precision is limited to 4ms.
     *
     * @param callback The function to call when the timer elapses.
     * @param [delay=4] The number of milliseconds to wait before calling the `callback`.
     * @return for use with {@link clearInterval}
     */
    function setInterval<TArgs extends any[]>(
      callback: (...args: TArgs) => void,
      ms?: number
    ): Timeout;

    /**
     * Cancels a `Timeout` object created by `setInterval()`.
     * @param timeout A `Timeout` object as returned by {@link setInterval}
     */
    function clearInterval(interval: Timeout): void;

    /**
     * Schedules the "immediate" execution of the `callback` after I/O events'
     * callbacks.
     *
     * @param callback The function to call at the end of this turn of the Node.js `Event Loop`
     * @return for use with {@link clearImmediate}
     */
    function setImmediate<TArgs extends any[]>(
      callback: (...args: TArgs) => void
    ): void;
  }
}
