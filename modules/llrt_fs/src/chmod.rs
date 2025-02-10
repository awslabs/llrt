#[cfg(unix)]
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

pub async fn chmod(ctx: Ctx<'_>, path: String, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        tokio::fs::set_permissions(&path, PermissionsExt::from_mode(mode))
            .await
            .or_throw_msg(&ctx, &["Can't set permissions of \"", &path, "\""].concat())?;
    }
    #[cfg(not(unix))]
    {
        _ = ctx;
        _ = path;
        _ = mode;
    }
    Ok(())
}

pub fn chmod_sync(ctx: Ctx<'_>, path: String, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::fs::set_permissions(&path, PermissionsExt::from_mode(mode))
            .or_throw_msg(&ctx, &["Can't set permissions of \"", &path, "\""].concat())?;
    }
    #[cfg(not(unix))]
    {
        _ = ctx;
        _ = path;
        _ = mode;
    }
    Ok(())
}
