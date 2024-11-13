/**
 * The `assert` module provides a set of assertion functions for verifying invariants.
 */
declare module "assert" {
  /**
   * An alias of {@link ok}.
   * @param value The input that is checked for being truthy.
   */
  function assert(value: unknown, message?: string | Error): asserts value;
  /**
   * Tests if `value` is truthy.
   *
   * If `value` is not truthy, an `AssertionError` is thrown with a `message` property set equal to the value of the `message` parameter. If the `message` parameter is `undefined`, a default
   * error message is assigned. If the `message` parameter is an instance of an `Error` then it will be thrown instead of the `AssertionError`.
   * If no arguments are passed in at all `message` will be set to the string:`` 'No value argument passed to `assert.ok()`' ``.
   *
   * ```js
   * import * as assert from 'assert';
   *
   * assert.ok(true);
   * // OK
   * assert.ok(1);
   * // OK
   *
   * assert.ok();
   * // TypeError: Error calling function with 0 argument(s) while 1 where expected
   *
   * assert.ok(false, 'it\'s false');
   * // AssertionError: it's false
   *
   * assert.ok(false);
   * // AssertionError: The expression was evaluated to a falsy value
   *
   * assert.ok(0);
   * // AssertionError: The expression was evaluated to a falsy value
   * ```
   */
  function ok(value: unknown, message?: string | Error): asserts value;
}
