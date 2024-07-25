export {};

interface EventListener {
  (evt: Event): void;
}

interface AddEventListenerOptions {
  /** When `true`, the listener is automatically removed when it is first invoked. Default: `false`. */
  once?: boolean;
}

declare global {
  type EventKey = string | symbol;

  /** An event which takes place in the system. */
  interface Event {
    /** Returns the type of event, e.g. "click", "hashchange", or "submit". */
    readonly type: EventKey;
  }

  class CustomEvent<D = any> implements Event {
    constructor(type: string, opts?: { details?: D });
    readonly type: string;
    readonly details: D | null;
  }

  /**
   * EventTarget is an interface implemented by objects that can
   * receive events and may have listeners for them.
   */
  class EventTarget {
    constructor();

    /**
     * Adds a new handler for the `type` event. Any given `listener` is added only once per `type`.
     *
     * If the `once` option is true, the `listener` is removed after the next time a `type` event is dispatched.
     */
    addEventListener(
      type: EventKey,
      listener: EventListener,
      options?: AddEventListenerOptions
    ): void;

    /** Dispatches a synthetic event event to target */
    dispatchEvent(event: Event): void;

    /** Removes the event listener in target's event listener list with the same type and callback */
    removeEventListener(type: EventKey, listener: EventListener): void;
  }
}
