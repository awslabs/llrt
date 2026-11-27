// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{
    civil::DateTime, fmt::temporal::DateTimeParser, SpanRelativeTo, ZonedDateTime as Zoned,
};
use llrt_utils::result::ResultExt;
use rquickjs::{Class, Ctx, FromJs, Result, Value};

use crate::plain_date::PlainDate;
use crate::plain_date_time::PlainDateTime;
use crate::zoned_date_time::ZonedDateTime;

use super::calendar::{split_calendar_annotation, Calendar};
use super::zoned::from_object_with_time_zone;

pub(crate) enum RelativeTo {
    Civil(DateTime),
    Zoned(Zoned),
}

impl RelativeTo {
    pub(crate) fn from(ctx: &Ctx<'_>, value: Option<&Value<'_>>) -> Result<Option<Self>> {
        let Some(value) = value else {
            return Ok(None);
        };
        if value.is_undefined() {
            return Ok(None);
        }
        if let Some(obj) = value.as_object() {
            if let Some(cls) = Class::<PlainDate>::from_object(obj) {
                let pd = cls.borrow().clone();
                return Ok(Some(Self::Civil(pd.into_inner().into())));
            }
            if let Some(cls) = Class::<PlainDateTime>::from_object(obj) {
                let pdt = cls.borrow().clone();
                return Ok(Some(Self::Civil(pdt.into_inner())));
            }
            if let Some(cls) = Class::<ZonedDateTime>::from_object(obj) {
                let zdt = cls.borrow().clone();
                return Ok(Some(Self::Zoned(zdt.into_inner())));
            }

            let tz = obj.get::<_, Value>("timeZone")?;
            if !tz.is_undefined() {
                let tz = String::from_js(obj.ctx(), tz)?;
                let zoned = from_object_with_time_zone(ctx, obj, &tz)?;
                Calendar::from_object(ctx, obj)?;
                return Ok(Some(Self::Zoned(zoned)));
            }

            let date = PlainDate::from_object(ctx, obj)?;
            return Ok(Some(Self::Civil(date.into_inner().into())));
        }

        let str = value
            .as_string()
            .and_then(|s| s.to_string().ok())
            .or_throw_type(ctx, "Invalid relativeTo")?;

        let (without_calendar, _) = split_calendar_annotation(&str);
        let pieces = DateTimeParser::new()
            .parse_pieces(&without_calendar)
            .or_throw_range(ctx, "")?;

        if pieces.time_zone_annotation().is_some() {
            let zoned = ZonedDateTime::from_str(ctx, &str)?;
            Ok(Some(Self::Zoned(zoned.into_inner())))
        } else {
            let date = PlainDate::from_str(ctx, &str)?;
            Ok(Some(Self::Civil(date.into_inner().into())))
        }
    }

    pub(crate) fn as_relative(&self) -> SpanRelativeTo<'_> {
        match self {
            Self::Civil(datetime) => (*datetime).into(),
            Self::Zoned(zoned) => zoned.into(),
        }
    }
}
