/**
 * This module provides an implementation of a subset of the W3C [Web Performance APIs](https://w3c.github.io/perf-timing-primer/) as well as additional APIs for
 * Node.js-specific performance measurements.
 *
*/
declare module "perf_hooks" {
  interface Performance {
    /**
     * Returns the current high resolution millisecond timestamp, where 0 represents the start of the current `node` process.
     * @since v8.5.0
     */
    now(): number;
    /**
     * The [`timeOrigin`](https://w3c.github.io/hr-time/#dom-performance-timeorigin) specifies the high resolution millisecond timestamp
     * at which the current `node` process began, measured in Unix time.
     * @since v8.5.0
     */
    readonly timeOrigin: number;
    /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Performance/toJSON) */
    toJSON(): { timeOrigin: number }; // TODO: llrt currently has only one field
  }
  var performance: Performance
}
