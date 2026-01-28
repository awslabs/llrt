// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exhaustive comparison tests between chrono-tz and our compact implementation.
//! This generates HUNDREDS of individual tests to ensure complete parity.

use chrono::{DateTime, NaiveDate, Offset};
use chrono_tz::Tz as ChronoTz;
use llrt_tz::Tz as CompactTz;

/// Get offset in minutes using chrono-tz
fn chrono_offset(tz: ChronoTz, timestamp_secs: i64) -> i16 {
    let dt = DateTime::from_timestamp(timestamp_secs, 0).unwrap();
    let local = dt.with_timezone(&tz);
    (local.offset().fix().local_minus_utc() / 60) as i16
}

/// Get offset in minutes using compact implementation
fn compact_offset(tz_name: &str, timestamp_secs: i64) -> i16 {
    let tz: CompactTz = tz_name.parse().unwrap();
    tz.offset_at_timestamp(timestamp_secs)
}

/// Compare a single timestamp
fn compare_at(tz_name: &str, tz: ChronoTz, ts: i64) -> Result<(), String> {
    let chrono = chrono_offset(tz, ts);
    let compact = compact_offset(tz_name, ts);
    if chrono != compact {
        let dt = DateTime::from_timestamp(ts, 0).unwrap();
        Err(format!(
            "{} at {} (ts={}): chrono={}, compact={}",
            tz_name,
            dt.format("%Y-%m-%d %H:%M:%S UTC"),
            ts,
            chrono,
            compact
        ))
    } else {
        Ok(())
    }
}

/// Test a timezone exhaustively
fn test_timezone_exhaustive(tz_name: &str) {
    let tz: ChronoTz = tz_name.parse().unwrap();
    let mut errors = Vec::new();

    // Test every year from 1970 to 2025
    // Historical data covers through 2025 for all timezones
    for year in 1970..=2025 {
        // Test every month
        for month in 1..=12u32 {
            // Test days 1, 15, and last day of month
            let days_in_month = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                        29
                    } else {
                        28
                    }
                },
                _ => 30,
            };

            for day in [1, 15, days_in_month] {
                // Test multiple hours including DST transition times
                for hour in [0, 1, 2, 3, 6, 12, 18, 23] {
                    if let Some(dt) = NaiveDate::from_ymd_opt(year, month, day)
                        .and_then(|d| d.and_hms_opt(hour, 0, 0))
                    {
                        let ts = dt.and_utc().timestamp();
                        if let Err(e) = compare_at(tz_name, tz, ts) {
                            errors.push(e);
                        }
                    }
                }
            }
        }
    }

    if !errors.is_empty() {
        panic!(
            "Found {} mismatches for {}:\n{}",
            errors.len(),
            tz_name,
            errors
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

// ============================================================================
// NORTH AMERICA (50+ tests)
// ============================================================================

#[test]
fn test_america_adak() {
    test_timezone_exhaustive("America/Adak");
}
#[test]
fn test_america_anchorage() {
    test_timezone_exhaustive("America/Anchorage");
}
#[test]
fn test_america_anguilla() {
    test_timezone_exhaustive("America/Anguilla");
}
#[test]
fn test_america_antigua() {
    test_timezone_exhaustive("America/Antigua");
}
#[test]
fn test_america_araguaina() {
    test_timezone_exhaustive("America/Araguaina");
}
#[test]
fn test_america_argentina_buenos_aires() {
    test_timezone_exhaustive("America/Argentina/Buenos_Aires");
}
#[test]
fn test_america_argentina_catamarca() {
    test_timezone_exhaustive("America/Argentina/Catamarca");
}
#[test]
fn test_america_argentina_cordoba() {
    test_timezone_exhaustive("America/Argentina/Cordoba");
}
#[test]
fn test_america_argentina_jujuy() {
    test_timezone_exhaustive("America/Argentina/Jujuy");
}
#[test]
fn test_america_argentina_la_rioja() {
    test_timezone_exhaustive("America/Argentina/La_Rioja");
}
#[test]
fn test_america_argentina_mendoza() {
    test_timezone_exhaustive("America/Argentina/Mendoza");
}
#[test]
fn test_america_argentina_rio_gallegos() {
    test_timezone_exhaustive("America/Argentina/Rio_Gallegos");
}
#[test]
fn test_america_argentina_salta() {
    test_timezone_exhaustive("America/Argentina/Salta");
}
#[test]
fn test_america_argentina_san_juan() {
    test_timezone_exhaustive("America/Argentina/San_Juan");
}
#[test]
fn test_america_argentina_san_luis() {
    test_timezone_exhaustive("America/Argentina/San_Luis");
}
#[test]
fn test_america_argentina_tucuman() {
    test_timezone_exhaustive("America/Argentina/Tucuman");
}
#[test]
fn test_america_argentina_ushuaia() {
    test_timezone_exhaustive("America/Argentina/Ushuaia");
}
#[test]
fn test_america_aruba() {
    test_timezone_exhaustive("America/Aruba");
}
#[test]
fn test_america_asuncion() {
    test_timezone_exhaustive("America/Asuncion");
}
#[test]
fn test_america_atikokan() {
    test_timezone_exhaustive("America/Atikokan");
}
#[test]
fn test_america_bahia() {
    test_timezone_exhaustive("America/Bahia");
}
#[test]
fn test_america_bahia_banderas() {
    test_timezone_exhaustive("America/Bahia_Banderas");
}
#[test]
fn test_america_barbados() {
    test_timezone_exhaustive("America/Barbados");
}
#[test]
fn test_america_belem() {
    test_timezone_exhaustive("America/Belem");
}
#[test]
fn test_america_belize() {
    test_timezone_exhaustive("America/Belize");
}
#[test]
fn test_america_blanc_sablon() {
    test_timezone_exhaustive("America/Blanc-Sablon");
}
#[test]
fn test_america_boa_vista() {
    test_timezone_exhaustive("America/Boa_Vista");
}
#[test]
fn test_america_bogota() {
    test_timezone_exhaustive("America/Bogota");
}
#[test]
fn test_america_boise() {
    test_timezone_exhaustive("America/Boise");
}
#[test]
fn test_america_cambridge_bay() {
    test_timezone_exhaustive("America/Cambridge_Bay");
}
#[test]
fn test_america_campo_grande() {
    test_timezone_exhaustive("America/Campo_Grande");
}
#[test]
fn test_america_cancun() {
    test_timezone_exhaustive("America/Cancun");
}
#[test]
fn test_america_caracas() {
    test_timezone_exhaustive("America/Caracas");
}
#[test]
fn test_america_cayenne() {
    test_timezone_exhaustive("America/Cayenne");
}
#[test]
fn test_america_cayman() {
    test_timezone_exhaustive("America/Cayman");
}
#[test]
fn test_america_chicago() {
    test_timezone_exhaustive("America/Chicago");
}
#[test]
fn test_america_chihuahua() {
    test_timezone_exhaustive("America/Chihuahua");
}
#[test]
fn test_america_costa_rica() {
    test_timezone_exhaustive("America/Costa_Rica");
}
#[test]
fn test_america_creston() {
    test_timezone_exhaustive("America/Creston");
}
#[test]
fn test_america_cuiaba() {
    test_timezone_exhaustive("America/Cuiaba");
}
#[test]
fn test_america_curacao() {
    test_timezone_exhaustive("America/Curacao");
}
#[test]
fn test_america_danmarkshavn() {
    test_timezone_exhaustive("America/Danmarkshavn");
}
#[test]
fn test_america_dawson() {
    test_timezone_exhaustive("America/Dawson");
}
#[test]
fn test_america_dawson_creek() {
    test_timezone_exhaustive("America/Dawson_Creek");
}
#[test]
fn test_america_denver() {
    test_timezone_exhaustive("America/Denver");
}
#[test]
fn test_america_detroit() {
    test_timezone_exhaustive("America/Detroit");
}
#[test]
fn test_america_dominica() {
    test_timezone_exhaustive("America/Dominica");
}
#[test]
fn test_america_edmonton() {
    test_timezone_exhaustive("America/Edmonton");
}
#[test]
fn test_america_eirunepe() {
    test_timezone_exhaustive("America/Eirunepe");
}
#[test]
fn test_america_el_salvador() {
    test_timezone_exhaustive("America/El_Salvador");
}
#[test]
fn test_america_fort_nelson() {
    test_timezone_exhaustive("America/Fort_Nelson");
}
#[test]
fn test_america_fortaleza() {
    test_timezone_exhaustive("America/Fortaleza");
}
#[test]
fn test_america_glace_bay() {
    test_timezone_exhaustive("America/Glace_Bay");
}
#[test]
fn test_america_goose_bay() {
    test_timezone_exhaustive("America/Goose_Bay");
}
#[test]
fn test_america_grand_turk() {
    test_timezone_exhaustive("America/Grand_Turk");
}
#[test]
fn test_america_grenada() {
    test_timezone_exhaustive("America/Grenada");
}
#[test]
fn test_america_guadeloupe() {
    test_timezone_exhaustive("America/Guadeloupe");
}
#[test]
fn test_america_guatemala() {
    test_timezone_exhaustive("America/Guatemala");
}
#[test]
fn test_america_guayaquil() {
    test_timezone_exhaustive("America/Guayaquil");
}
#[test]
fn test_america_guyana() {
    test_timezone_exhaustive("America/Guyana");
}
#[test]
fn test_america_halifax() {
    test_timezone_exhaustive("America/Halifax");
}
#[test]
fn test_america_havana() {
    test_timezone_exhaustive("America/Havana");
}
#[test]
fn test_america_hermosillo() {
    test_timezone_exhaustive("America/Hermosillo");
}
#[test]
fn test_america_indiana_indianapolis() {
    test_timezone_exhaustive("America/Indiana/Indianapolis");
}
#[test]
fn test_america_indiana_knox() {
    test_timezone_exhaustive("America/Indiana/Knox");
}
#[test]
fn test_america_indiana_marengo() {
    test_timezone_exhaustive("America/Indiana/Marengo");
}
#[test]
fn test_america_indiana_petersburg() {
    test_timezone_exhaustive("America/Indiana/Petersburg");
}
#[test]
fn test_america_indiana_tell_city() {
    test_timezone_exhaustive("America/Indiana/Tell_City");
}
#[test]
fn test_america_indiana_vevay() {
    test_timezone_exhaustive("America/Indiana/Vevay");
}
#[test]
fn test_america_indiana_vincennes() {
    test_timezone_exhaustive("America/Indiana/Vincennes");
}
#[test]
fn test_america_indiana_winamac() {
    test_timezone_exhaustive("America/Indiana/Winamac");
}
#[test]
fn test_america_inuvik() {
    test_timezone_exhaustive("America/Inuvik");
}
#[test]
fn test_america_iqaluit() {
    test_timezone_exhaustive("America/Iqaluit");
}
#[test]
fn test_america_jamaica() {
    test_timezone_exhaustive("America/Jamaica");
}
#[test]
fn test_america_juneau() {
    test_timezone_exhaustive("America/Juneau");
}
#[test]
fn test_america_kentucky_louisville() {
    test_timezone_exhaustive("America/Kentucky/Louisville");
}
#[test]
fn test_america_kentucky_monticello() {
    test_timezone_exhaustive("America/Kentucky/Monticello");
}
#[test]
fn test_america_kralendijk() {
    test_timezone_exhaustive("America/Kralendijk");
}
#[test]
fn test_america_la_paz() {
    test_timezone_exhaustive("America/La_Paz");
}
#[test]
fn test_america_lima() {
    test_timezone_exhaustive("America/Lima");
}
#[test]
fn test_america_los_angeles() {
    test_timezone_exhaustive("America/Los_Angeles");
}
#[test]
fn test_america_lower_princes() {
    test_timezone_exhaustive("America/Lower_Princes");
}
#[test]
fn test_america_maceio() {
    test_timezone_exhaustive("America/Maceio");
}
#[test]
fn test_america_managua() {
    test_timezone_exhaustive("America/Managua");
}
#[test]
fn test_america_manaus() {
    test_timezone_exhaustive("America/Manaus");
}
#[test]
fn test_america_marigot() {
    test_timezone_exhaustive("America/Marigot");
}
#[test]
fn test_america_martinique() {
    test_timezone_exhaustive("America/Martinique");
}
#[test]
fn test_america_matamoros() {
    test_timezone_exhaustive("America/Matamoros");
}
#[test]
fn test_america_mazatlan() {
    test_timezone_exhaustive("America/Mazatlan");
}
#[test]
fn test_america_menominee() {
    test_timezone_exhaustive("America/Menominee");
}
#[test]
fn test_america_merida() {
    test_timezone_exhaustive("America/Merida");
}
#[test]
fn test_america_metlakatla() {
    test_timezone_exhaustive("America/Metlakatla");
}
#[test]
fn test_america_mexico_city() {
    test_timezone_exhaustive("America/Mexico_City");
}
#[test]
fn test_america_miquelon() {
    test_timezone_exhaustive("America/Miquelon");
}
#[test]
fn test_america_moncton() {
    test_timezone_exhaustive("America/Moncton");
}
#[test]
fn test_america_monterrey() {
    test_timezone_exhaustive("America/Monterrey");
}
#[test]
fn test_america_montevideo() {
    test_timezone_exhaustive("America/Montevideo");
}
#[test]
fn test_america_montserrat() {
    test_timezone_exhaustive("America/Montserrat");
}
#[test]
fn test_america_nassau() {
    test_timezone_exhaustive("America/Nassau");
}
#[test]
fn test_america_new_york() {
    test_timezone_exhaustive("America/New_York");
}
#[test]
fn test_america_nipigon() {
    test_timezone_exhaustive("America/Nipigon");
}
#[test]
fn test_america_nome() {
    test_timezone_exhaustive("America/Nome");
}
#[test]
fn test_america_noronha() {
    test_timezone_exhaustive("America/Noronha");
}
#[test]
fn test_america_north_dakota_beulah() {
    test_timezone_exhaustive("America/North_Dakota/Beulah");
}
#[test]
fn test_america_north_dakota_center() {
    test_timezone_exhaustive("America/North_Dakota/Center");
}
#[test]
fn test_america_north_dakota_new_salem() {
    test_timezone_exhaustive("America/North_Dakota/New_Salem");
}
#[test]
fn test_america_nuuk() {
    test_timezone_exhaustive("America/Nuuk");
}
#[test]
fn test_america_ojinaga() {
    test_timezone_exhaustive("America/Ojinaga");
}
#[test]
fn test_america_panama() {
    test_timezone_exhaustive("America/Panama");
}
#[test]
fn test_america_pangnirtung() {
    test_timezone_exhaustive("America/Pangnirtung");
}
#[test]
fn test_america_paramaribo() {
    test_timezone_exhaustive("America/Paramaribo");
}
#[test]
fn test_america_phoenix() {
    test_timezone_exhaustive("America/Phoenix");
}
#[test]
fn test_america_port_au_prince() {
    test_timezone_exhaustive("America/Port-au-Prince");
}
#[test]
fn test_america_port_of_spain() {
    test_timezone_exhaustive("America/Port_of_Spain");
}
#[test]
fn test_america_porto_velho() {
    test_timezone_exhaustive("America/Porto_Velho");
}
#[test]
fn test_america_puerto_rico() {
    test_timezone_exhaustive("America/Puerto_Rico");
}
#[test]
fn test_america_punta_arenas() {
    test_timezone_exhaustive("America/Punta_Arenas");
}
#[test]
fn test_america_rainy_river() {
    test_timezone_exhaustive("America/Rainy_River");
}
#[test]
fn test_america_rankin_inlet() {
    test_timezone_exhaustive("America/Rankin_Inlet");
}
#[test]
fn test_america_recife() {
    test_timezone_exhaustive("America/Recife");
}
#[test]
fn test_america_regina() {
    test_timezone_exhaustive("America/Regina");
}
#[test]
fn test_america_resolute() {
    test_timezone_exhaustive("America/Resolute");
}
#[test]
fn test_america_rio_branco() {
    test_timezone_exhaustive("America/Rio_Branco");
}
#[test]
fn test_america_santarem() {
    test_timezone_exhaustive("America/Santarem");
}
#[test]
fn test_america_santiago() {
    test_timezone_exhaustive("America/Santiago");
}
#[test]
fn test_america_santo_domingo() {
    test_timezone_exhaustive("America/Santo_Domingo");
}
#[test]
fn test_america_sao_paulo() {
    test_timezone_exhaustive("America/Sao_Paulo");
}
#[test]
fn test_america_scoresbysund() {
    test_timezone_exhaustive("America/Scoresbysund");
}
#[test]
fn test_america_sitka() {
    test_timezone_exhaustive("America/Sitka");
}
#[test]
fn test_america_st_barthelemy() {
    test_timezone_exhaustive("America/St_Barthelemy");
}
#[test]
fn test_america_st_johns() {
    test_timezone_exhaustive("America/St_Johns");
}
#[test]
fn test_america_st_kitts() {
    test_timezone_exhaustive("America/St_Kitts");
}
#[test]
fn test_america_st_lucia() {
    test_timezone_exhaustive("America/St_Lucia");
}
#[test]
fn test_america_st_thomas() {
    test_timezone_exhaustive("America/St_Thomas");
}
#[test]
fn test_america_st_vincent() {
    test_timezone_exhaustive("America/St_Vincent");
}
#[test]
fn test_america_swift_current() {
    test_timezone_exhaustive("America/Swift_Current");
}
#[test]
fn test_america_tegucigalpa() {
    test_timezone_exhaustive("America/Tegucigalpa");
}
#[test]
fn test_america_thule() {
    test_timezone_exhaustive("America/Thule");
}
#[test]
fn test_america_thunder_bay() {
    test_timezone_exhaustive("America/Thunder_Bay");
}
#[test]
fn test_america_tijuana() {
    test_timezone_exhaustive("America/Tijuana");
}
#[test]
fn test_america_toronto() {
    test_timezone_exhaustive("America/Toronto");
}
#[test]
fn test_america_tortola() {
    test_timezone_exhaustive("America/Tortola");
}
#[test]
fn test_america_vancouver() {
    test_timezone_exhaustive("America/Vancouver");
}
#[test]
fn test_america_whitehorse() {
    test_timezone_exhaustive("America/Whitehorse");
}
#[test]
fn test_america_winnipeg() {
    test_timezone_exhaustive("America/Winnipeg");
}
#[test]
fn test_america_yakutat() {
    test_timezone_exhaustive("America/Yakutat");
}
#[test]
fn test_america_yellowknife() {
    test_timezone_exhaustive("America/Yellowknife");
}

// ============================================================================
// EUROPE (60+ tests)
// ============================================================================

#[test]
fn test_europe_amsterdam() {
    test_timezone_exhaustive("Europe/Amsterdam");
}
#[test]
fn test_europe_andorra() {
    test_timezone_exhaustive("Europe/Andorra");
}
#[test]
fn test_europe_astrakhan() {
    test_timezone_exhaustive("Europe/Astrakhan");
}
#[test]
fn test_europe_athens() {
    test_timezone_exhaustive("Europe/Athens");
}
#[test]
fn test_europe_belgrade() {
    test_timezone_exhaustive("Europe/Belgrade");
}
#[test]
fn test_europe_berlin() {
    test_timezone_exhaustive("Europe/Berlin");
}
#[test]
fn test_europe_bratislava() {
    test_timezone_exhaustive("Europe/Bratislava");
}
#[test]
fn test_europe_brussels() {
    test_timezone_exhaustive("Europe/Brussels");
}
#[test]
fn test_europe_bucharest() {
    test_timezone_exhaustive("Europe/Bucharest");
}
#[test]
fn test_europe_budapest() {
    test_timezone_exhaustive("Europe/Budapest");
}
#[test]
fn test_europe_busingen() {
    test_timezone_exhaustive("Europe/Busingen");
}
#[test]
fn test_europe_chisinau() {
    test_timezone_exhaustive("Europe/Chisinau");
}
#[test]
fn test_europe_copenhagen() {
    test_timezone_exhaustive("Europe/Copenhagen");
}
#[test]
fn test_europe_dublin() {
    test_timezone_exhaustive("Europe/Dublin");
}
#[test]
fn test_europe_gibraltar() {
    test_timezone_exhaustive("Europe/Gibraltar");
}
#[test]
fn test_europe_guernsey() {
    test_timezone_exhaustive("Europe/Guernsey");
}
#[test]
fn test_europe_helsinki() {
    test_timezone_exhaustive("Europe/Helsinki");
}
#[test]
fn test_europe_isle_of_man() {
    test_timezone_exhaustive("Europe/Isle_of_Man");
}
#[test]
fn test_europe_istanbul() {
    test_timezone_exhaustive("Europe/Istanbul");
}
#[test]
fn test_europe_jersey() {
    test_timezone_exhaustive("Europe/Jersey");
}
#[test]
fn test_europe_kaliningrad() {
    test_timezone_exhaustive("Europe/Kaliningrad");
}
#[test]
fn test_europe_kiev() {
    test_timezone_exhaustive("Europe/Kiev");
}
#[test]
fn test_europe_kirov() {
    test_timezone_exhaustive("Europe/Kirov");
}
#[test]
fn test_europe_lisbon() {
    test_timezone_exhaustive("Europe/Lisbon");
}
#[test]
fn test_europe_ljubljana() {
    test_timezone_exhaustive("Europe/Ljubljana");
}
#[test]
fn test_europe_london() {
    test_timezone_exhaustive("Europe/London");
}
#[test]
fn test_europe_luxembourg() {
    test_timezone_exhaustive("Europe/Luxembourg");
}
#[test]
fn test_europe_madrid() {
    test_timezone_exhaustive("Europe/Madrid");
}
#[test]
fn test_europe_malta() {
    test_timezone_exhaustive("Europe/Malta");
}
#[test]
fn test_europe_mariehamn() {
    test_timezone_exhaustive("Europe/Mariehamn");
}
#[test]
fn test_europe_minsk() {
    test_timezone_exhaustive("Europe/Minsk");
}
#[test]
fn test_europe_monaco() {
    test_timezone_exhaustive("Europe/Monaco");
}
#[test]
fn test_europe_moscow() {
    test_timezone_exhaustive("Europe/Moscow");
}
#[test]
fn test_europe_oslo() {
    test_timezone_exhaustive("Europe/Oslo");
}
#[test]
fn test_europe_paris() {
    test_timezone_exhaustive("Europe/Paris");
}
#[test]
fn test_europe_podgorica() {
    test_timezone_exhaustive("Europe/Podgorica");
}
#[test]
fn test_europe_prague() {
    test_timezone_exhaustive("Europe/Prague");
}
#[test]
fn test_europe_riga() {
    test_timezone_exhaustive("Europe/Riga");
}
#[test]
fn test_europe_rome() {
    test_timezone_exhaustive("Europe/Rome");
}
#[test]
fn test_europe_samara() {
    test_timezone_exhaustive("Europe/Samara");
}
#[test]
fn test_europe_san_marino() {
    test_timezone_exhaustive("Europe/San_Marino");
}
#[test]
fn test_europe_sarajevo() {
    test_timezone_exhaustive("Europe/Sarajevo");
}
#[test]
fn test_europe_saratov() {
    test_timezone_exhaustive("Europe/Saratov");
}
#[test]
fn test_europe_simferopol() {
    test_timezone_exhaustive("Europe/Simferopol");
}
#[test]
fn test_europe_skopje() {
    test_timezone_exhaustive("Europe/Skopje");
}
#[test]
fn test_europe_sofia() {
    test_timezone_exhaustive("Europe/Sofia");
}
#[test]
fn test_europe_stockholm() {
    test_timezone_exhaustive("Europe/Stockholm");
}
#[test]
fn test_europe_tallinn() {
    test_timezone_exhaustive("Europe/Tallinn");
}
#[test]
fn test_europe_tirane() {
    test_timezone_exhaustive("Europe/Tirane");
}
#[test]
fn test_europe_ulyanovsk() {
    test_timezone_exhaustive("Europe/Ulyanovsk");
}
#[test]
fn test_europe_uzhgorod() {
    test_timezone_exhaustive("Europe/Uzhgorod");
}
#[test]
fn test_europe_vaduz() {
    test_timezone_exhaustive("Europe/Vaduz");
}
#[test]
fn test_europe_vatican() {
    test_timezone_exhaustive("Europe/Vatican");
}
#[test]
fn test_europe_vienna() {
    test_timezone_exhaustive("Europe/Vienna");
}
#[test]
fn test_europe_vilnius() {
    test_timezone_exhaustive("Europe/Vilnius");
}
#[test]
fn test_europe_volgograd() {
    test_timezone_exhaustive("Europe/Volgograd");
}
#[test]
fn test_europe_warsaw() {
    test_timezone_exhaustive("Europe/Warsaw");
}
#[test]
fn test_europe_zagreb() {
    test_timezone_exhaustive("Europe/Zagreb");
}
#[test]
fn test_europe_zaporozhye() {
    test_timezone_exhaustive("Europe/Zaporozhye");
}
#[test]
fn test_europe_zurich() {
    test_timezone_exhaustive("Europe/Zurich");
}

// ============================================================================
// ASIA (80+ tests)
// ============================================================================

#[test]
fn test_asia_aden() {
    test_timezone_exhaustive("Asia/Aden");
}
#[test]
fn test_asia_almaty() {
    test_timezone_exhaustive("Asia/Almaty");
}
#[test]
fn test_asia_amman() {
    test_timezone_exhaustive("Asia/Amman");
}
#[test]
fn test_asia_anadyr() {
    test_timezone_exhaustive("Asia/Anadyr");
}
#[test]
fn test_asia_aqtau() {
    test_timezone_exhaustive("Asia/Aqtau");
}
#[test]
fn test_asia_aqtobe() {
    test_timezone_exhaustive("Asia/Aqtobe");
}
#[test]
fn test_asia_ashgabat() {
    test_timezone_exhaustive("Asia/Ashgabat");
}
#[test]
fn test_asia_atyrau() {
    test_timezone_exhaustive("Asia/Atyrau");
}
#[test]
fn test_asia_baghdad() {
    test_timezone_exhaustive("Asia/Baghdad");
}
#[test]
fn test_asia_bahrain() {
    test_timezone_exhaustive("Asia/Bahrain");
}
#[test]
fn test_asia_baku() {
    test_timezone_exhaustive("Asia/Baku");
}
#[test]
fn test_asia_bangkok() {
    test_timezone_exhaustive("Asia/Bangkok");
}
#[test]
fn test_asia_barnaul() {
    test_timezone_exhaustive("Asia/Barnaul");
}
#[test]
fn test_asia_beirut() {
    test_timezone_exhaustive("Asia/Beirut");
}
#[test]
fn test_asia_bishkek() {
    test_timezone_exhaustive("Asia/Bishkek");
}
#[test]
fn test_asia_brunei() {
    test_timezone_exhaustive("Asia/Brunei");
}
#[test]
fn test_asia_chita() {
    test_timezone_exhaustive("Asia/Chita");
}
#[test]
fn test_asia_choibalsan() {
    test_timezone_exhaustive("Asia/Choibalsan");
}
#[test]
fn test_asia_colombo() {
    test_timezone_exhaustive("Asia/Colombo");
}
#[test]
fn test_asia_damascus() {
    test_timezone_exhaustive("Asia/Damascus");
}
#[test]
fn test_asia_dhaka() {
    test_timezone_exhaustive("Asia/Dhaka");
}
#[test]
fn test_asia_dili() {
    test_timezone_exhaustive("Asia/Dili");
}
#[test]
fn test_asia_dubai() {
    test_timezone_exhaustive("Asia/Dubai");
}
#[test]
fn test_asia_dushanbe() {
    test_timezone_exhaustive("Asia/Dushanbe");
}
#[test]
fn test_asia_famagusta() {
    test_timezone_exhaustive("Asia/Famagusta");
}
#[test]
fn test_asia_gaza() {
    test_timezone_exhaustive("Asia/Gaza");
}
#[test]
fn test_asia_hebron() {
    test_timezone_exhaustive("Asia/Hebron");
}
#[test]
fn test_asia_ho_chi_minh() {
    test_timezone_exhaustive("Asia/Ho_Chi_Minh");
}
#[test]
fn test_asia_hong_kong() {
    test_timezone_exhaustive("Asia/Hong_Kong");
}
#[test]
fn test_asia_hovd() {
    test_timezone_exhaustive("Asia/Hovd");
}
#[test]
fn test_asia_irkutsk() {
    test_timezone_exhaustive("Asia/Irkutsk");
}
#[test]
fn test_asia_jakarta() {
    test_timezone_exhaustive("Asia/Jakarta");
}
#[test]
fn test_asia_jayapura() {
    test_timezone_exhaustive("Asia/Jayapura");
}
#[test]
fn test_asia_jerusalem() {
    test_timezone_exhaustive("Asia/Jerusalem");
}
#[test]
fn test_asia_kabul() {
    test_timezone_exhaustive("Asia/Kabul");
}
#[test]
fn test_asia_kamchatka() {
    test_timezone_exhaustive("Asia/Kamchatka");
}
#[test]
fn test_asia_karachi() {
    test_timezone_exhaustive("Asia/Karachi");
}
#[test]
fn test_asia_kathmandu() {
    test_timezone_exhaustive("Asia/Kathmandu");
}
#[test]
fn test_asia_khandyga() {
    test_timezone_exhaustive("Asia/Khandyga");
}
#[test]
fn test_asia_kolkata() {
    test_timezone_exhaustive("Asia/Kolkata");
}
#[test]
fn test_asia_krasnoyarsk() {
    test_timezone_exhaustive("Asia/Krasnoyarsk");
}
#[test]
fn test_asia_kuala_lumpur() {
    test_timezone_exhaustive("Asia/Kuala_Lumpur");
}
#[test]
fn test_asia_kuching() {
    test_timezone_exhaustive("Asia/Kuching");
}
#[test]
fn test_asia_kuwait() {
    test_timezone_exhaustive("Asia/Kuwait");
}
#[test]
fn test_asia_macau() {
    test_timezone_exhaustive("Asia/Macau");
}
#[test]
fn test_asia_magadan() {
    test_timezone_exhaustive("Asia/Magadan");
}
#[test]
fn test_asia_makassar() {
    test_timezone_exhaustive("Asia/Makassar");
}
#[test]
fn test_asia_manila() {
    test_timezone_exhaustive("Asia/Manila");
}
#[test]
fn test_asia_muscat() {
    test_timezone_exhaustive("Asia/Muscat");
}
#[test]
fn test_asia_nicosia() {
    test_timezone_exhaustive("Asia/Nicosia");
}
#[test]
fn test_asia_novokuznetsk() {
    test_timezone_exhaustive("Asia/Novokuznetsk");
}
#[test]
fn test_asia_novosibirsk() {
    test_timezone_exhaustive("Asia/Novosibirsk");
}
#[test]
fn test_asia_omsk() {
    test_timezone_exhaustive("Asia/Omsk");
}
#[test]
fn test_asia_oral() {
    test_timezone_exhaustive("Asia/Oral");
}
#[test]
fn test_asia_phnom_penh() {
    test_timezone_exhaustive("Asia/Phnom_Penh");
}
#[test]
fn test_asia_pontianak() {
    test_timezone_exhaustive("Asia/Pontianak");
}
#[test]
fn test_asia_pyongyang() {
    test_timezone_exhaustive("Asia/Pyongyang");
}
#[test]
fn test_asia_qatar() {
    test_timezone_exhaustive("Asia/Qatar");
}
#[test]
fn test_asia_qostanay() {
    test_timezone_exhaustive("Asia/Qostanay");
}
#[test]
fn test_asia_qyzylorda() {
    test_timezone_exhaustive("Asia/Qyzylorda");
}
#[test]
fn test_asia_riyadh() {
    test_timezone_exhaustive("Asia/Riyadh");
}
#[test]
fn test_asia_sakhalin() {
    test_timezone_exhaustive("Asia/Sakhalin");
}
#[test]
fn test_asia_samarkand() {
    test_timezone_exhaustive("Asia/Samarkand");
}
#[test]
fn test_asia_seoul() {
    test_timezone_exhaustive("Asia/Seoul");
}
#[test]
fn test_asia_shanghai() {
    test_timezone_exhaustive("Asia/Shanghai");
}
#[test]
fn test_asia_singapore() {
    test_timezone_exhaustive("Asia/Singapore");
}
#[test]
fn test_asia_srednekolymsk() {
    test_timezone_exhaustive("Asia/Srednekolymsk");
}
#[test]
fn test_asia_taipei() {
    test_timezone_exhaustive("Asia/Taipei");
}
#[test]
fn test_asia_tashkent() {
    test_timezone_exhaustive("Asia/Tashkent");
}
#[test]
fn test_asia_tbilisi() {
    test_timezone_exhaustive("Asia/Tbilisi");
}
#[test]
fn test_asia_tehran() {
    test_timezone_exhaustive("Asia/Tehran");
}
#[test]
fn test_asia_thimphu() {
    test_timezone_exhaustive("Asia/Thimphu");
}
#[test]
fn test_asia_tokyo() {
    test_timezone_exhaustive("Asia/Tokyo");
}
#[test]
fn test_asia_tomsk() {
    test_timezone_exhaustive("Asia/Tomsk");
}
#[test]
fn test_asia_ulaanbaatar() {
    test_timezone_exhaustive("Asia/Ulaanbaatar");
}
#[test]
fn test_asia_urumqi() {
    test_timezone_exhaustive("Asia/Urumqi");
}
#[test]
fn test_asia_ust_nera() {
    test_timezone_exhaustive("Asia/Ust-Nera");
}
#[test]
fn test_asia_vientiane() {
    test_timezone_exhaustive("Asia/Vientiane");
}
#[test]
fn test_asia_vladivostok() {
    test_timezone_exhaustive("Asia/Vladivostok");
}
#[test]
fn test_asia_yakutsk() {
    test_timezone_exhaustive("Asia/Yakutsk");
}
#[test]
fn test_asia_yangon() {
    test_timezone_exhaustive("Asia/Yangon");
}
#[test]
fn test_asia_yekaterinburg() {
    test_timezone_exhaustive("Asia/Yekaterinburg");
}
#[test]
fn test_asia_yerevan() {
    test_timezone_exhaustive("Asia/Yerevan");
}

// ============================================================================
// AFRICA (50+ tests)
// ============================================================================

#[test]
fn test_africa_abidjan() {
    test_timezone_exhaustive("Africa/Abidjan");
}
#[test]
fn test_africa_accra() {
    test_timezone_exhaustive("Africa/Accra");
}
#[test]
fn test_africa_addis_ababa() {
    test_timezone_exhaustive("Africa/Addis_Ababa");
}
#[test]
fn test_africa_algiers() {
    test_timezone_exhaustive("Africa/Algiers");
}
#[test]
fn test_africa_asmara() {
    test_timezone_exhaustive("Africa/Asmara");
}
#[test]
fn test_africa_bamako() {
    test_timezone_exhaustive("Africa/Bamako");
}
#[test]
fn test_africa_bangui() {
    test_timezone_exhaustive("Africa/Bangui");
}
#[test]
fn test_africa_banjul() {
    test_timezone_exhaustive("Africa/Banjul");
}
#[test]
fn test_africa_bissau() {
    test_timezone_exhaustive("Africa/Bissau");
}
#[test]
fn test_africa_blantyre() {
    test_timezone_exhaustive("Africa/Blantyre");
}
#[test]
fn test_africa_brazzaville() {
    test_timezone_exhaustive("Africa/Brazzaville");
}
#[test]
fn test_africa_bujumbura() {
    test_timezone_exhaustive("Africa/Bujumbura");
}
#[test]
fn test_africa_cairo() {
    test_timezone_exhaustive("Africa/Cairo");
}
#[test]
fn test_africa_casablanca() {
    test_timezone_exhaustive("Africa/Casablanca");
}
#[test]
fn test_africa_ceuta() {
    test_timezone_exhaustive("Africa/Ceuta");
}
#[test]
fn test_africa_conakry() {
    test_timezone_exhaustive("Africa/Conakry");
}
#[test]
fn test_africa_dakar() {
    test_timezone_exhaustive("Africa/Dakar");
}
#[test]
fn test_africa_dar_es_salaam() {
    test_timezone_exhaustive("Africa/Dar_es_Salaam");
}
#[test]
fn test_africa_djibouti() {
    test_timezone_exhaustive("Africa/Djibouti");
}
#[test]
fn test_africa_douala() {
    test_timezone_exhaustive("Africa/Douala");
}
#[test]
fn test_africa_el_aaiun() {
    test_timezone_exhaustive("Africa/El_Aaiun");
}
#[test]
fn test_africa_freetown() {
    test_timezone_exhaustive("Africa/Freetown");
}
#[test]
fn test_africa_gaborone() {
    test_timezone_exhaustive("Africa/Gaborone");
}
#[test]
fn test_africa_harare() {
    test_timezone_exhaustive("Africa/Harare");
}
#[test]
fn test_africa_johannesburg() {
    test_timezone_exhaustive("Africa/Johannesburg");
}
#[test]
fn test_africa_juba() {
    test_timezone_exhaustive("Africa/Juba");
}
#[test]
fn test_africa_kampala() {
    test_timezone_exhaustive("Africa/Kampala");
}
#[test]
fn test_africa_khartoum() {
    test_timezone_exhaustive("Africa/Khartoum");
}
#[test]
fn test_africa_kigali() {
    test_timezone_exhaustive("Africa/Kigali");
}
#[test]
fn test_africa_kinshasa() {
    test_timezone_exhaustive("Africa/Kinshasa");
}
#[test]
fn test_africa_lagos() {
    test_timezone_exhaustive("Africa/Lagos");
}
#[test]
fn test_africa_libreville() {
    test_timezone_exhaustive("Africa/Libreville");
}
#[test]
fn test_africa_lome() {
    test_timezone_exhaustive("Africa/Lome");
}
#[test]
fn test_africa_luanda() {
    test_timezone_exhaustive("Africa/Luanda");
}
#[test]
fn test_africa_lubumbashi() {
    test_timezone_exhaustive("Africa/Lubumbashi");
}
#[test]
fn test_africa_lusaka() {
    test_timezone_exhaustive("Africa/Lusaka");
}
#[test]
fn test_africa_malabo() {
    test_timezone_exhaustive("Africa/Malabo");
}
#[test]
fn test_africa_maputo() {
    test_timezone_exhaustive("Africa/Maputo");
}
#[test]
fn test_africa_maseru() {
    test_timezone_exhaustive("Africa/Maseru");
}
#[test]
fn test_africa_mbabane() {
    test_timezone_exhaustive("Africa/Mbabane");
}
#[test]
fn test_africa_mogadishu() {
    test_timezone_exhaustive("Africa/Mogadishu");
}
#[test]
fn test_africa_monrovia() {
    test_timezone_exhaustive("Africa/Monrovia");
}
#[test]
fn test_africa_nairobi() {
    test_timezone_exhaustive("Africa/Nairobi");
}
#[test]
fn test_africa_ndjamena() {
    test_timezone_exhaustive("Africa/Ndjamena");
}
#[test]
fn test_africa_niamey() {
    test_timezone_exhaustive("Africa/Niamey");
}
#[test]
fn test_africa_nouakchott() {
    test_timezone_exhaustive("Africa/Nouakchott");
}
#[test]
fn test_africa_ouagadougou() {
    test_timezone_exhaustive("Africa/Ouagadougou");
}
#[test]
fn test_africa_porto_novo() {
    test_timezone_exhaustive("Africa/Porto-Novo");
}
#[test]
fn test_africa_sao_tome() {
    test_timezone_exhaustive("Africa/Sao_Tome");
}
#[test]
fn test_africa_tripoli() {
    test_timezone_exhaustive("Africa/Tripoli");
}
#[test]
fn test_africa_tunis() {
    test_timezone_exhaustive("Africa/Tunis");
}
#[test]
fn test_africa_windhoek() {
    test_timezone_exhaustive("Africa/Windhoek");
}

// ============================================================================
// AUSTRALIA & PACIFIC (40+ tests)
// ============================================================================

#[test]
fn test_australia_adelaide() {
    test_timezone_exhaustive("Australia/Adelaide");
}
#[test]
fn test_australia_brisbane() {
    test_timezone_exhaustive("Australia/Brisbane");
}
#[test]
fn test_australia_broken_hill() {
    test_timezone_exhaustive("Australia/Broken_Hill");
}
#[test]
fn test_australia_darwin() {
    test_timezone_exhaustive("Australia/Darwin");
}
#[test]
fn test_australia_eucla() {
    test_timezone_exhaustive("Australia/Eucla");
}
#[test]
fn test_australia_hobart() {
    test_timezone_exhaustive("Australia/Hobart");
}
#[test]
fn test_australia_lindeman() {
    test_timezone_exhaustive("Australia/Lindeman");
}
#[test]
fn test_australia_lord_howe() {
    test_timezone_exhaustive("Australia/Lord_Howe");
}
#[test]
fn test_australia_melbourne() {
    test_timezone_exhaustive("Australia/Melbourne");
}
#[test]
fn test_australia_perth() {
    test_timezone_exhaustive("Australia/Perth");
}
#[test]
fn test_australia_sydney() {
    test_timezone_exhaustive("Australia/Sydney");
}

#[test]
fn test_pacific_apia() {
    test_timezone_exhaustive("Pacific/Apia");
}
#[test]
fn test_pacific_auckland() {
    test_timezone_exhaustive("Pacific/Auckland");
}
#[test]
fn test_pacific_bougainville() {
    test_timezone_exhaustive("Pacific/Bougainville");
}
#[test]
fn test_pacific_chatham() {
    test_timezone_exhaustive("Pacific/Chatham");
}
#[test]
fn test_pacific_chuuk() {
    test_timezone_exhaustive("Pacific/Chuuk");
}
#[test]
fn test_pacific_easter() {
    test_timezone_exhaustive("Pacific/Easter");
}
#[test]
fn test_pacific_efate() {
    test_timezone_exhaustive("Pacific/Efate");
}
#[test]
fn test_pacific_fakaofo() {
    test_timezone_exhaustive("Pacific/Fakaofo");
}
#[test]
fn test_pacific_fiji() {
    test_timezone_exhaustive("Pacific/Fiji");
}
#[test]
fn test_pacific_funafuti() {
    test_timezone_exhaustive("Pacific/Funafuti");
}
#[test]
fn test_pacific_galapagos() {
    test_timezone_exhaustive("Pacific/Galapagos");
}
#[test]
fn test_pacific_gambier() {
    test_timezone_exhaustive("Pacific/Gambier");
}
#[test]
fn test_pacific_guadalcanal() {
    test_timezone_exhaustive("Pacific/Guadalcanal");
}
#[test]
fn test_pacific_guam() {
    test_timezone_exhaustive("Pacific/Guam");
}
#[test]
fn test_pacific_honolulu() {
    test_timezone_exhaustive("Pacific/Honolulu");
}
#[test]
fn test_pacific_kanton() {
    test_timezone_exhaustive("Pacific/Kanton");
}
#[test]
fn test_pacific_kiritimati() {
    test_timezone_exhaustive("Pacific/Kiritimati");
}
#[test]
fn test_pacific_kosrae() {
    test_timezone_exhaustive("Pacific/Kosrae");
}
#[test]
fn test_pacific_kwajalein() {
    test_timezone_exhaustive("Pacific/Kwajalein");
}
#[test]
fn test_pacific_majuro() {
    test_timezone_exhaustive("Pacific/Majuro");
}
#[test]
fn test_pacific_marquesas() {
    test_timezone_exhaustive("Pacific/Marquesas");
}
#[test]
fn test_pacific_midway() {
    test_timezone_exhaustive("Pacific/Midway");
}
#[test]
fn test_pacific_nauru() {
    test_timezone_exhaustive("Pacific/Nauru");
}
#[test]
fn test_pacific_niue() {
    test_timezone_exhaustive("Pacific/Niue");
}
#[test]
fn test_pacific_norfolk() {
    test_timezone_exhaustive("Pacific/Norfolk");
}
#[test]
fn test_pacific_noumea() {
    test_timezone_exhaustive("Pacific/Noumea");
}
#[test]
fn test_pacific_pago_pago() {
    test_timezone_exhaustive("Pacific/Pago_Pago");
}
#[test]
fn test_pacific_palau() {
    test_timezone_exhaustive("Pacific/Palau");
}
#[test]
fn test_pacific_pitcairn() {
    test_timezone_exhaustive("Pacific/Pitcairn");
}
#[test]
fn test_pacific_pohnpei() {
    test_timezone_exhaustive("Pacific/Pohnpei");
}
#[test]
fn test_pacific_port_moresby() {
    test_timezone_exhaustive("Pacific/Port_Moresby");
}
#[test]
fn test_pacific_rarotonga() {
    test_timezone_exhaustive("Pacific/Rarotonga");
}
#[test]
fn test_pacific_saipan() {
    test_timezone_exhaustive("Pacific/Saipan");
}
#[test]
fn test_pacific_tahiti() {
    test_timezone_exhaustive("Pacific/Tahiti");
}
#[test]
fn test_pacific_tarawa() {
    test_timezone_exhaustive("Pacific/Tarawa");
}
#[test]
fn test_pacific_tongatapu() {
    test_timezone_exhaustive("Pacific/Tongatapu");
}
#[test]
fn test_pacific_wake() {
    test_timezone_exhaustive("Pacific/Wake");
}
#[test]
fn test_pacific_wallis() {
    test_timezone_exhaustive("Pacific/Wallis");
}

// ============================================================================
// ATLANTIC, INDIAN, ANTARCTIC (20+ tests)
// ============================================================================

#[test]
fn test_atlantic_azores() {
    test_timezone_exhaustive("Atlantic/Azores");
}
#[test]
fn test_atlantic_bermuda() {
    test_timezone_exhaustive("Atlantic/Bermuda");
}
#[test]
fn test_atlantic_canary() {
    test_timezone_exhaustive("Atlantic/Canary");
}
#[test]
fn test_atlantic_cape_verde() {
    test_timezone_exhaustive("Atlantic/Cape_Verde");
}
#[test]
fn test_atlantic_faroe() {
    test_timezone_exhaustive("Atlantic/Faroe");
}
#[test]
fn test_atlantic_madeira() {
    test_timezone_exhaustive("Atlantic/Madeira");
}
#[test]
fn test_atlantic_reykjavik() {
    test_timezone_exhaustive("Atlantic/Reykjavik");
}
#[test]
fn test_atlantic_south_georgia() {
    test_timezone_exhaustive("Atlantic/South_Georgia");
}
#[test]
fn test_atlantic_st_helena() {
    test_timezone_exhaustive("Atlantic/St_Helena");
}
#[test]
fn test_atlantic_stanley() {
    test_timezone_exhaustive("Atlantic/Stanley");
}

#[test]
fn test_indian_antananarivo() {
    test_timezone_exhaustive("Indian/Antananarivo");
}
#[test]
fn test_indian_chagos() {
    test_timezone_exhaustive("Indian/Chagos");
}
#[test]
fn test_indian_christmas() {
    test_timezone_exhaustive("Indian/Christmas");
}
#[test]
fn test_indian_cocos() {
    test_timezone_exhaustive("Indian/Cocos");
}
#[test]
fn test_indian_comoro() {
    test_timezone_exhaustive("Indian/Comoro");
}
#[test]
fn test_indian_kerguelen() {
    test_timezone_exhaustive("Indian/Kerguelen");
}
#[test]
fn test_indian_mahe() {
    test_timezone_exhaustive("Indian/Mahe");
}
#[test]
fn test_indian_maldives() {
    test_timezone_exhaustive("Indian/Maldives");
}
#[test]
fn test_indian_mauritius() {
    test_timezone_exhaustive("Indian/Mauritius");
}
#[test]
fn test_indian_mayotte() {
    test_timezone_exhaustive("Indian/Mayotte");
}
#[test]
fn test_indian_reunion() {
    test_timezone_exhaustive("Indian/Reunion");
}

#[test]
fn test_antarctica_casey() {
    test_timezone_exhaustive("Antarctica/Casey");
}
#[test]
fn test_antarctica_davis() {
    test_timezone_exhaustive("Antarctica/Davis");
}
#[test]
fn test_antarctica_dumontdurville() {
    test_timezone_exhaustive("Antarctica/DumontDUrville");
}
#[test]
fn test_antarctica_macquarie() {
    test_timezone_exhaustive("Antarctica/Macquarie");
}
#[test]
fn test_antarctica_mawson() {
    test_timezone_exhaustive("Antarctica/Mawson");
}
#[test]
fn test_antarctica_mcmurdo() {
    test_timezone_exhaustive("Antarctica/McMurdo");
}
#[test]
fn test_antarctica_palmer() {
    test_timezone_exhaustive("Antarctica/Palmer");
}
#[test]
fn test_antarctica_rothera() {
    test_timezone_exhaustive("Antarctica/Rothera");
}
#[test]
fn test_antarctica_syowa() {
    test_timezone_exhaustive("Antarctica/Syowa");
}
#[test]
fn test_antarctica_troll() {
    test_timezone_exhaustive("Antarctica/Troll");
}
#[test]
fn test_antarctica_vostok() {
    test_timezone_exhaustive("Antarctica/Vostok");
}

// ============================================================================
// SPECIAL ZONES (10+ tests)
// ============================================================================

#[test]
fn test_utc() {
    test_timezone_exhaustive("UTC");
}
#[test]
fn test_etc_gmt() {
    test_timezone_exhaustive("Etc/GMT");
}
#[test]
fn test_etc_gmt_plus_0() {
    test_timezone_exhaustive("Etc/GMT+0");
}
#[test]
fn test_etc_gmt_minus_0() {
    test_timezone_exhaustive("Etc/GMT-0");
}
#[test]
fn test_etc_gmt_plus_1() {
    test_timezone_exhaustive("Etc/GMT+1");
}
#[test]
fn test_etc_gmt_plus_12() {
    test_timezone_exhaustive("Etc/GMT+12");
}
#[test]
fn test_etc_gmt_minus_1() {
    test_timezone_exhaustive("Etc/GMT-1");
}
#[test]
fn test_etc_gmt_minus_14() {
    test_timezone_exhaustive("Etc/GMT-14");
}
#[test]
fn test_etc_utc() {
    test_timezone_exhaustive("Etc/UTC");
}
