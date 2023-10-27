use rquickjs::{function::Opt, Ctx, Object, Result};
use tokio::fs;

use crate::{buffer::Buffer, util::ResultExt};

//TODO implement options
pub async fn read_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    _options: Opt<Object<'js>>,
) -> Result<Buffer> {
    let bytes = fs::read(path.clone())
        .await
        .or_throw_msg(&ctx, &format!("Can't read \"{}\"", &path))?;
    Ok(Buffer(bytes))
}
