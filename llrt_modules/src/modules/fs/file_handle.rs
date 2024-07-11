use std::path::PathBuf;

use either::Either;
use llrt_utils::array_buffer::ArrayBufferView;
use llrt_utils::object::ObjectExt;
use llrt_utils::result::{OptionExt, ResultExt};
use rquickjs::function::Opt;
use rquickjs::{Ctx, Error, FromJs, Object, Result, Value};
use tokio::io::AsyncReadExt;
use tokio::{fs::File, task};

use super::read_file;

const DEFAULT_BUFFER_SIZE: usize = 16384;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct FileHandle {
    #[qjs(skip_trace)]
    file: Option<File>,
    #[qjs(skip_trace)]
    path: PathBuf,
}

impl FileHandle {
    pub fn new(file: File, path: PathBuf) -> Self {
        Self {
            file: Some(file),
            path,
        }
    }

    fn file(&self, ctx: &Ctx<'_>) -> Result<&File> {
        self.file.as_ref().or_throw_msg(ctx, "FileHandle is closed")
    }

    fn file_mut(&mut self, ctx: &Ctx<'_>) -> Result<&mut File> {
        self.file.as_mut().or_throw_msg(ctx, "FileHandle is closed")
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl FileHandle {
    async fn chmod(&self, ctx: Ctx<'_>, mode: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = std::fs::Permissions::from_mode(mode);
            self.file(&ctx)?
                .set_permissions(perm)
                .await
                .or_throw_msg(&ctx, "Can't modify file permissions")?;
        }
        Ok(())
    }

    async fn chown(&self, ctx: Ctx<'_>, uid: u32, gid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            let path = self.path.clone();
            task::spawn_blocking(move || std::os::unix::fs::chown(&path, Some(uid), Some(gid)))
                .await
                .or_throw(&ctx)?
                .or_throw_msg(&ctx, "Can't modify file owner")?;
        }
        Ok(())
    }

    async fn close(&mut self) {
        if let Some(file) = self.file.take() {
            drop(file.into_std().await);
        }
    }

    async fn datasync(&self, ctx: Ctx<'_>) -> Result<()> {
        self.file(&ctx)?
            .sync_data()
            .await
            .or_throw_msg(&ctx, "Can't sync file data")?;
        Ok(())
    }

    #[qjs(get)]
    async fn fd(&self, ctx: Ctx<'_>) -> Result<isize> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            self.file(&ctx)?.as_raw_fd().try_into().or_throw(&ctx)
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            let handle = self.file(&ctx)?.as_raw_handle();
            Ok(handle as isize)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Ok(0)
        }
    }

    async fn read<'js>(
        &mut self,
        ctx: Ctx<'js>,
        buffer_or_options: Opt<Either<ArrayBufferView<'js>, ReadOptions<'js>>>,
        options_or_offset: Opt<Either<ReadOptions<'js>, usize>>,
        length: Opt<usize>,
        // Not supporting position for now since it is not available in tokio.
        // See https://github.com/tokio-rs/tokio/issues/699
        // position: Opt<Either<isize, Null>>,
    ) -> Result<Object<'js>> {
        let options_1 = match buffer_or_options.0 {
            Some(Either::Left(buffer)) => ReadOptions {
                buffer: Some(buffer),
                ..Default::default()
            },
            Some(Either::Right(options)) => options,
            None => ReadOptions::default(),
        };
        let options_2 = match options_or_offset.0 {
            Some(Either::Left(options)) => options,
            Some(Either::Right(offset)) => ReadOptions {
                offset: Some(offset),
                ..Default::default()
            },
            None => ReadOptions::default(),
        };

        let buffer = options_1
            .buffer
            .or(options_2.buffer)
            .unwrap_or_else_ok(|| ArrayBufferView::new(ctx.clone(), DEFAULT_BUFFER_SIZE))?;
        let offset = options_1.offset.or(options_2.offset).unwrap_or(0);
        let length = options_1
            .length
            .or(options_2.length)
            .or(length.0)
            .unwrap_or_else(|| buffer.len() - offset);

        // It is not safe to pass the buffer from `ArrayBufferView` to `File::read`
        // since the read is done in a different thread and we cannot garantee
        // that multiple read calls are not done with the same buffer.
        // Ideally, we should make our own version of `BufReader` to reuse the buffer
        // instead of doing an allocation on each read.
        let mut buf = vec![0u8; length];
        let bytes_read = self
            .file_mut(&ctx)?
            .read(&mut buf)
            .await
            .or_throw_msg(&ctx, "Failed to read file")?;

        let dst_buf = unsafe {
            buffer
                .buffer_mut()
                .or_throw_msg(&ctx, "Buffer is detached")?
        };
        dst_buf[offset..].copy_from_slice(&buf);

        let result = Object::new(ctx)?;
        result.set("bytesRead", bytes_read)?;
        result.set("buffer", buffer)?;
        Ok(result)
    }

    async fn read_file<'js>(
        &mut self,
        ctx: Ctx<'js>,
        options: Opt<Either<String, read_file::ReadFileOptions>>,
    ) -> Result<Value<'js>> {
        let size = self
            .file(&ctx)?
            .metadata()
            .await
            .map(|m| m.len() as usize)
            .ok();
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size.unwrap_or(0))
            .or_throw_msg(&ctx, "Out of memory")?;

        self.file_mut(&ctx)?
            .read_to_end(&mut bytes)
            .await
            .or_throw_msg(&ctx, "Failed to read file")?;
        read_file::handle_read_file_bytes(&ctx, options, bytes)
    }

    async fn read_lines(&self) {}

    async fn stat(&self) {}

    async fn sync(&self, ctx: Ctx<'_>) -> Result<()> {
        self.file(&ctx)?
            .sync_all()
            .await
            .or_throw_msg(&ctx, "Can't sync file")
    }

    async fn truncate(&mut self, ctx: Ctx<'_>, len: Opt<u64>) -> Result<()> {
        let len = len.0.unwrap_or(0);
        self.file_mut(&ctx)?
            .set_len(len)
            .await
            .or_throw_msg(&ctx, "Can't truncate file")
    }

    async fn utimes(&self) {}

    async fn write(&mut self, ctx: Ctx<'_>) {}

    async fn write_file(&self) {}
}

#[derive(Default)]
struct ReadOptions<'js> {
    buffer: Option<ArrayBufferView<'js>>,
    offset: Option<usize>,
    length: Option<usize>,
}

impl<'js> FromJs<'js> for ReadOptions<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let buffer = obj.get_optional::<_, ArrayBufferView<'js>>("buffer")?;
        let offset = obj.get_optional::<_, usize>("offset")?;
        let length = obj.get_optional::<_, usize>("length")?;

        Ok(Self {
            buffer,
            offset,
            length,
        })
    }
}

#[cfg(test)]
mod tests {
    use rquickjs::Class;
    use tokio::fs::OpenOptions;

    use super::*;
    use crate::test::{call_test, test_async_with, ModuleEvaluator};

    async fn given_file(content: &str, options: &mut OpenOptions) -> (File, PathBuf) {
        // Create file
        let path = crate::test::given_file(content).await;

        // Open in right mode
        let file = options.open(&path).await.unwrap();
        (file, path)
    }

    #[tokio::test]
    async fn test_file_handle_read() {
        let (file, path) = given_file("Hello World", OpenOptions::new().read(true)).await;
        test_async_with(|ctx| {
            Box::pin(async move {
                Class::<FileHandle>::register(&ctx).unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        export async function test(filehandle) {
                            const buffer = new ArrayBuffer(4096);
                            const view = new Uint8Array(buffer);
                            const read = await filehandle.read(view);
                            return Array.from(view);
                        }
                    "#,
                )
                .await
                .unwrap();

                let result =
                    call_test::<Vec<u8>, _>(&ctx, &module, (FileHandle::new(file, path),)).await;

                assert!(result.starts_with(b"Hello World"));
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_file_handle_read_file() {
        let (file, path) = given_file("Hello World", OpenOptions::new().read(true)).await;
        test_async_with(|ctx| {
            Box::pin(async move {
                Class::<FileHandle>::register(&ctx).unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        export async function test(filehandle) {
                            const data = await filehandle.readFile("utf8");
                            return data;
                        }
                    "#,
                )
                .await
                .unwrap();

                let result =
                    call_test::<String, _>(&ctx, &module, (FileHandle::new(file, path),)).await;

                assert_eq!(result, "Hello World");
            })
        })
        .await;
    }
}
