#[cfg(test)]
mod tests {
    use llrt_test::test_async_with;
    use rquickjs::Promise;

    fn eval_async<'js>(ctx: &rquickjs::Ctx<'js>, js: &str) -> rquickjs::Result<Promise<'js>> {
        ctx.eval(format!("(async () => {{ {js} }})()"))
    }

    #[tokio::test]
    async fn identity_passthrough() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream();
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write("one");
                    writer.write("two");
                    writer.close();

                    const chunks = [];
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        chunks.push(value);
                    }
                    if (chunks.join(",") !== "one,two") throw new Error("got: " + chunks);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn transform_chunks() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            controller.enqueue(chunk.toUpperCase());
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write("hello");
                    writer.write("world");
                    writer.close();

                    const chunks = [];
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        chunks.push(value);
                    }
                    if (chunks.join(" ") !== "HELLO WORLD") throw new Error("got: " + chunks);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn one_to_many_expansion() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            for (const byte of chunk) {
                                controller.enqueue(byte);
                            }
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write([1, 2, 3]);
                    writer.close();

                    const chunks = [];
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        chunks.push(value);
                    }
                    if (chunks.join(",") !== "1,2,3") throw new Error("got: " + chunks);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn flush_on_close() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            controller.enqueue(chunk);
                        },
                        flush(controller) {
                            controller.enqueue("DONE");
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write("a");
                    writer.close();

                    const chunks = [];
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        chunks.push(value);
                    }
                    if (chunks.join(",") !== "a,DONE") throw new Error("got: " + chunks);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn pipe_through_chain() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const source = new ReadableStream({
                        start(controller) {
                            controller.enqueue("hello");
                            controller.enqueue("world");
                            controller.close();
                        }
                    });

                    const upper = new TransformStream({
                        transform(chunk, c) { c.enqueue(chunk.toUpperCase()); }
                    });
                    const exclaim = new TransformStream({
                        transform(chunk, c) { c.enqueue(chunk + "!"); }
                    });

                    const reader = source
                        .pipeThrough(upper)
                        .pipeThrough(exclaim)
                        .getReader();

                    const chunks = [];
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        chunks.push(value);
                    }
                    if (chunks.join(" ") !== "HELLO! WORLD!") throw new Error("got: " + chunks);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn async_transform() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        async transform(chunk, controller) {
                            await new Promise(r => setTimeout(r, 1));
                            controller.enqueue(chunk * 2);
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write(5);
                    writer.close();

                    const { value } = await reader.read();
                    if (value !== 10) throw new Error("expected 10, got " + value);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn error_propagates_to_reader() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            controller.error(new Error("broken"));
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write("x").catch(() => {});

                    try {
                        await reader.read();
                        throw new Error("should have thrown");
                    } catch (e) {
                        if (e.message !== "broken") throw new Error("wrong error: " + e.message);
                    }
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn start_receives_controller() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    let controllerRef;
                    const ts = new TransformStream({
                        start(controller) {
                            controllerRef = controller;
                            controller.enqueue("from-start");
                        }
                    });

                    if (typeof controllerRef.desiredSize !== "number")
                        throw new Error("controller.desiredSize should be a number");

                    const reader = ts.readable.getReader();
                    const { value } = await reader.read();
                    if (value !== "from-start") throw new Error("got: " + value);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn terminate_closes_readable() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            if (chunk === "stop") {
                                controller.terminate();
                                return;
                            }
                            controller.enqueue(chunk);
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    writer.write("keep").catch(() => {});
                    writer.write("stop").catch(() => {});

                    const { value } = await reader.read();
                    if (value !== "keep") throw new Error("got: " + value);

                    const { done } = await reader.read();
                    if (!done) throw new Error("expected stream to be closed after terminate");
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }

    #[tokio::test]
    async fn illegal_constructor() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(&ctx, r#"
                    try {
                        new TransformStreamDefaultController();
                        throw new Error("should have thrown");
                    } catch (e) {
                        if (!(e instanceof TypeError)) throw new Error("expected TypeError, got " + e);
                    }
                "#).unwrap().into_future::<()>().await.unwrap();
            })
        }).await;
    }

    #[tokio::test]
    async fn pipe_to_writable_stream() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                eval_async(
                    &ctx,
                    r#"
                    const collected = [];
                    const source = new ReadableStream({
                        start(c) { c.enqueue(1); c.enqueue(2); c.enqueue(3); c.close(); }
                    });
                    const transform = new TransformStream({
                        transform(chunk, c) { c.enqueue(chunk * 10); }
                    });
                    const sink = new WritableStream({
                        write(chunk) { collected.push(chunk); }
                    });

                    await source.pipeThrough(transform).pipeTo(sink);

                    if (collected.join(",") !== "10,20,30") throw new Error("got: " + collected);
                "#,
                )
                .unwrap()
                .into_future::<()>()
                .await
                .unwrap();
            })
        })
        .await;
    }
}
