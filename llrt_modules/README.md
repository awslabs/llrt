# LLRT Modules

LLRT Modules is a meta-module of [rquickjs](https://github.com/DelSkayn/rquickjs) modules that can be used independantly of LLRT (**L**ow **L**atency **R**un**t**ime). They aim to bring to [quickjs](https://bellard.org/quickjs/) APIs from [Node.js](https://nodejs.org/) and [WinterTC](https://wintertc.org/). You can use this meta-module, but each module is also a unique crate.

LLRT (**L**ow **L**atency **R**un**t**ime) is a lightweight JavaScript runtime designed to address the growing demand for fast and efficient Serverless applications.

## Usage

The package is not available in the crate registry yet, but you can clone the repo and import it as a local path.

Use this script to set everything up:

```bash
cd your_project_dir
git clone https://github.com/awslabs/llrt.git

cd llrt
npm i
make js
```

Each module has a feature flag, they are all enabled by default but if you prefer to can decide which one you need.
Check the [Compability matrix](#compatibility-matrix) for the full list.

```toml
[dependencies]
llrt_modules = { path = "llrt/llrt_modules", default-features = true } # load from local path
rquickjs = { git = "https://github.com/DelSkayn/rquickjs.git", version = "0.10.0", features = [
"full-async"] }
tokio = { version = "1", features = ["full"] }

```

Once you have enable a module, you can import it in your runtime.

> [!NOTE]
> Some modules currently require that you call an `init` function **before** they evaluated.

```rust
use llrt_modules::buffer;
use rquickjs::{async_with, context::EvalOptions, AsyncContext, AsyncRuntime, Error, Module};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let runtime = AsyncRuntime::new()?;
    let context = AsyncContext::full(&runtime).await?;

    async_with!(context => |ctx| {
        buffer::init(&ctx)?;
        let (_module, module_eval) = Module::evaluate_def::<buffer::BufferModule,_>(ctx.clone(), "buffer")?;
        module_eval.into_future::<()>().await?;

        let mut options = EvalOptions::default();
        options.global = false;
        if let Err(Error::Exception) = ctx.eval_with_options::<(), _>(
            r#"
            import { Buffer } from "node:buffer";
            Buffer.alloc(10);
            "#,
            options
        ){
            println!("{:#?}", ctx.catch());
        };

        Ok::<_, Error>(())
    })
    .await?;

    Ok(())
}
```

Using ModuleBuilder makes it even simpler.

```rust
use llrt_modules::module_builder::ModuleBuilder;
use rquickjs::{async_with, context::EvalOptions, AsyncContext, AsyncRuntime, Error, Module};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let runtime = AsyncRuntime::new()?;

    let module_builder = ModuleBuilder::default();
    let (module_resolver, module_loader, global_attachment) = module_builder.build();
    runtime.set_loader((module_resolver,), (module_loader,)).await;

    let context = AsyncContext::full(&runtime).await?;

    async_with!(context => |ctx| {
        global_attachment.attach(&ctx)?;

        let mut options = EvalOptions::default();
        options.global = false;
        if let Err(Error::Exception) = ctx.eval_with_options::<(), _>(
            r#"
            import { Buffer } from "node:buffer";
            Buffer.alloc(10);
            "#,
            options
        ){
            println!("{:#?}", ctx.catch());
        };

        Ok::<_, Error>(())
    })
    .await?;

    Ok(())
}

```

## Compatibility matrix

> [!NOTE]
> Only a fraction of the Node.js APIs are supported. Below is a high level overview of partially supported APIs and modules.

|                | Node.js | LLRT Modules | Feature          | Crate                 |
| -------------- | ------- | ------------ | ---------------- | --------------------- |
| abort          | ✔︎     | ✔︎️         | `abort`          | `llrt_abort`          |
| assert         | ✔︎     | ⚠️           | `assert`         | `llrt_assert`         |
| async_hooks    | ✔︎     | ⚠️           | `async-hooks`    | `llrt_async_hooks`    |
| buffer         | ✔︎     | ⚠️           | `buffer`         | `llrt_buffer`         |
| child process  | ✔︎     | ⚠️           | `child-process`  | `llrt_child_process`  |
| console        | ✔︎     | ⚠️           | `console`        | `llrt_console`        |
| crypto         | ✔︎     | ⚠️           | `crypto`         | `llrt_crypto`         |
| dns            | ✔︎     | ⚠️           | `dns`            | `llrt_dns`            |
| events         | ✔︎     | ⚠️           | `events`         | `llrt_events`         |
| exceptions     | ✔︎     | ⚠️           | `exceptions`     | `llrt_exceptions`     |
| fetch          | ✔︎     | ⚠️           | `fetch`          | `llrt_fetch`          |
| fs/promises    | ✔︎     | ⚠️           | `fs`             | `llrt_fs`             |
| fs             | ✔︎     | ⚠️           | `fs`             | `llrt_fs`             |
| navigator      | ✔︎     | ⚠️           | `navigator`      | `llrt_navigator`      |
| net            | ✔︎     | ⚠️           | `net`            | `llrt_net`            |
| os             | ✔︎     | ⚠️           | `os`             | `llrt_os`             |
| path           | ✔︎     | ⚠️           | `path`           | `llrt_path`           |
| perf hooks     | ✔︎     | ⚠️           | `perf-hooks`     | `llrt_perf_hooks`     |
| stream (lib)   | N/A     | ✔︎          | N/A              | `llrt_stream`         |
| string_decoder | ✔︎     | ✔︎          | `string_decoder` | `llrt_string_decoder` |
| timers         | ✔︎     | ⚠️           | `timers`         | `llrt_timers`         |
| process        | ✔︎     | ⚠️           | `process`        | `llrt_process`        |
| tty            | ✔︎     | ⚠️           | `tty`            | `llrt_tty`            |
| url            | ✔︎     | ⚠️           | `url`            | `llrt_url`            |
| util           | ✔︎     | ⚠️           | `util`           | `llrt_util`           |
| zlib           | ✔︎     | ⚠️           | `zlib`           | `llrt_zlib`           |
| Other modules  | ✔︎     | ✘            | N/A              | N/A                   |

_⚠️ = partially supported_

## License

This module is licensed under the Apache-2.0 License.
