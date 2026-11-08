// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, error::Error, fmt, str::FromStr};

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

macro_rules! define_calendar_kinds {
    ($( $kind:ident => $id:literal, )+) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub(crate) enum CalendarKind {
            $( $kind, )+
        }
        impl CalendarKind {
            fn from_id(id: &str) -> Option<Self> {
                match id {
                    $( $id => Some(Self::$kind), )+
                    _ => None,
                }
            }
            const fn id(&self) -> &'static str {
                match self {
                    $( Self::$kind => $id, )+
                }
            }
        }
    };
}

define_calendar_kinds! {
    Iso8601 => "iso8601",
    Buddhist => "buddhist",
    Chinese => "chinese",
    Coptic => "coptic",
    Dangi => "dangi",
    Ethiopian => "ethiopic",
    EthiopianAmeteAlem => "ethioaa",
    Gregorian => "gregory",
    Hebrew => "hebrew",
    Indian => "indian",
    IslamicCivil => "islamic-civil",
    IslamicTabular => "islamic-tbla",
    IslamicUmalqura => "islamic-umalqura",
    Japanese => "japanese",
    Persian => "persian",
    Roc => "roc",
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Calendar {
    kind: CalendarKind,
}
impl FromStr for Calendar {
    type Err = CalendarError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = canonicalize_calendar_identifier(s);
        let kind = match CalendarKind::from_id(&s) {
            Some(kind) => kind,
            None => return Err(CalendarError::UnknownCalendar(s.into_owned())),
        };
        Ok(Self { kind })
    }
}
impl Calendar {
    pub const fn id(&self) -> &'static str {
        self.kind.id()
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

pub(crate) fn canonicalize_calendar_identifier<'a>(id: &'a str) -> Cow<'a, str> {
    let normalized = id.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "gregorian" => Cow::Borrowed("gregory"),
        "ethiopian" => Cow::Borrowed("ethiopic"),
        "ethiopic-amete-alem" => Cow::Borrowed("ethioaa"),
        "islamicc" => Cow::Borrowed("islamic-civil"),
        _ if normalized == id => Cow::Borrowed(id),
        _ => Cow::Owned(normalized),
    }
}
