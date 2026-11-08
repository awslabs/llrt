// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{error::Error, fmt, str::FromStr};

use rquickjs::{Ctx, Object, Result};

use super::parse_temporal_calendar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarError {
    UnknownCalendar(String),
}
impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCalendar(id) => {
                write!(f, "Unknown calendar: {id}")
            },
        }
    }
}
impl Error for CalendarError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CalendarKind {
    Iso8601,
    Buddhist,
    Chinese,
    Coptic,
    Dangi,
    Ethiopian,
    EthiopianAmeteAlem,
    Gregorian,
    Hebrew,
    Indian,
    IslamicCivil,
    IslamicTabular,
    IslamicUmalqura,
    Japanese,
    Persian,
    Roc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Calendar {
    kind: CalendarKind,
}
impl FromStr for Calendar {
    type Err = CalendarError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = canonicalize_calendar_identifier(s);
        let kind = match s.as_str() {
            "iso8601" => CalendarKind::Iso8601,
            "buddhist" => CalendarKind::Buddhist,
            "chinese" => CalendarKind::Chinese,
            "coptic" => CalendarKind::Coptic,
            "dangi" => CalendarKind::Dangi,
            "ethiopic" => CalendarKind::Ethiopian,
            "ethioaa" => CalendarKind::EthiopianAmeteAlem,
            "gregory" => CalendarKind::Gregorian,
            "hebrew" => CalendarKind::Hebrew,
            "indian" => CalendarKind::Indian,
            "islamic-civil" => CalendarKind::IslamicCivil,
            "islamic-tbla" => CalendarKind::IslamicTabular,
            "islamic-umalqura" => CalendarKind::IslamicUmalqura,
            "japanese" => CalendarKind::Japanese,
            "persian" => CalendarKind::Persian,
            "roc" => CalendarKind::Roc,
            _ => return Err(CalendarError::UnknownCalendar(s)),
        };
        Ok(Self { kind })
    }
}
impl Calendar {
    pub const fn id(&self) -> &'static str {
        match self.kind {
            CalendarKind::Iso8601 => "iso8601",
            CalendarKind::Buddhist => "buddhist",
            CalendarKind::Chinese => "chinese",
            CalendarKind::Coptic => "coptic",
            CalendarKind::Dangi => "dangi",
            CalendarKind::Ethiopian => "ethiopic",
            CalendarKind::EthiopianAmeteAlem => "ethioaa",
            CalendarKind::Gregorian => "gregory",
            CalendarKind::Hebrew => "hebrew",
            CalendarKind::Indian => "indian",
            CalendarKind::IslamicCivil => "islamic-civil",
            CalendarKind::IslamicTabular => "islamic-tbla",
            CalendarKind::IslamicUmalqura => "islamic-umalqura",
            CalendarKind::Japanese => "japanese",
            CalendarKind::Persian => "persian",
            CalendarKind::Roc => "roc",
        }
    }

    pub fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Calendar> {
        let ca = obj.get::<_, String>("Calendar").ok();
        parse_temporal_calendar(ctx, ca)
    }
}

pub(crate) fn split_calendar_annotation(s: &str) -> (String, Option<String>) {
    if let Some(start) = s.rfind("[u-ca=") {
        if s.ends_with(']') {
            let jiff = s[..start].to_owned();
            let calendar = s[start + "[u-ca=".len()..s.len() - 1].to_owned();
            return (jiff, Some(calendar));
        }
    }
    (s.to_owned(), None)
}

pub(crate) fn canonicalize_calendar_identifier(id: &str) -> String {
    match id.trim().to_ascii_lowercase().as_str() {
        "gregorian" => "gregory".into(),
        "ethiopian" => "ethiopic".into(),
        "ethiopic-amete-alem" => "ethioaa".into(),
        "islamicc" => "islamic-civil".into(),
        id => id.into(),
    }
}
