use llrt_test::test_async_with;
use rquickjs::Promise;

fn eval_async<'js>(ctx: &rquickjs::Ctx<'js>, js: &str) -> rquickjs::Result<Promise<'js>> {
    ctx.eval(format!("(async () => {{ {js} }})()"))
}

fn init(ctx: &rquickjs::Ctx<'_>) {
    llrt_stream_web::init(ctx).unwrap();
    crate::init(ctx).unwrap();
}

#[tokio::test]
async fn encoder_stream_encodes_chunk() {
    test_async_with(|ctx| {
        init(&ctx);
        Box::pin(async move {
            eval_async(
                &ctx,
                r#"
                const es = new TextEncoderStream();
                const writer = es.writable.getWriter();
                const reader = es.readable.getReader();

                writer.write("hi");
                writer.close();

                const { value, done } = await reader.read();
                if (done) throw new Error("expected a chunk");
                if (!(value instanceof Uint8Array)) throw new Error("expected Uint8Array");
                if (value.join(",") !== "104,105") throw new Error("got: " + value.join(","));
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
async fn decoder_stream_decodes_split_utf8_char() {
    test_async_with(|ctx| {
        init(&ctx);
        Box::pin(async move {
            eval_async(
                &ctx,
                r#"
                const ds = new TextDecoderStream();
                const writer = ds.writable.getWriter();
                const reader = ds.readable.getReader();

                const euro = new Uint8Array([0xe2, 0x82, 0xac]);
                writer.write(euro.slice(0, 1));
                writer.write(euro.slice(1));
                writer.close();

                let result = "";
                while (true) {
                    const { value, done } = await reader.read();
                    if (done) break;
                    result += value;
                }
                if (result !== "\u20ac") throw new Error("got: " + result);
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
async fn decoder_stream_encoding_getter() {
    test_async_with(|ctx| {
        init(&ctx);
        Box::pin(async move {
            eval_async(
                &ctx,
                r#"
                const ds = new TextDecoderStream("utf-8");
                if (ds.encoding !== "utf-8") throw new Error("got: " + ds.encoding);
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
