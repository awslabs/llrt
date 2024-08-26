# LLRT Modules

LLRT Modules is a library of [rquickjs](https://github.com/DelSkayn/rquickjs) modules that can be used independantly of LLRT (**L**ow **L**atency **R**un**t**ime). They aim to bring to [quickjs](https://bellard.org/quickjs/) APIs from [Node.js](https://nodejs.org/) and [WinterCG](https://wintercg.org/).

LLRT (**L**ow **L**atency **R**un**t**ime) is a lightweight JavaScript runtime designed to address the growing demand for fast and efficient Serverless applications.

## Usage

Each module has a feature flag, they are all enabled by default but if you prefer to can decide which one you need.
Check the [Compability matrix](#compatibility-matrix) for the full list.

```toml
[dependencies]
llrt_modules = { version = "<version>", features = ["<feature>"], default-features = false }
```

Once you have enable a module, you can import it in your runtime.

> [!NOTE]
> Some modules currently require that you call an `init` function **before** they evaluated.

```rust
use llrt_modules::buffer;
use rquickjs::{AsyncRuntime, AsyncContext, async_with, Error, Module};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let runtime = AsyncRuntime::new()?;
    let context = AsyncContext::full(&runtime).await?;

    async_with!(context => |ctx| {
        buffer::init(ctx)?;
        let (_module, module_eval) = Module::evaluate_def::<buffer::BufferModule, _>(ctx.clone(), "buffer")?;
        module_eval.into_future().await?;

        ctx.eval(
          r#"
          import { Buffer } from "buffer";
          Buffer.alloc(10);
          "#,
        )?;

        Ok::<_, Error>(())
    })
    .await?;

    Ok(())
}
```

## Compatibility matrix

> [!NOTE]
> Only a fraction of the Node.js APIs are supported. Below is a high level overview of partially supported APIs and modules.

|               | Node.js | LLRT Modules | Feature         |
| ------------- | ------- | ------------ | --------------- |
| buffer        | ✔︎     | ✔︎️         | `buffer`        |
| child process | ✔︎     | ⚠️           | `child-process` |
| crypto        | ✔︎     | ⚠️           | `crypto`        |
| events        | ✔︎     | ⚠️           | `events`        |
| exceptions    | ✔︎     | ⚠️           | `exceptions`    |
| fs/promises   | ✔︎     | ⚠️           | `fs`            |
| fs            | ✔︎     | ⚠️           | `fs`            |
| navigator     | ✔︎     | ⚠️           | `navigator`     |
| net           | ✔︎     | ⚠️           | `net`           |
| os            | ✔︎     | ⚠️           | `os`            |
| path          | ✔︎     | ✔︎          | `path`          |
| perf hooks    | ✔︎     | ⚠️           | `perf-hooks`    |
| process       | ✔︎     | ✔︎          | `process`       |
| zlib          | ✔︎     | ⚠️           | `zlib`          |
| Other modules | ✔︎     | ✘            | N/A             |

_⚠️ = partially supported_
_⏱ = planned partial support_
_\* = Not native_
_\*\* = Use fetch instead_

## License

This library is licensed under the Apache-2.0 License.
