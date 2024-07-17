// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::path::PathBuf;

use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, Ctx, Exception, Result};
use tokio::fs::OpenOptions;

use super::file_handle::FileHandle;

pub async fn open(ctx: Ctx<'_>, path: String, flags: String, mode: Opt<u32>) -> Result<FileHandle> {
    let mut options = OpenOptions::new();
    match flags.as_str() {
        // We are not supporting the sync modes
        "a" => options.append(true).create(true),
        "ax" => options.append(true).create_new(true),
        "a+" => options.append(true).read(true),
        "r" => options.read(true),
        "r+" => options.read(true).write(true),
        "w" => options.write(true).create(true).truncate(true),
        "wx" => options.write(true).create_new(true),
        "w+" => options.write(true).read(true).create(true).truncate(true),
        "wx+" => options.write(true).read(true).create_new(true),
        _ => {
            return Err(Exception::throw_message(
                &ctx,
                &format!("Invalid flags '{}'", flags),
            ))
        },
    };
    #[cfg(unix)]
    {
        let mode = mode.0.unwrap_or(0o666);
        options.mode(mode);
    }
    #[cfg(not(unix))]
    {
        _ = mode;
    }

    let path = PathBuf::from(path);
    let file = options
        .open(&path)
        .await
        .or_throw_msg(&ctx, "Cannot open file")?;

    Ok(FileHandle::new(file, path))
}

#[cfg(test)]
mod tests {
    use crate::{
        buffer,
        fs::FsPromisesModule,
        test::{call_test, given_file, test_async_with, ModuleEvaluator},
    };

    #[tokio::test]
    async fn test_file_handle_read() {
        let path = given_file("Hello World")
            .await
            .to_string_lossy()
            .to_string();
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<FsPromisesModule>(ctx.clone(), "fs/promises")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { open } from 'fs/promises';

                        export async function test(path) {
                            let filehandle = null;
                            try {
                                filehandle = await open(path, 'r+');
                                let { buffer } = await filehandle.read();
                                return Array.from(buffer);
                            } finally {
                                await filehandle?.close();
                            }
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<Vec<u8>, _>(&ctx, &module, (path,)).await;

                assert!(result.starts_with(b"Hello World"));
            })
        })
        .await;
    }
}
