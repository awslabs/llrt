export {};

declare global {
  class AbortController {
    /**
     * Creates a new `AbortController` object instance.
     */
    constructor();

    /**
     * Returns the AbortSignal object associated with this object.
     */
    readonly signal: AbortSignal;

    /**
     * Invoking this method will set this object's AbortSignal's aborted flag and signal to any observers that the associated activity is to be aborted.
     */
    abort(reason?: any): void;
  }

  /** A signal object that allows you to communicate with a DOM request (such as a Fetch) and abort it if required via an AbortController object. */
  class AbortSignal extends EventTarget {
    /**
     * Creates a new `AbortSignal` object instance.
     */
    constructor();

    /**
     * Returns true if this AbortSignal's AbortController has signaled to abort, and false otherwise.
     */
    readonly aborted: boolean;

    /**
     * A JavaScript value providing the abort reason, once the signal has aborted.
     */
    readonly reason: any;

    /**
     * Registers an event listener callback to execute when an `abort` event is observed.
     */
    onabort: null | ((this: AbortSignal, event: Event) => any);

    /**
     * Throws the signal's abort reason if the signal has been aborted; otherwise it does nothing.
     */
    throwIfAborted(): void;

    /**
     * Returns an `AbortSignal` instance that is already set as aborted.
     *
     * @param reason The reason for the abort.
     */
    static abort(reason?: any): AbortSignal;

    /**
     * Returns an `AbortSignal` instance that will automatically abort after a specified time.
     *
     * @param milliseconds The number of milliseconds to wait before aborting.
     */
    static timeout(milliseconds: number): AbortSignal;

    /**
     * Returns an `AbortSignal` that aborts when any of the given abort signals abort.
     *
     * @param signals An array of `AbortSignal` objects to observe.
     */
    static any(signals: AbortSignal[]): AbortSignal;
  }
}
