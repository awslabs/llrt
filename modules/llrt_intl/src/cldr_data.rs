// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Baked CLDR locale data for date/time formatting.
//!
//! This module contains pre-extracted patterns from the Unicode CLDR project
//! for a subset of common locales, enabling locale-aware date/time formatting
//! without requiring the full ICU library.
//!
//! Data sourced from: https://github.com/unicode-org/cldr-json

/// Locale-specific date/time formatting data
#[derive(Debug, Clone)]
pub struct LocaleData {
    /// Date format patterns (full, long, medium, short)
    pub date_formats: DateFormats,
    /// Time format patterns (full, long, medium, short)
    pub time_formats: TimeFormats,
    /// Pattern for combining date and time (e.g., "{1}, {0}")
    pub datetime_pattern: &'static str,
    /// Month names (wide format, 1-indexed internally but stored 0-indexed)
    pub months_wide: [&'static str; 12],
    /// Month names (abbreviated format)
    pub months_abbr: [&'static str; 12],
    /// Weekday names (wide format, 0=Sunday)
    pub days_wide: [&'static str; 7],
    /// Weekday names (abbreviated format)
    pub days_abbr: [&'static str; 7],
    /// AM marker
    pub am: &'static str,
    /// PM marker
    pub pm: &'static str,
    /// Whether this locale uses 12-hour time by default
    pub hour12_default: bool,
}

/// Date format patterns for different styles
#[derive(Debug, Clone)]
pub struct DateFormats {
    pub full: &'static str,
    pub long: &'static str,
    pub medium: &'static str,
    pub short: &'static str,
}

/// Time format patterns for different styles
#[derive(Debug, Clone)]
pub struct TimeFormats {
    pub full: &'static str,
    pub long: &'static str,
    pub medium: &'static str,
    pub short: &'static str,
}

/// Get locale data for a given locale string.
/// Falls back to en-US for unknown locales.
pub fn get_locale_data(locale: &str) -> &'static LocaleData {
    // Normalize locale: lowercase, handle both - and _
    let locale_lower = locale.to_lowercase().replace('_', "-");

    // Try exact match first, then language-only fallback
    match locale_lower.as_str() {
        "en-us" | "en" => &EN_US,
        "en-gb" | "en-au" | "en-nz" | "en-ie" => &EN_GB,
        "de" | "de-de" | "de-at" | "de-ch" => &DE,
        "fr" | "fr-fr" | "fr-ca" | "fr-be" | "fr-ch" => &FR,
        "es" | "es-es" | "es-mx" | "es-ar" => &ES,
        "it" | "it-it" => &IT,
        "pt" | "pt-pt" | "pt-br" => &PT,
        "nl" | "nl-nl" | "nl-be" => &NL,
        "ru" | "ru-ru" => &RU,
        "ja" | "ja-jp" => &JA,
        "ko" | "ko-kr" => &KO,
        "zh" | "zh-cn" | "zh-hans" => &ZH,
        "zh-tw" | "zh-hant" | "zh-hk" => &ZH_TW,
        "ar" | "ar-sa" | "ar-eg" => &AR,
        // High priority locales
        "hi" | "hi-in" => &HI,
        "bn" | "bn-bd" | "bn-in" => &BN,
        "vi" | "vi-vn" => &VI,
        "th" | "th-th" => &TH,
        "id" | "id-id" => &ID,
        "tr" | "tr-tr" => &TR,
        "pl" | "pl-pl" => &PL,
        "uk" | "uk-ua" => &UK,
        // Medium priority locales
        "sv" | "sv-se" => &SV,
        "da" | "da-dk" => &DA,
        "nb" | "nb-no" | "no" | "nn" | "nn-no" => &NB,
        "fi" | "fi-fi" => &FI,
        "cs" | "cs-cz" => &CS,
        "el" | "el-gr" => &EL,
        "he" | "he-il" | "iw" => &HE,
        "hu" | "hu-hu" => &HU,
        "ro" | "ro-ro" => &RO,
        // Fallback to en-US
        _ => {
            // Try to match just the language part
            if let Some(lang) = locale_lower.split('-').next() {
                match lang {
                    "en" => &EN_US,
                    "de" => &DE,
                    "fr" => &FR,
                    "es" => &ES,
                    "it" => &IT,
                    "pt" => &PT,
                    "nl" => &NL,
                    "ru" => &RU,
                    "ja" => &JA,
                    "ko" => &KO,
                    "zh" => &ZH,
                    "ar" => &AR,
                    "hi" => &HI,
                    "bn" => &BN,
                    "vi" => &VI,
                    "th" => &TH,
                    "id" => &ID,
                    "tr" => &TR,
                    "pl" => &PL,
                    "uk" => &UK,
                    "sv" => &SV,
                    "da" => &DA,
                    "nb" | "no" | "nn" => &NB,
                    "fi" => &FI,
                    "cs" => &CS,
                    "el" => &EL,
                    "he" | "iw" => &HE,
                    "hu" => &HU,
                    "ro" => &RO,
                    _ => &EN_US,
                }
            } else {
                &EN_US
            }
        },
    }
}

// English (US) - en-US
static EN_US: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, MMMM d, y",
        long: "MMMM d, y",
        medium: "MMM d, y",
        short: "M/d/yy",
    },
    time_formats: TimeFormats {
        full: "h:mm:ss a zzzz",
        long: "h:mm:ss a z",
        medium: "h:mm:ss a",
        short: "h:mm a",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    months_abbr: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    days_wide: [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ],
    days_abbr: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    am: "AM",
    pm: "PM",
    hour12_default: true,
};

// English (GB) - en-GB
static EN_GB: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd/MM/y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    months_abbr: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    days_wide: [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ],
    days_abbr: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    am: "am",
    pm: "pm",
    hour12_default: false,
};

// German - de
static DE: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d. MMMM y",
        long: "d. MMMM y",
        medium: "dd.MM.y",
        short: "dd.MM.yy",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ],
    months_abbr: [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sep.", "Okt.", "Nov.",
        "Dez.",
    ],
    days_wide: [
        "Sonntag",
        "Montag",
        "Dienstag",
        "Mittwoch",
        "Donnerstag",
        "Freitag",
        "Samstag",
    ],
    days_abbr: ["So.", "Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa."],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// French - fr
static FR: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd/MM/y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    months_abbr: [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ],
    days_wide: [
        "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
    ],
    days_abbr: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Spanish - es
static ES: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d 'de' MMMM 'de' y",
        long: "d 'de' MMMM 'de' y",
        medium: "d MMM y",
        short: "d/M/yy",
    },
    time_formats: TimeFormats {
        full: "H:mm:ss zzzz",
        long: "H:mm:ss z",
        medium: "H:mm:ss",
        short: "H:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    months_abbr: [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
    ],
    days_wide: [
        "domingo",
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
    ],
    days_abbr: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
    am: "a.\u{a0}m.",
    pm: "p.\u{a0}m.",
    hour12_default: false,
};

// Italian - it
static IT: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd/MM/yy",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    months_abbr: [
        "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
    ],
    days_wide: [
        "domenica",
        "lunedì",
        "martedì",
        "mercoledì",
        "giovedì",
        "venerdì",
        "sabato",
    ],
    days_abbr: ["dom", "lun", "mar", "mer", "gio", "ven", "sab"],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Portuguese - pt
static PT: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d 'de' MMMM 'de' y",
        long: "d 'de' MMMM 'de' y",
        medium: "d 'de' MMM 'de' y",
        short: "dd/MM/y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ],
    months_abbr: [
        "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
    ],
    days_wide: [
        "domingo",
        "segunda-feira",
        "terça-feira",
        "quarta-feira",
        "quinta-feira",
        "sexta-feira",
        "sábado",
    ],
    days_abbr: ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Dutch - nl
static NL: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd-MM-y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "januari",
        "februari",
        "maart",
        "april",
        "mei",
        "juni",
        "juli",
        "augustus",
        "september",
        "oktober",
        "november",
        "december",
    ],
    months_abbr: [
        "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec",
    ],
    days_wide: [
        "zondag",
        "maandag",
        "dinsdag",
        "woensdag",
        "donderdag",
        "vrijdag",
        "zaterdag",
    ],
    days_abbr: ["zo", "ma", "di", "wo", "do", "vr", "za"],
    am: "a.m.",
    pm: "p.m.",
    hour12_default: false,
};

// Russian - ru
static RU: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y 'г'.",
        long: "d MMMM y 'г'.",
        medium: "d MMM y 'г'.",
        short: "dd.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "января",
        "февраля",
        "марта",
        "апреля",
        "мая",
        "июня",
        "июля",
        "августа",
        "сентября",
        "октября",
        "ноября",
        "декабря",
    ],
    months_abbr: [
        "янв.",
        "февр.",
        "мар.",
        "апр.",
        "мая",
        "июн.",
        "июл.",
        "авг.",
        "сент.",
        "окт.",
        "нояб.",
        "дек.",
    ],
    days_wide: [
        "воскресенье",
        "понедельник",
        "вторник",
        "среда",
        "четверг",
        "пятница",
        "суббота",
    ],
    days_abbr: ["вс", "пн", "вт", "ср", "чт", "пт", "сб"],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Japanese - ja
static JA: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "y年M月d日EEEE",
        long: "y年M月d日",
        medium: "y/MM/dd",
        short: "y/MM/dd",
    },
    time_formats: TimeFormats {
        full: "H時mm分ss秒 zzzz",
        long: "H:mm:ss z",
        medium: "H:mm:ss",
        short: "H:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    months_abbr: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    days_wide: [
        "日曜日",
        "月曜日",
        "火曜日",
        "水曜日",
        "木曜日",
        "金曜日",
        "土曜日",
    ],
    days_abbr: ["日", "月", "火", "水", "木", "金", "土"],
    am: "午前",
    pm: "午後",
    hour12_default: false,
};

// Korean - ko
static KO: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "y년 MMMM d일 EEEE",
        long: "y년 MMMM d일",
        medium: "y. M. d.",
        short: "yy. M. d.",
    },
    time_formats: TimeFormats {
        full: "a h시 m분 s초 zzzz",
        long: "a h시 m분 s초 z",
        medium: "a h:mm:ss",
        short: "a h:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
    ],
    months_abbr: [
        "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
    ],
    days_wide: [
        "일요일",
        "월요일",
        "화요일",
        "수요일",
        "목요일",
        "금요일",
        "토요일",
    ],
    days_abbr: ["일", "월", "화", "수", "목", "금", "토"],
    am: "오전",
    pm: "오후",
    hour12_default: true,
};

// Chinese (Simplified) - zh
static ZH: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "y年M月d日EEEE",
        long: "y年M月d日",
        medium: "y年M月d日",
        short: "y/M/d",
    },
    time_formats: TimeFormats {
        full: "zzzz HH:mm:ss",
        long: "z HH:mm:ss",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ],
    months_abbr: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    days_wide: [
        "星期日",
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
    ],
    days_abbr: ["周日", "周一", "周二", "周三", "周四", "周五", "周六"],
    am: "上午",
    pm: "下午",
    hour12_default: false,
};

// Chinese (Traditional) - zh-TW
static ZH_TW: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "y年M月d日 EEEE",
        long: "y年M月d日",
        medium: "y年M月d日",
        short: "y/M/d",
    },
    time_formats: TimeFormats {
        full: "ah:mm:ss [zzzz]",
        long: "ah:mm:ss [z]",
        medium: "ah:mm:ss",
        short: "ah:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    months_abbr: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    days_wide: [
        "星期日",
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
    ],
    days_abbr: ["週日", "週一", "週二", "週三", "週四", "週五", "週六"],
    am: "上午",
    pm: "下午",
    hour12_default: true,
};

// Arabic - ar
static AR: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE، d MMMM y",
        long: "d MMMM y",
        medium: "dd/MM/y",
        short: "d/M/y",
    },
    time_formats: TimeFormats {
        full: "h:mm:ss a zzzz",
        long: "h:mm:ss a z",
        medium: "h:mm:ss a",
        short: "h:mm a",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "يناير",
        "فبراير",
        "مارس",
        "أبريل",
        "مايو",
        "يونيو",
        "يوليو",
        "أغسطس",
        "سبتمبر",
        "أكتوبر",
        "نوفمبر",
        "ديسمبر",
    ],
    months_abbr: [
        "يناير",
        "فبراير",
        "مارس",
        "أبريل",
        "مايو",
        "يونيو",
        "يوليو",
        "أغسطس",
        "سبتمبر",
        "أكتوبر",
        "نوفمبر",
        "ديسمبر",
    ],
    days_wide: [
        "الأحد",
        "الاثنين",
        "الثلاثاء",
        "الأربعاء",
        "الخميس",
        "الجمعة",
        "السبت",
    ],
    days_abbr: [
        "الأحد",
        "الاثنين",
        "الثلاثاء",
        "الأربعاء",
        "الخميس",
        "الجمعة",
        "السبت",
    ],
    am: "ص",
    pm: "م",
    hour12_default: true,
};

// Hindi - hi
static HI: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "d/M/yy",
    },
    time_formats: TimeFormats {
        full: "h:mm:ss a zzzz",
        long: "h:mm:ss a z",
        medium: "h:mm:ss a",
        short: "h:mm a",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "जनवरी",
        "फ़रवरी",
        "मार्च",
        "अप्रैल",
        "मई",
        "जून",
        "जुलाई",
        "अगस्त",
        "सितंबर",
        "अक्तूबर",
        "नवंबर",
        "दिसंबर",
    ],
    months_abbr: [
        "जन॰",
        "फ़र॰",
        "मार्च",
        "अप्रैल",
        "मई",
        "जून",
        "जुल॰",
        "अग॰",
        "सित॰",
        "अक्तू॰",
        "नव॰",
        "दिस॰",
    ],
    days_wide: [
        "रविवार",
        "सोमवार",
        "मंगलवार",
        "बुधवार",
        "गुरुवार",
        "शुक्रवार",
        "शनिवार",
    ],
    days_abbr: ["रवि", "सोम", "मंगल", "बुध", "गुरु", "शुक्र", "शनि"],
    am: "am",
    pm: "pm",
    hour12_default: true,
};

// Bengali - bn
static BN: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM, y",
        long: "d MMMM, y",
        medium: "d MMM, y",
        short: "d/M/yy",
    },
    time_formats: TimeFormats {
        full: "h:mm:ss a zzzz",
        long: "h:mm:ss a z",
        medium: "h:mm:ss a",
        short: "h:mm a",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "জানুয়ারী",
        "ফেব্রুয়ারী",
        "মার্চ",
        "এপ্রিল",
        "মে",
        "জুন",
        "জুলাই",
        "আগস্ট",
        "সেপ্টেম্বর",
        "অক্টোবর",
        "নভেম্বর",
        "ডিসেম্বর",
    ],
    months_abbr: [
        "জানু",
        "ফেব",
        "মার্চ",
        "এপ্রি",
        "মে",
        "জুন",
        "জুলাই",
        "আগ",
        "সেপ",
        "অক্টো",
        "নভে",
        "ডিসে",
    ],
    days_wide: [
        "রবিবার",
        "সোমবার",
        "মঙ্গলবার",
        "বুধবার",
        "বৃহস্পতিবার",
        "শুক্রবার",
        "শনিবার",
    ],
    days_abbr: ["রবি", "সোম", "মঙ্গল", "বুধ", "বৃহস্পতি", "শুক্র", "শনি"],
    am: "AM",
    pm: "PM",
    hour12_default: true,
};

// Vietnamese - vi
static VI: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM, y",
        long: "d MMMM, y",
        medium: "d MMM, y",
        short: "dd/MM/y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "tháng 1",
        "tháng 2",
        "tháng 3",
        "tháng 4",
        "tháng 5",
        "tháng 6",
        "tháng 7",
        "tháng 8",
        "tháng 9",
        "tháng 10",
        "tháng 11",
        "tháng 12",
    ],
    months_abbr: [
        "thg 1", "thg 2", "thg 3", "thg 4", "thg 5", "thg 6", "thg 7", "thg 8", "thg 9", "thg 10",
        "thg 11", "thg 12",
    ],
    days_wide: [
        "Chủ Nhật",
        "Thứ Hai",
        "Thứ Ba",
        "Thứ Tư",
        "Thứ Năm",
        "Thứ Sáu",
        "Thứ Bảy",
    ],
    days_abbr: ["CN", "Th 2", "Th 3", "Th 4", "Th 5", "Th 6", "Th 7"],
    am: "SA",
    pm: "CH",
    hour12_default: false,
};

// Thai - th
static TH: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEEที่ d MMMM G y",
        long: "d MMMM G y",
        medium: "d MMM y",
        short: "d/M/yy",
    },
    time_formats: TimeFormats {
        full: "H นาฬิกา mm นาที ss วินาที zzzz",
        long: "H นาฬิกา mm นาที ss วินาที z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "มกราคม",
        "กุมภาพันธ์",
        "มีนาคม",
        "เมษายน",
        "พฤษภาคม",
        "มิถุนายน",
        "กรกฎาคม",
        "สิงหาคม",
        "กันยายน",
        "ตุลาคม",
        "พฤศจิกายน",
        "ธันวาคม",
    ],
    months_abbr: [
        "ม.ค.",
        "ก.พ.",
        "มี.ค.",
        "เม.ย.",
        "พ.ค.",
        "มิ.ย.",
        "ก.ค.",
        "ส.ค.",
        "ก.ย.",
        "ต.ค.",
        "พ.ย.",
        "ธ.ค.",
    ],
    days_wide: [
        "วันอาทิตย์",
        "วันจันทร์",
        "วันอังคาร",
        "วันพุธ",
        "วันพฤหัสบดี",
        "วันศุกร์",
        "วันเสาร์",
    ],
    days_abbr: ["อา.", "จ.", "อ.", "พ.", "พฤ.", "ศ.", "ส."],
    am: "ก่อนเที่ยง",
    pm: "หลังเที่ยง",
    hour12_default: false,
};

// Indonesian - id
static ID: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, dd MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd/MM/yy",
    },
    time_formats: TimeFormats {
        full: "HH.mm.ss zzzz",
        long: "HH.mm.ss z",
        medium: "HH.mm.ss",
        short: "HH.mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ],
    months_abbr: [
        "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
    ],
    days_wide: [
        "Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu",
    ],
    days_abbr: ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Turkish - tr
static TR: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "d MMMM y EEEE",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "d.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ],
    months_abbr: [
        "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
    ],
    days_wide: [
        "Pazar",
        "Pazartesi",
        "Salı",
        "Çarşamba",
        "Perşembe",
        "Cuma",
        "Cumartesi",
    ],
    days_abbr: ["Paz", "Pzt", "Sal", "Çar", "Per", "Cum", "Cmt"],
    am: "ÖÖ",
    pm: "ÖS",
    hour12_default: false,
};

// Polish - pl
static PL: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "stycznia",
        "lutego",
        "marca",
        "kwietnia",
        "maja",
        "czerwca",
        "lipca",
        "sierpnia",
        "września",
        "października",
        "listopada",
        "grudnia",
    ],
    months_abbr: [
        "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
    ],
    days_wide: [
        "niedziela",
        "poniedziałek",
        "wtorek",
        "środa",
        "czwartek",
        "piątek",
        "sobota",
    ],
    days_abbr: ["niedz.", "pon.", "wt.", "śr.", "czw.", "pt.", "sob."],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Ukrainian - uk
static UK: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y 'р'.",
        long: "d MMMM y 'р'.",
        medium: "d MMM y 'р'.",
        short: "dd.MM.yy",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "січня",
        "лютого",
        "березня",
        "квітня",
        "травня",
        "червня",
        "липня",
        "серпня",
        "вересня",
        "жовтня",
        "листопада",
        "грудня",
    ],
    months_abbr: [
        "січ.",
        "лют.",
        "бер.",
        "квіт.",
        "трав.",
        "черв.",
        "лип.",
        "серп.",
        "вер.",
        "жовт.",
        "лист.",
        "груд.",
    ],
    days_wide: [
        "неділя",
        "понеділок",
        "вівторок",
        "середа",
        "четвер",
        "пʼятниця",
        "субота",
    ],
    days_abbr: ["нд", "пн", "вт", "ср", "чт", "пт", "сб"],
    am: "дп",
    pm: "пп",
    hour12_default: false,
};

// Swedish - sv
static SV: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "y-MM-dd",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "januari",
        "februari",
        "mars",
        "april",
        "maj",
        "juni",
        "juli",
        "augusti",
        "september",
        "oktober",
        "november",
        "december",
    ],
    months_abbr: [
        "jan.", "feb.", "mars", "apr.", "maj", "juni", "juli", "aug.", "sep.", "okt.", "nov.",
        "dec.",
    ],
    days_wide: [
        "söndag", "måndag", "tisdag", "onsdag", "torsdag", "fredag", "lördag",
    ],
    days_abbr: ["sön", "mån", "tis", "ons", "tors", "fre", "lör"],
    am: "fm",
    pm: "em",
    hour12_default: false,
};

// Danish - da
static DA: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE 'den' d. MMMM y",
        long: "d. MMMM y",
        medium: "d. MMM y",
        short: "dd.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH.mm.ss zzzz",
        long: "HH.mm.ss z",
        medium: "HH.mm.ss",
        short: "HH.mm",
    },
    datetime_pattern: "{1} 'kl'. {0}",
    months_wide: [
        "januar",
        "februar",
        "marts",
        "april",
        "maj",
        "juni",
        "juli",
        "august",
        "september",
        "oktober",
        "november",
        "december",
    ],
    months_abbr: [
        "jan.", "feb.", "mar.", "apr.", "maj", "jun.", "jul.", "aug.", "sep.", "okt.", "nov.",
        "dec.",
    ],
    days_wide: [
        "søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag",
    ],
    days_abbr: ["søn.", "man.", "tir.", "ons.", "tor.", "fre.", "lør."],
    am: "AM",
    pm: "PM",
    hour12_default: false,
};

// Norwegian Bokmål - nb
static NB: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d. MMMM y",
        long: "d. MMMM y",
        medium: "d. MMM y",
        short: "dd.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "januar",
        "februar",
        "mars",
        "april",
        "mai",
        "juni",
        "juli",
        "august",
        "september",
        "oktober",
        "november",
        "desember",
    ],
    months_abbr: [
        "jan.", "feb.", "mar.", "apr.", "mai", "jun.", "jul.", "aug.", "sep.", "okt.", "nov.",
        "des.",
    ],
    days_wide: [
        "søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag",
    ],
    days_abbr: ["søn.", "man.", "tir.", "ons.", "tor.", "fre.", "lør."],
    am: "a.m.",
    pm: "p.m.",
    hour12_default: false,
};

// Finnish - fi
static FI: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "cccc d. MMMM y",
        long: "d. MMMM y",
        medium: "d.M.y",
        short: "d.M.y",
    },
    time_formats: TimeFormats {
        full: "H.mm.ss zzzz",
        long: "H.mm.ss z",
        medium: "H.mm.ss",
        short: "H.mm",
    },
    datetime_pattern: "{1} 'klo' {0}",
    months_wide: [
        "tammikuuta",
        "helmikuuta",
        "maaliskuuta",
        "huhtikuuta",
        "toukokuuta",
        "kesäkuuta",
        "heinäkuuta",
        "elokuuta",
        "syyskuuta",
        "lokakuuta",
        "marraskuuta",
        "joulukuuta",
    ],
    months_abbr: [
        "tammik.", "helmik.", "maalisk.", "huhtik.", "toukok.", "kesäk.", "heinäk.", "elok.",
        "syysk.", "lokak.", "marrask.", "jouluk.",
    ],
    days_wide: [
        "sunnuntaina",
        "maanantaina",
        "tiistaina",
        "keskiviikkona",
        "torstaina",
        "perjantaina",
        "lauantaina",
    ],
    days_abbr: ["su", "ma", "ti", "ke", "to", "pe", "la"],
    am: "ap.",
    pm: "ip.",
    hour12_default: false,
};

// Czech - cs
static CS: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d. MMMM y",
        long: "d. MMMM y",
        medium: "d. M. y",
        short: "dd.MM.yy",
    },
    time_formats: TimeFormats {
        full: "H:mm:ss zzzz",
        long: "H:mm:ss z",
        medium: "H:mm:ss",
        short: "H:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "ledna",
        "února",
        "března",
        "dubna",
        "května",
        "června",
        "července",
        "srpna",
        "září",
        "října",
        "listopadu",
        "prosince",
    ],
    months_abbr: [
        "led", "úno", "bře", "dub", "kvě", "čvn", "čvc", "srp", "zář", "říj", "lis", "pro",
    ],
    days_wide: [
        "neděle",
        "pondělí",
        "úterý",
        "středa",
        "čtvrtek",
        "pátek",
        "sobota",
    ],
    days_abbr: ["ne", "po", "út", "st", "čt", "pá", "so"],
    am: "dop.",
    pm: "odp.",
    hour12_default: false,
};

// Greek - el
static EL: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "d/M/yy",
    },
    time_formats: TimeFormats {
        full: "h:mm:ss a zzzz",
        long: "h:mm:ss a z",
        medium: "h:mm:ss a",
        short: "h:mm a",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "Ιανουαρίου",
        "Φεβρουαρίου",
        "Μαρτίου",
        "Απριλίου",
        "Μαΐου",
        "Ιουνίου",
        "Ιουλίου",
        "Αυγούστου",
        "Σεπτεμβρίου",
        "Οκτωβρίου",
        "Νοεμβρίου",
        "Δεκεμβρίου",
    ],
    months_abbr: [
        "Ιαν", "Φεβ", "Μαρ", "Απρ", "Μαΐ", "Ιουν", "Ιουλ", "Αυγ", "Σεπ", "Οκτ", "Νοε", "Δεκ",
    ],
    days_wide: [
        "Κυριακή",
        "Δευτέρα",
        "Τρίτη",
        "Τετάρτη",
        "Πέμπτη",
        "Παρασκευή",
        "Σάββατο",
    ],
    days_abbr: ["Κυρ", "Δευ", "Τρί", "Τετ", "Πέμ", "Παρ", "Σάβ"],
    am: "π.μ.",
    pm: "μ.μ.",
    hour12_default: true,
};

// Hebrew - he
static HE: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d בMMMM y",
        long: "d בMMMM y",
        medium: "d בMMM y",
        short: "d.M.y",
    },
    time_formats: TimeFormats {
        full: "H:mm:ss zzzz",
        long: "H:mm:ss z",
        medium: "H:mm:ss",
        short: "H:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "ינואר",
        "פברואר",
        "מרץ",
        "אפריל",
        "מאי",
        "יוני",
        "יולי",
        "אוגוסט",
        "ספטמבר",
        "אוקטובר",
        "נובמבר",
        "דצמבר",
    ],
    months_abbr: [
        "ינו׳", "פבר׳", "מרץ", "אפר׳", "מאי", "יוני", "יולי", "אוג׳", "ספט׳", "אוק׳", "נוב׳",
        "דצמ׳",
    ],
    days_wide: [
        "יום ראשון",
        "יום שני",
        "יום שלישי",
        "יום רביעי",
        "יום חמישי",
        "יום שישי",
        "יום שבת",
    ],
    days_abbr: [
        "יום א׳",
        "יום ב׳",
        "יום ג׳",
        "יום ד׳",
        "יום ה׳",
        "יום ו׳",
        "שבת",
    ],
    am: "לפנה״צ",
    pm: "אחה״צ",
    hour12_default: false,
};

// Hungarian - hu
static HU: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "y. MMMM d., EEEE",
        long: "y. MMMM d.",
        medium: "y. MMM d.",
        short: "y. MM. dd.",
    },
    time_formats: TimeFormats {
        full: "H:mm:ss zzzz",
        long: "H:mm:ss z",
        medium: "H:mm:ss",
        short: "H:mm",
    },
    datetime_pattern: "{1} {0}",
    months_wide: [
        "január",
        "február",
        "március",
        "április",
        "május",
        "június",
        "július",
        "augusztus",
        "szeptember",
        "október",
        "november",
        "december",
    ],
    months_abbr: [
        "jan.", "febr.", "márc.", "ápr.", "máj.", "jún.", "júl.", "aug.", "szept.", "okt.", "nov.",
        "dec.",
    ],
    days_wide: [
        "vasárnap",
        "hétfő",
        "kedd",
        "szerda",
        "csütörtök",
        "péntek",
        "szombat",
    ],
    days_abbr: ["V", "H", "K", "Sze", "Cs", "P", "Szo"],
    am: "de.",
    pm: "du.",
    hour12_default: false,
};

// Romanian - ro
static RO: LocaleData = LocaleData {
    date_formats: DateFormats {
        full: "EEEE, d MMMM y",
        long: "d MMMM y",
        medium: "d MMM y",
        short: "dd.MM.y",
    },
    time_formats: TimeFormats {
        full: "HH:mm:ss zzzz",
        long: "HH:mm:ss z",
        medium: "HH:mm:ss",
        short: "HH:mm",
    },
    datetime_pattern: "{1}, {0}",
    months_wide: [
        "ianuarie",
        "februarie",
        "martie",
        "aprilie",
        "mai",
        "iunie",
        "iulie",
        "august",
        "septembrie",
        "octombrie",
        "noiembrie",
        "decembrie",
    ],
    months_abbr: [
        "ian.", "feb.", "mar.", "apr.", "mai", "iun.", "iul.", "aug.", "sept.", "oct.", "nov.",
        "dec.",
    ],
    days_wide: [
        "duminică",
        "luni",
        "marți",
        "miercuri",
        "joi",
        "vineri",
        "sâmbătă",
    ],
    days_abbr: ["dum.", "lun.", "mar.", "mie.", "joi", "vin.", "sâm."],
    am: "a.m.",
    pm: "p.m.",
    hour12_default: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_locale_data_exact_match() {
        let data = get_locale_data("en-US");
        assert_eq!(data.date_formats.short, "M/d/yy");
        assert!(data.hour12_default);

        let data = get_locale_data("de-DE");
        assert_eq!(data.date_formats.short, "dd.MM.yy");
        assert!(!data.hour12_default);
    }

    #[test]
    fn test_get_locale_data_language_fallback() {
        let data = get_locale_data("de");
        assert_eq!(data.date_formats.short, "dd.MM.yy");

        let data = get_locale_data("fr");
        assert_eq!(data.date_formats.short, "dd/MM/y");
    }

    #[test]
    fn test_get_locale_data_unknown_fallback() {
        let data = get_locale_data("xx-YY");
        // Should fall back to en-US
        assert_eq!(data.date_formats.short, "M/d/yy");
    }

    #[test]
    fn test_get_locale_data_case_insensitive() {
        let data1 = get_locale_data("en-US");
        let data2 = get_locale_data("EN-US");
        let data3 = get_locale_data("En-Us");
        assert_eq!(data1.date_formats.short, data2.date_formats.short);
        assert_eq!(data2.date_formats.short, data3.date_formats.short);
    }

    #[test]
    fn test_get_locale_data_underscore() {
        let data = get_locale_data("en_US");
        assert_eq!(data.date_formats.short, "M/d/yy");
    }

    #[test]
    fn test_months_count() {
        let data = get_locale_data("en-US");
        assert_eq!(data.months_wide.len(), 12);
        assert_eq!(data.months_abbr.len(), 12);
    }

    #[test]
    fn test_days_count() {
        let data = get_locale_data("en-US");
        assert_eq!(data.days_wide.len(), 7);
        assert_eq!(data.days_abbr.len(), 7);
    }
}
