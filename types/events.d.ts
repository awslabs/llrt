declare module "events" {
  type EventMap<T> = Record<keyof T, any[]> | DefaultEventMap;
  type DefaultEventMap = [never];
  type AnyRest = [...args: any[]];
  type Args<K, T> = T extends DefaultEventMap
    ? AnyRest
    : K extends keyof T
      ? T[K]
      : never;
  type Key<K, T> = T extends DefaultEventMap ? EventKey : K | keyof T;
  type Key2<K, T> = T extends DefaultEventMap ? EventKey : K & keyof T;
  type Listener<K, T, F> = T extends DefaultEventMap
    ? F
    : K extends keyof T
      ? T[K] extends unknown[]
        ? (...args: T[K]) => void
        : never
      : never;
  type Listener1<K, T> = Listener<K, T, (...args: any[]) => void>;

  export class EventEmitter<T extends EventMap<T> = DefaultEventMap> {
    constructor();

    /**
     * Alias for `emitter.on(eventName, listener)`.
     */
    addListener<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Adds the `listener` function to the end of the listeners array for the event
     * named `eventName`. No checks are made to see if the `listener` has already
     * been added. Multiple calls passing the same combination of `eventName` and
     * `listener` will result in the `listener` being added, and called, multiple times.
     *
     * ```js
     * server.on('connection', (stream) => {
     *   console.log('someone connected!');
     * });
     * ```
     *
     * Returns a reference to the `EventEmitter`, so that calls can be chained.
     *
     * By default, event listeners are invoked in the order they are added. The `emitter.prependListener()` method can be used as an alternative to add the
     * event listener to the beginning of the listeners array.
     *
     * ```js
     * import { EventEmitter } from 'events';
     * const myEE = new EventEmitter();
     * myEE.on('foo', () => console.log('a'));
     * myEE.prependListener('foo', () => console.log('b'));
     * myEE.emit('foo');
     * // Prints:
     * //   b
     * //   a
     * ```
     * @param eventName The name of the event.
     * @param listener The callback function
     */
    on<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Adds a **one-time** `listener` function for the event named `eventName`. The
     * next time `eventName` is triggered, this listener is removed and then invoked.
     *
     * Returns a reference to the `EventEmitter`, so that calls can be chained.
     *
     * By default, event listeners are invoked in the order they are added. The `emitter.prependOnceListener()` method can be used as an alternative to add the
     * event listener to the beginning of the listeners array.
     *
     * ```js
     * import { EventEmitter } from 'events';
     * const myEE = new EventEmitter();
     * myEE.once('foo', () => console.log('a'));
     * myEE.prependOnceListener('foo', () => console.log('b'));
     * myEE.emit('foo');
     * // Prints:
     * //   b
     * //   a
     * ```
     * @since v0.3.0
     * @param eventName The name of the event.
     * @param listener The callback function
     */
    once<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Removes the specified `listener` from the listener array for the event named `eventName`.
     *
     * `removeListener()` will remove, at most, one instance of a listener from the
     * listener array. If any single listener has been added multiple times to the
     * listener array for the specified `eventName`, then `removeListener()` must be
     * called multiple times to remove each instance.
     *
     * Once an event is emitted, all listeners attached to it at the time of emitting are called in order.
     * This implies that any `removeListener()` calls _after_ emitting and _before_ the last listener finishes execution
     * will not remove them from `emit()` in progress. Subsequent events behave as expected.
     *
     * ```js
     * import { EventEmitter } from 'events';
     * class MyEmitter extends EventEmitter {}
     * const myEmitter = new MyEmitter();
     *
     * const callbackA = () => {
     *   console.log('A');
     *   myEmitter.removeListener('event', callbackB);
     * };
     *
     * const callbackB = () => {
     *   console.log('B');
     * };
     *
     * myEmitter.on('event', callbackA);
     *
     * myEmitter.on('event', callbackB);
     *
     * // callbackA removes listener callbackB but it will still be called.
     * // Internal listener array at time of emit [callbackA, callbackB]
     * myEmitter.emit('event');
     * // Prints:
     * //   A
     * //   B
     *
     * // callbackB is now removed.
     * // Internal listener array [callbackA]
     * myEmitter.emit('event');
     * // Prints:
     * //   A
     * ```
     *
     * Because listeners are managed using an internal array, calling this will
     * change the position indices of any listener registered _after_ the listener
     * being removed. This will not impact the order in which listeners are called,
     * but it means that any copies of the listener array as returned by
     * the `emitter.listeners()` method will need to be recreated.
     *
     * When a single function has been added as a handler multiple times for a single
     * event (as in the example below), `removeListener()` will remove the most
     * recently added instance. In the example the `once('ping')` listener is removed:
     *
     * ```js
     * import { EventEmitter } from 'events';
     * const ee = new EventEmitter();
     *
     * function pong() {
     *   console.log('pong');
     * }
     *
     * ee.on('ping', pong);
     * ee.once('ping', pong);
     * ee.removeListener('ping', pong);
     *
     * ee.emit('ping');
     * ee.emit('ping');
     * ```
     *
     * Returns a reference to the `EventEmitter`, so that calls can be chained.
     */
    removeListener<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Alias for `emitter.removeListener()`.
     */
    off<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Synchronously calls each of the listeners registered for the event named `eventName`, in the order they were registered, passing the supplied arguments
     * to each.
     *
     * ```js
     * import { EventEmitter } from 'events';
     * const myEmitter = new EventEmitter();
     *
     * // First listener
     * myEmitter.on('event', function firstListener() {
     *   console.log('Helloooo! first listener');
     * });
     * // Second listener
     * myEmitter.on('event', function secondListener(arg1, arg2) {
     *   console.log(`event with parameters ${arg1}, ${arg2} in second listener`);
     * });
     * // Third listener
     * myEmitter.on('event', function thirdListener(...args) {
     *   const parameters = args.join(', ');
     *   console.log(`event with parameters ${parameters} in third listener`);
     * });
     *
     * myEmitter.emit('event', 1, 2, 3, 4, 5);
     *
     * // Prints:
     * // Helloooo! first listener
     * // event with parameters 1, 2 in second listener
     * // event with parameters 1, 2, 3, 4, 5 in third listener
     * ```
     */
    emit<K>(eventName: Key<K, T>, ...args: Args<K, T>): void;

    /**
     * Adds the `listener` function to the _beginning_ of the listeners array for the
     * event named `eventName`. No checks are made to see if the `listener` has
     * already been added. Multiple calls passing the same combination of `eventName`
     * and `listener` will result in the `listener` being added, and called, multiple times.
     *
     *
     * Returns a reference to the `EventEmitter`, so that calls can be chained.
     * @param eventName The name of the event.
     * @param listener The callback function
     */
    prependListener<K>(eventName: Key<K, T>, listener: Listener1<K, T>): this;

    /**
     * Adds a **one-time**`listener` function for the event named `eventName` to the _beginning_ of the listeners array.
     * The next time `eventName` is triggered, this listener is removed, and then invoked.
     *
     * Returns a reference to the `EventEmitter`, so that calls can be chained.
     * @param eventName The name of the event.
     * @param listener The callback function
     */
    prependOnceListener<K>(
      eventName: Key<K, T>,
      listener: Listener1<K, T>
    ): this;

    /**
     * Returns an array listing the events for which the emitter has registered
     * listeners. The values in the array are strings or `Symbol`s.
     *
     * ```js
     * import { EventEmitter } from 'events';
     *
     * const myEE = new EventEmitter();
     * myEE.on('foo', () => {});
     * myEE.on('bar', () => {});
     *
     * const sym = Symbol('symbol');
     * myEE.on(sym, () => {});
     *
     * console.log(myEE.eventNames());
     * // Prints: [ 'foo', 'bar', Symbol(symbol) ]
     * ```
     */
    eventNames(): Array<EventKey & Key2<unknown, T>>;
  }

  export default EventEmitter;
}
