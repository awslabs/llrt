// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Result};

pub fn promise(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    if global
        .get::<_, bool>("__promise_patched__")
        .unwrap_or(false)
    {
        return Ok(());
    }

    ctx.eval::<(), _>(
        r#"
        (function() {
            globalThis.__promise_patched__ = true;

            const OriginalPromise = globalThis.Promise;

            function runAfterTicks(fn, count = 1) {
                if (count <= 0) {
                    fn();
                } else {
                    queueMicrotask(() => runAfterTicks(fn, count - 1));
                }
            }

            function HookedPromise(executor) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();
                __async_hook_init(id, "PROMISE", triggerId);

                return new OriginalPromise((resolve, reject) => {
                    try {
                        executor(
                            (value) => {
                                if (value instanceof OriginalPromise) {
                                    __async_hook_func("promiseResolve", id);
                                }
                                __async_hook_func("before", id);
                                resolve(value);
                                runAfterTicks(() => {
                                    __async_hook_func("after", id);
                                    runAfterTicks(() => {
                                        __async_hook_func("destroy", id);
                                    }, 3);
                                }, 2);
                            },
                            (err) => {
                                __async_hook_func("before", id);
                                reject(err);
                                runAfterTicks(() => {
                                    __async_hook_func("after", id);
                                    runAfterTicks(() => {
                                        __async_hook_func("destroy", id);
                                    }, 3);
                                }, 2);
                            }
                        );
                    } catch (err) {
                        reject(err);
                    }
                });
            }

            HookedPromise.prototype = OriginalPromise.prototype;
            Object.setPrototypeOf(HookedPromise, OriginalPromise);

            // --- Patch: resolve ---
            HookedPromise.resolve = function (value) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();
                __async_hook_init(id, "PROMISE", triggerId);
                __async_hook_func("promiseResolve", id);

                return OriginalPromise.resolve(value).then((v) => {
                    __async_hook_func("before", id);
                    return v;
                }).finally(() => {
                    runAfterTicks(() => {
                        __async_hook_func("after", id);
                        runAfterTicks(() => {
                            __async_hook_func("destroy", id);
                        }, 3);
                    }, 2);
                });
            };

            // --- Patch: reject ---
            HookedPromise.reject = function (reason) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();
                __async_hook_init(id, "PROMISE", triggerId);

                return OriginalPromise.reject(reason).catch((e) => {
                    __async_hook_func("before", id);
                    throw e;
                }).finally(() => {
                    runAfterTicks(() => {
                        __async_hook_func("after", id);
                        runAfterTicks(() => {
                            __async_hook_func("destroy", id);
                        }, 3);
                    }, 2);
                });
            };

            // --- Patch: all ---
            HookedPromise.all = function (iterable) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();
                __async_hook_init(id, "PROMISE", triggerId);
                __async_hook_func("promiseResolve", id);

                return OriginalPromise.all(iterable).then((v) => {
                    __async_hook_func("before", id);
                    return v;
                }).finally(() => {
                    runAfterTicks(() => {
                        __async_hook_func("after", id);
                        runAfterTicks(() => {
                            __async_hook_func("destroy", id);
                        }, 3);
                    }, 2);
                });
            };

            // --- Patch: race ---
            HookedPromise.race = function (iterable) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();
                __async_hook_init(id, "PROMISE", triggerId);
                __async_hook_func("promiseResolve", id);

                return OriginalPromise.race(iterable).then((v) => {
                    __async_hook_func("before", id);
                    return v;
                }).finally(() => {
                    runAfterTicks(() => {
                        __async_hook_func("after", id);
                        runAfterTicks(() => {
                            __async_hook_func("destroy", id);
                        }, 3);
                    }, 2);
                });
            };

            // Optional: patch allSettled, any
            ["allSettled", "any"].forEach((method) => {
                HookedPromise[method] = function (iterable) {
                    const id = __async_hook_next_id();
                    const triggerId = __async_hook_exec_id();
                    __async_hook_init(id, "PROMISE", triggerId);
                    __async_hook_func("promiseResolve", id);

                    return OriginalPromise[method](iterable).finally(() => {
                        __async_hook_func("before", id);
                        runAfterTicks(() => {
                            __async_hook_func("after", id);
                            runAfterTicks(() => {
                                __async_hook_func("destroy", id);
                            }, 3);
                        }, 2);
                    });
                };
            });

            globalThis.Promise = HookedPromise;
        })();
    "#,
    )?;

    Ok(())
}

pub fn timeout(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    if global
        .get::<_, bool>("__timeout_patched__")
        .unwrap_or(false)
    {
        return Ok(());
    }

    ctx.eval::<(), _>(
        r#"
        (function() {
            globalThis.__timeout_patched__ = true;

            const originalSetTimeout = globalThis.setTimeout;

            globalThis.setTimeout = function(callback, delay, ...args) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();

                __async_hook_init(id, "Timeout", triggerId);

                __async_hook_func("before", id);

                const timeoutId = originalSetTimeout(() => {
                    __async_hook_func("after", id);
                    callback(...args);

                    queueMicrotask(() => {
                        __async_hook_func("destroy", id);
                    });
                }, delay);

                return timeoutId;
            };
        })();
    "#,
    )?;

    Ok(())
}

pub fn immediate(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    if global
        .get::<_, bool>("__immediate_patched__")
        .unwrap_or(false)
    {
        return Ok(());
    }

    ctx.eval::<(), _>(
        r#"
        (function() {
            globalThis.__immediate_patched__ = true;
            
            const originalSetImmediate = globalThis.setImmediate;

            globalThis.setImmediate = function(callback, ...args) {
                const id = __async_hook_next_id();
                const triggerId = __async_hook_exec_id();

                __async_hook_init(id, "Immediate", triggerId);

                __async_hook_func("before", id);

                const immediateId = originalSetImmediate(() => {
                    __async_hook_func("after", id);
                    callback(...args);

                    queueMicrotask(() => {
                        __async_hook_func("destroy", id);
                    });
                });

                return immediateId;
            };
        })();
    "#,
    )?;

    Ok(())
}
