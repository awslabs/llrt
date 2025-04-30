/**
 * We strongly discourage the use of the `async_hooks` API.
 * Other APIs that can cover most of its use cases include:
 *
 * * [`AsyncLocalStorage`](https://nodejs.org/docs/latest-v22.x/api/async_context.html#class-asynclocalstorage) tracks async context
 * * [`process.getActiveResourcesInfo()`](https://nodejs.org/docs/latest-v22.x/api/process.html#processgetactiveresourcesinfo) tracks active resources
 *
 * The `async_hooks` module provides an API to track asynchronous resources.
 * It can be accessed using:
 *
 * ```js
 * import async_hooks from 'async_hooks';
 * ```
 * @experimental
 * @see [source](https://github.com/nodejs/node/blob/v22.x/lib/async_hooks.js)
 */
declare module "async_hooks" {
  /**
   * ```js
   * import { executionAsyncId } from 'async_hooks';
   * import fs from 'fs';
   *
   * console.log(executionAsyncId());  // 1 - bootstrap
   * const path = '.';
   * fs.open(path, 'r', (err, fd) => {
   *   console.log(executionAsyncId());  // 6 - open()
   * });
   * ```
   *
   * The ID returned from `executionAsyncId()` is related to execution timing, not
   * causality (which is covered by `triggerAsyncId()`):
   *
   * ```js
   * const server = net.createServer((conn) => {
   *   // Returns the ID of the server, not of the new connection, because the
   *   // callback runs in the execution scope of the server's MakeCallback().
   *   async_hooks.executionAsyncId();
   *
   * }).listen(port, () => {
   *   // Returns the ID of a TickObject (process.nextTick()) because all
   *   // callbacks passed to .listen() are wrapped in a nextTick().
   *   async_hooks.executionAsyncId();
   * });
   * ```
   *
   * Promise contexts may not get precise `executionAsyncIds` by default.
   * See the section on [promise execution tracking](https://nodejs.org/docs/latest-v22.x/api/async_hooks.html#promise-execution-tracking).
   * @return The `asyncId` of the current execution context. Useful to track when something calls.
   */
  function executionAsyncId(): number;
  /**
   * Resource objects returned by `executionAsyncResource()` are most often internal
   * LLRT handle objects with undocumented APIs. Using any functions or properties
   * on the object is likely to crash your application and should be avoided.
   *
   * Using `executionAsyncResource()` in the top-level execution context will
   * return an empty object as there is no handle or request object to use,
   * but having an object representing the top-level can be helpful.
   *
   * ```js
   * import { open } from 'fs';
   * import { executionAsyncId, executionAsyncResource } from 'async_hooks';
   *
   * console.log(executionAsyncId(), executionAsyncResource());  // 1 {}
   * open(new URL(import.meta.url), 'r', (err, fd) => {
   *   console.log(executionAsyncId(), executionAsyncResource());  // 7 FSReqWrap
   * });
   * ```
   *
   * This can be used to implement continuation local storage without the
   * use of a tracking `Map` to store the metadata:
   *
   * ```js
   * import { createServer } from 'http';
   * import {
   *   executionAsyncId,
   *   executionAsyncResource,
   *   createHook,
   * } from 'async_hooks';
   * const sym = Symbol('state'); // Private symbol to avoid pollution
   *
   * createHook({
   *   init(asyncId, type, triggerAsyncId, resource) {
   *     const cr = executionAsyncResource();
   *     if (cr) {
   *       resource[sym] = cr[sym];
   *     }
   *   },
   * }).enable();
   *
   * const server = createServer((req, res) => {
   *   executionAsyncResource()[sym] = { state: req.url };
   *   setTimeout(function() {
   *     res.end(JSON.stringify(executionAsyncResource()[sym]));
   *   }, 100);
   * }).listen(3000);
   * ```
   * @return The resource representing the current execution. Useful to store data within the resource.
   */
  function executionAsyncResource(): object;
  /**
   * ```js
   * const server = net.createServer((conn) => {
   *   // The resource that caused (or triggered) this callback to be called
   *   // was that of the new connection. Thus the return value of triggerAsyncId()
   *   // is the asyncId of "conn".
   *   async_hooks.triggerAsyncId();
   *
   * }).listen(port, () => {
   *   // Even though all callbacks passed to .listen() are wrapped in a nextTick()
   *   // the callback itself exists because the call to the server's .listen()
   *   // was made. So the return value would be the ID of the server.
   *   async_hooks.triggerAsyncId();
   * });
   * ```
   *
   * Promise contexts may not get valid `triggerAsyncId`s by default. See
   * the section on [promise execution tracking](https://nodejs.org/docs/latest-v22.x/api/async_hooks.html#promise-execution-tracking).
   * @return The ID of the resource responsible for calling the callback that is currently being executed.
   */
  function triggerAsyncId(): number;
  interface HookCallbacks {
    /**
     * Called when a class is constructed that has the possibility to emit an asynchronous event.
     * @param asyncId A unique ID for the async resource
     * @param type The type of the async resource
     * @param triggerAsyncId The unique ID of the async resource in whose execution context this async resource was created
     */
    init?(asyncId: number, type: string, triggerAsyncId: number): void;
    /**
     * When an asynchronous operation is initiated or completes a callback is called to notify the user.
     * The before callback is called just before said callback is executed.
     * @param asyncId the unique identifier assigned to the resource about to execute the callback.
     */
    before?(asyncId: number): void;
    /**
     * Called immediately after the callback specified in `before` is completed.
     *
     * If an uncaught exception occurs during execution of the callback, then `after` will run after the `'uncaughtException'` event is emitted or a `domain`'s handler runs.
     * @param asyncId the unique identifier assigned to the resource which has executed the callback.
     */
    after?(asyncId: number): void;
    /**
     * Called when a promise has resolve() called. This may not be in the same execution id
     * as the promise itself.
     * @param asyncId the unique id for the promise that was resolve()d.
     */
    promiseResolve?(asyncId: number): void;
    /**
     * Called after the resource corresponding to asyncId is destroyed
     * @param asyncId a unique ID for the async resource
     */
    destroy?(asyncId: number): void;
  }
  interface AsyncHook {
    /**
     * Enable the callbacks for a given AsyncHook instance. If no callbacks are provided enabling is a noop.
     */
    enable(): this;
    /**
     * Disable the callbacks for a given AsyncHook instance from the global pool of AsyncHook callbacks to be executed. Once a hook has been disabled it will not be called again until enabled.
     */
    disable(): this;
  }
  /**
   * Registers functions to be called for different lifetime events of each async
   * operation.
   *
   * The callbacks `init()`/`before()`/`after()`/`destroy()` are called for the
   * respective asynchronous event during a resource's lifetime.
   *
   * All callbacks are optional. For example, if only resource cleanup needs to
   * be tracked, then only the `destroy` callback needs to be passed. The
   * specifics of all functions that can be passed to `callbacks` is in the `Hook Callbacks` section.
   *
   * ```js
   * import { createHook } from 'async_hooks';
   *
   * const asyncHook = createHook({
   *   init(asyncId, type, triggerAsyncId, resource) { },
   *   destroy(asyncId) { },
   * });
   * ```
   *
   * The callbacks will be inherited via the prototype chain:
   *
   * ```js
   * class MyAsyncCallbacks {
   *   init(asyncId, type, triggerAsyncId, resource) { }
   *   destroy(asyncId) {}
   * }
   *
   * class MyAddedCallbacks extends MyAsyncCallbacks {
   *   before(asyncId) { }
   *   after(asyncId) { }
   * }
   *
   * const asyncHook = async_hooks.createHook(new MyAddedCallbacks());
   * ```
   *
   * Because promises are asynchronous resources whose lifecycle is tracked
   * via the async hooks mechanism, the `init()`, `before()`, `after()`, and`destroy()` callbacks _must not_ be async functions that return promises.
   * @param callbacks The `Hook Callbacks` to register
   * @return Instance used for disabling and enabling hooks
   */
  function createHook(callbacks: HookCallbacks): AsyncHook;
}
declare module "async_hooks" {
  export * from "async_hooks";
}
