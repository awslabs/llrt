use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};

pub(crate) fn rename_error(from: &str, to: &str) -> String {
    ["Can't rename file/folder from \"", from, "\" to \"", to, "\""].concat()
}

pub async fn rename(ctx: Ctx<'_>, old_path: String, new_path: String) -> Result<()> {
    tokio::fs::rename(&old_path, &new_path)
        .await
        .or_throw_msg(&ctx, &rename_error(&old_path, &new_path))?;
    Ok(())
}

pub fn rename_sync(ctx: Ctx<'_>, old_path: String, new_path: String) -> Result<()> {
    std::fs::rename(&old_path, &new_path)
        .or_throw_msg(&ctx, &rename_error(&old_path, &new_path))?;
    Ok(())
}
