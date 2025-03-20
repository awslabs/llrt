/**
 * The `console` module provides a simple debugging console that is similar to
 * the JavaScript console mechanism provided by web browsers.
 *
 * Example using the global `console`:
 *
 * ```js
 * console.log('hello world');
 * // Prints: hello world, to stdout
 * console.log('hello %s', 'world');
 * // Prints: hello world, to stdout
 * console.error(new Error('Whoops, something bad happened'));
 * // Prints error message and stack trace to stderr:
 * //   Error: Whoops, something bad happened
 * //     at [eval]:5:15
 * //     at Script.runInThisContext (vm:132:18)
 * //     at Object.runInThisContext (vm:309:38)
 * //     at internal/process/execution:77:19
 * //     at [eval]-wrapper:6:22
 * //     at evalScript (internal/process/execution:76:60)
 * //     at internal/main/eval_string:23:3
 *
 * const name = 'Will Robinson';
 * console.warn(`Danger ${name}! Danger!`);
 * // Prints: Danger Will Robinson! Danger!, to stderr
 * ```
 *
 * Example using the `Console` class:
 *
 * ```js
 * const out = getStreamSomehow();
 * const err = getStreamSomehow();
 * const myConsole = new console.Console(out, err);
 *
 * myConsole.log('hello world');
 * // Prints: hello world, to out
 * myConsole.log('hello %s', 'world');
 * // Prints: hello world, to out
 * myConsole.error(new Error('Whoops, something bad happened'));
 * // Prints: [Error: Whoops, something bad happened], to err
 *
 * const name = 'Will Robinson';
 * myConsole.warn(`Danger ${name}! Danger!`);
 * // Prints: Danger Will Robinson! Danger!, to err
 * ```
 * @see [source](https://github.com/nodejs/node/blob/v22.x/lib/console.js)
 */
declare module "console" {
    global {
        // This needs to be global to avoid TS2403 in case lib.dom.d.ts is present in the same build
        interface Console {
            /**
             * `console.assert()` writes a message if `value` is [falsy](https://developer.mozilla.org/en-US/docs/Glossary/Falsy) or omitted. It only
             * writes a message and does not otherwise affect execution. The output always
             * starts with `"Assertion failed"`. If provided, `message` is formatted using
             * [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args).
             *
             * If `value` is [truthy](https://developer.mozilla.org/en-US/docs/Glossary/Truthy), nothing happens.
             *
             * ```js
             * console.assert(true, 'does nothing');
             *
             * console.assert(false, 'Whoops %s work', 'didn\'t');
             * // Assertion failed: Whoops didn't work
             *
             * console.assert();
             * // Assertion failed
             * ```
             * @param value The value tested for being truthy.
             * @param message All arguments besides `value` are used as error message.
             */
            assert(value: any, message?: string, ...optionalParams: any[]): void;
            /**
             * When `stdout` is a TTY, calling `console.clear()` will attempt to clear the
             * TTY. When `stdout` is not a TTY, this method does nothing.
             *
             * The specific operation of `console.clear()` can vary across operating systems
             * and terminal types. For most Linux operating systems, `console.clear()` operates similarly to the `clear` shell command. On Windows, `console.clear()` will clear only the output in the
             * current terminal viewport for the LLRT
             * binary.
             */
            clear(): void;
            /**
             * The `console.debug()` function is an alias for {@link log}.
             */
            debug(message?: any, ...optionalParams: any[]): void;
            /**
             * Prints to `stderr` with newline. Multiple arguments can be passed, with the
             * first used as the primary message and all additional used as substitution
             * values similar to [`printf(3)`](http://man7.org/linux/man-pages/man3/printf.3.html)
             * (the arguments are all passed to [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args)).
             *
             * ```js
             * const code = 5;
             * console.error('error #%d', code);
             * // Prints: error #5, to stderr
             * console.error('error', code);
             * // Prints: error 5, to stderr
             * ```
             *
             * If formatting elements (e.g. `%d`) are not found in the first string then
             * [`util.inspect()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilinspectobject-options) is called on each argument and the
             * resulting string values are concatenated. See [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args)
             * for more information.
             */
            error(message?: any, ...optionalParams: any[]): void;
            /**
             * The `console.info()` function is an alias for {@link log}.
             */
            info(message?: any, ...optionalParams: any[]): void;
            /**
             * Prints to `stdout` with newline. Multiple arguments can be passed, with the
             * first used as the primary message and all additional used as substitution
             * values similar to [`printf(3)`](http://man7.org/linux/man-pages/man3/printf.3.html)
             * (the arguments are all passed to [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args)).
             *
             * ```js
             * const count = 5;
             * console.log('count: %d', count);
             * // Prints: count: 5, to stdout
             * console.log('count:', count);
             * // Prints: count: 5, to stdout
             * ```
             *
             * See [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args) for more information.
             */
            log(message?: any, ...optionalParams: any[]): void;
            /**
             * Prints to `stderr` the string `'Trace: '`, followed by the [`util.format()`](https://nodejs.org/docs/latest-v22.x/api/util.html#utilformatformat-args)
             * formatted message and stack trace to the current position in the code.
             *
             * ```js
             * console.trace('Show me');
             * // Prints: (stack trace will vary based on where trace is called)
             * //  Trace: Show me
             * //    at repl:2:9
             * //    at REPLServer.defaultEval (repl.js:248:27)
             * //    at bound (domain.js:287:14)
             * //    at REPLServer.runBound [as eval] (domain.js:300:12)
             * //    at REPLServer.<anonymous> (repl.js:412:12)
             * //    at emitOne (events.js:82:20)
             * //    at REPLServer.emit (events.js:169:7)
             * //    at REPLServer.Interface._onLine (readline.js:210:10)
             * //    at REPLServer.Interface._line (readline.js:549:8)
             * //    at REPLServer.Interface._ttyWrite (readline.js:826:14)
             * ```
             */
            trace(message?: any, ...optionalParams: any[]): void;
            /**
             * The `console.warn()` function is an alias for {@link error}.
             */
            warn(message?: any, ...optionalParams: any[]): void;
        }
        /**
         * The `console` module provides a simple debugging console that is similar to the
         * JavaScript console mechanism provided by web browsers.
         *
         *
         * _**Warning**_: The global console object's methods are neither consistently
         * synchronous like the browser APIs they resemble, nor are they consistently
         * asynchronous like all other LLRT streams. See the [`note on process I/O`](https://nodejs.org/docs/latest-v22.x/api/process.html#a-note-on-process-io) for
         * more information.
         *
         * Example using the global `console`:
         *
         * ```js
         * console.log('hello world');
         * // Prints: hello world, to stdout
         * console.log('hello %s', 'world');
         * // Prints: hello world, to stdout
         * console.error(new Error('Whoops, something bad happened'));
         * // Prints error message and stack trace to stderr:
         * //   Error: Whoops, something bad happened
         * //     at [eval]:5:15
         * //     at Script.runInThisContext (vm:132:18)
         * //     at Object.runInThisContext (vm:309:38)
         * //     at internal/process/execution:77:19
         * //     at [eval]-wrapper:6:22
         * //     at evalScript (internal/process/execution:76:60)
         * //     at internal/main/eval_string:23:3
         *
         * const name = 'Will Robinson';
         * console.warn(`Danger ${name}! Danger!`);
         * // Prints: Danger Will Robinson! Danger!, to stderr
         * ```
         *
         * Example using the `Console` class:
         *
         * ```js
         * const out = getStreamSomehow();
         * const err = getStreamSomehow();
         * const myConsole = new console.Console(out, err);
         *
         * myConsole.log('hello world');
         * // Prints: hello world, to out
         * myConsole.log('hello %s', 'world');
         * // Prints: hello world, to out
         * myConsole.error(new Error('Whoops, something bad happened'));
         * // Prints: [Error: Whoops, something bad happened], to err
         *
         * const name = 'Will Robinson';
         * myConsole.warn(`Danger ${name}! Danger!`);
         * // Prints: Danger Will Robinson! Danger!, to err
         * ```
         * @see [source](https://github.com/nodejs/node/blob/v22.x/lib/console.js)
         */
        var console: Console;
    }
    export = globalThis.console;
}
