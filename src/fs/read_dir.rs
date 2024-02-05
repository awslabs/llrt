use std::fs::Metadata;

use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Class, Ctx, IntoJs, Object, Result, Value,
};
use tokio::fs;

use crate::utils::result::ResultExt;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Dirent {
    #[qjs(skip_trace)]
    metadata: Metadata,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Dirent {
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }
}

pub struct ReadDir {
    items: Vec<(String, Option<Metadata>)>,
}

impl<'js> IntoJs<'js> for ReadDir {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (index, (name, metadata)) in self.items.into_iter().enumerate() {
            if let Some(metadata) = metadata {
                let dirent = Dirent { metadata };

                let dirent = Class::instance(ctx.clone(), dirent)?;
                dirent.set(PredefinedAtom::Name, name)?;
                arr.set(index, dirent)?;
            } else {
                arr.set(index, name)?;
            }
        }
        arr.into_js(ctx)
    }
}

pub async fn read_dir<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<Object<'js>>,
) -> Result<ReadDir> {
    let with_file_types = options
        .0
        .and_then(|opts| opts.get("withFileTypes").ok())
        .and_then(|file_types: Value| file_types.as_bool())
        .unwrap_or_default();

    let mut dir = fs::read_dir(path).await.or_throw(&ctx)?;

    let mut items = Vec::with_capacity(64);

    while let Some(child) = dir.next_entry().await? {
        if let Some(name) = child.path().file_name() {
            let name = name.to_string_lossy().to_string();

            if with_file_types {
                let metadata = child.metadata().await?;
                items.push((name, Some(metadata)))
            } else {
                items.push((name, None))
            }
        }
    }

    items.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

    Ok(ReadDir { items })
}
