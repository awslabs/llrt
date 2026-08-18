use llrt_test::test_sync_with;

#[tokio::test]
async fn high_water_mark_uses_javascript_number_conversion() {
    test_sync_with(|ctx| {
        crate::init(&ctx)?;
        ctx.eval::<(), _>(
            r#"
            const converted = {
                valueOf() {
                    return "7.5";
                },
            };

            const observed = [];
            new ReadableStream({
                start(controller) {
                    observed.push(controller.desiredSize);
                },
            }, { highWaterMark: converted });
            observed.push(
                new WritableStream({}, { highWaterMark: converted })
                    .getWriter().desiredSize,
            );

            const transform = new TransformStream({
                start(controller) {
                    observed.push(controller.desiredSize);
                },
            }, { highWaterMark: converted }, { highWaterMark: converted });
            observed.push(transform.writable.getWriter().desiredSize);

            observed.push(
                new CountQueuingStrategy({ highWaterMark: converted }).highWaterMark,
                new ByteLengthQueuingStrategy({ highWaterMark: converted }).highWaterMark,
            );

            if (observed.length !== 6 || observed.some(value => value !== 7.5)) {
                throw new Error(`Unexpected highWaterMark values: ${observed}`);
            }

            const primitiveConversions = [
                [false, 0],
                [true, 1],
                ["2.25", 2.25],
            ];
            for (const [input, expected] of primitiveConversions) {
                const strategy = new CountQueuingStrategy({ highWaterMark: input });
                if (strategy.highWaterMark !== expected) {
                    throw new Error(`${String(input)} converted to ${strategy.highWaterMark}`);
                }
            }
            "#,
        )
    })
    .await;
}

#[tokio::test]
async fn high_water_mark_conversion_order_and_errors_are_observable() {
    test_sync_with(|ctx| {
        crate::init(&ctx)?;
        ctx.eval::<(), _>(
            r#"
            const order = [];
            new WritableStream({}, {
                get highWaterMark() {
                    order.push("get highWaterMark");
                    return {
                        valueOf() {
                            order.push("convert highWaterMark");
                            return 1;
                        },
                    };
                },
                get size() {
                    order.push("get size");
                    return undefined;
                },
            });

            const expectedOrder =
                "get highWaterMark,convert highWaterMark,get size";
            if (order.join() !== expectedOrder) {
                throw new Error(`Unexpected conversion order: ${order}`);
            }

            const expectedError = new Error("number conversion failed");
            const throwingValue = {
                valueOf() {
                    throw expectedError;
                },
            };
            const factories = [
                () => new ReadableStream({}, { highWaterMark: throwingValue }),
                () => new WritableStream({}, { highWaterMark: throwingValue }),
                () => new TransformStream({}, { highWaterMark: throwingValue }),
                () => new TransformStream({}, {}, { highWaterMark: throwingValue }),
                () => new CountQueuingStrategy({ highWaterMark: throwingValue }),
                () => new ByteLengthQueuingStrategy({ highWaterMark: throwingValue }),
            ];

            for (const factory of factories) {
                try {
                    factory();
                    throw new Error("Expected number conversion to throw");
                } catch (error) {
                    if (error !== expectedError) {
                        throw new Error(`Unexpected conversion error: ${error}`);
                    }
                }
            }

            for (const input of [1n, Symbol("highWaterMark")]) {
                try {
                    new CountQueuingStrategy({ highWaterMark: input });
                    throw new Error("Expected ToNumber to reject the value");
                } catch (error) {
                    if (!(error instanceof TypeError)) {
                        throw new Error(`Expected TypeError, got ${error}`);
                    }
                }
            }
            "#,
        )
    })
    .await;
}

#[tokio::test]
async fn invalid_converted_stream_high_water_marks_throw_range_error() {
    test_sync_with(|ctx| {
        crate::init(&ctx)?;
        ctx.eval::<(), _>(
            r#"
            for (const highWaterMark of ["-1", "not a number"]) {
                const factories = [
                    () => new ReadableStream({}, { highWaterMark }),
                    () => new WritableStream({}, { highWaterMark }),
                    () => new TransformStream({}, { highWaterMark }),
                    () => new TransformStream({}, {}, { highWaterMark }),
                ];
                for (const factory of factories) {
                    try {
                        factory();
                        throw new Error("Expected invalid highWaterMark to throw");
                    } catch (error) {
                        if (!(error instanceof RangeError)) {
                            throw new Error(`Expected RangeError, got ${error}`);
                        }
                    }
                }
            }
            "#,
        )
    })
    .await;
}
