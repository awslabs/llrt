use rquickjs::{function::Opt, Ctx, Object, Result};
use tokio::fs;

use crate::utils::result::ResultExt;

#[allow(clippy::manual_async_fn)]
pub async fn rmdir<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<()> {
    let mut recursive = false;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
    }

    if recursive {
        fs::remove_dir_all(&path).await
    } else {
        fs::remove_dir(&path).await
    }
    .or_throw_msg(&ctx, &format!("Can't remove dir \"{}\"", &path))?;

    Ok(())
}

pub async fn rmfile<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<()> {
    let mut recursive = false;
    let mut force = false;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
        force = options.get("force").unwrap_or_default();
    }

    let res = async move {
        let is_dir = fs::metadata(&path)
            .await
            .map(|metadata| metadata.is_dir())
            .or_throw(&ctx)?;

        (if is_dir && recursive {
            fs::remove_dir_all(&path).await
        } else if is_dir && !recursive {
            fs::remove_dir(&path).await
        } else {
            fs::remove_file(&path).await
        })
        .or_throw_msg(&ctx, &format!("Can't remove file \"{}\"", &path))?;

        Ok(())
    }
    .await;

    if !force {
        return res;
    }

    Ok(())
}
