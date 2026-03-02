// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    prelude::{Func, Opt},
    Ctx, Object, Result,
};

use crate::instant::Instant;
use crate::zoned_date_time::ZonedDateTime;

pub(crate) fn define_object<'a>(ctx: &Ctx<'a>) -> Result<Object<'a>> {
    let obj = Object::new(ctx.clone())?;
    obj.set("instant", Func::from(Instant::now))?;
    obj.set("zonedDateTimeISO", Func::from(zoned_datetime_iso))?;
    obj.set(PredefinedAtom::SymbolToStringTag, "Temporal.Now")?;
    Ok(obj)
}

fn zoned_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<ZonedDateTime> {
    let ts = Instant::now().into_inner();
    ZonedDateTime::from_timestamp(&ctx, &ts, &timezone)
}
