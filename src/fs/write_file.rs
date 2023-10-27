use rquickjs::{Ctx, Result, Value};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::util::{get_bytes, ResultExt};

pub async fn write_file<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    let mut file = fs::File::create(&path)
        .await
        .or_throw_msg(&ctx, &format!("Can't create file \"{}\"", &path))?;

    let bytes = get_bytes(&ctx, data)?;
    file.write_all(&bytes)
        .await
        .or_throw_msg(&ctx, &format!("Can't write \"{}\"", &path))?;
    Ok(())
}
