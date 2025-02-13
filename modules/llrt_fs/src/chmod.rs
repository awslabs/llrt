#[cfg(unix)]
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

#[cfg(unix)]
pub(crate) fn chmod_error(path: &str) -> String {
    ["Can't set permissions of \"", path, "\""].concat()
}

pub(crate) async fn set_mode(ctx: Ctx<'_>, path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        tokio::fs::set_permissions(path, PermissionsExt::from_mode(mode))
            .await
            .or_throw_msg(&ctx, &chmod_error(path))?;
    }
    #[cfg(not(unix))]
    {
        _ = ctx;
        _ = path;
        _ = mode;
    }
    Ok(())
}

pub(crate) fn set_mode_sync(ctx: Ctx<'_>, path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, PermissionsExt::from_mode(mode))
            .or_throw_msg(&ctx, &chmod_error(path))?;
    }
    #[cfg(not(unix))]
    {
        _ = ctx;
        _ = path;
        _ = mode;
    }
    Ok(())
}

pub async fn chmod(ctx: Ctx<'_>, path: String, mode: u32) -> Result<()> {
    set_mode(ctx, &path, mode).await
}

pub fn chmod_sync(ctx: Ctx<'_>, path: String, mode: u32) -> Result<()> {
    set_mode_sync(ctx, &path, mode)
}
