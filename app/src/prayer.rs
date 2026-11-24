//! Prayer times as the original computes them (0x140031c46): PrayTimes method with Kemenag
//! angles (Subuh 20°, Isya 18°, sunset 0.833°, Shafi'i Ashar), two refinement passes, then
//! minutes = ceil(time*60 + 2 - 1e-6). City table copied from the original's data (0x1400df470).

use crate::Dst;

/// (province, city, latitude, longitude, UTC offset hours).
pub const CITIES: &[(&str, &str, f64, f64, f64)] = &[
    ("Aceh", "Banda Aceh", 5.55, 95.32, 7.0),
    ("Aceh", "Lhokseumawe", 5.18, 97.15, 7.0),
    ("Sumatera Utara", "Medan", 3.59, 98.67, 7.0),
    ("Sumatera Utara", "Pematangsiantar", 2.96, 99.06, 7.0),
    ("Sumatera Barat", "Padang", -0.95, 100.35, 7.0),
    ("Sumatera Barat", "Bukittinggi", -0.30, 100.37, 7.0),
    ("Riau", "Pekanbaru", 0.51, 101.45, 7.0),
    ("Riau", "Dumai", 1.67, 101.45, 7.0),
    ("Kepulauan Riau", "Tanjung Pinang", 0.92, 104.45, 7.0),
    ("Kepulauan Riau", "Batam", 1.13, 104.05, 7.0),
    ("Jambi", "Jambi", -1.61, 103.61, 7.0),
    ("Sumatera Selatan", "Palembang", -2.98, 104.75, 7.0),
    ("Sumatera Selatan", "Lubuklinggau", -3.30, 102.86, 7.0),
    ("Bangka Belitung", "Pangkal Pinang", -2.13, 106.11, 7.0),
    ("Bengkulu", "Bengkulu", -3.80, 102.27, 7.0),
    ("Lampung", "Bandar Lampung", -5.43, 105.26, 7.0),
    ("Lampung", "Metro", -5.11, 105.31, 7.0),
    ("DKI Jakarta", "Jakarta", -6.20, 106.85, 7.0),
    ("Banten", "Serang", -6.12, 106.15, 7.0),
    ("Banten", "Tangerang", -6.18, 106.63, 7.0),
    ("Banten", "Cilegon", -6.00, 106.05, 7.0),
    ("Jawa Barat", "Bandung", -6.91, 107.61, 7.0),
    ("Jawa Barat", "Bekasi", -6.24, 106.99, 7.0),
    ("Jawa Barat", "Bogor", -6.60, 106.80, 7.0),
    ("Jawa Barat", "Depok", -6.40, 106.82, 7.0),
    ("Jawa Barat", "Cirebon", -6.71, 108.56, 7.0),
    ("Jawa Barat", "Tasikmalaya", -7.33, 108.22, 7.0),
    ("Jawa Barat", "Sukabumi", -6.92, 106.93, 7.0),
    ("Jawa Barat", "Karawang", -6.32, 107.34, 7.0),
    ("Jawa Tengah", "Semarang", -6.97, 110.42, 7.0),
    ("Jawa Tengah", "Surakarta", -7.57, 110.82, 7.0),
    ("Jawa Tengah", "Magelang", -7.48, 110.22, 7.0),
    ("Jawa Tengah", "Pekalongan", -6.89, 109.68, 7.0),
    ("Jawa Tengah", "Tegal", -6.87, 109.14, 7.0),
    ("Jawa Tengah", "Purwokerto", -7.42, 109.24, 7.0),
    ("Jawa Tengah", "Salatiga", -7.33, 110.50, 7.0),
    ("Jawa Tengah", "Kudus", -6.81, 110.84, 7.0),
    ("DI Yogyakarta", "Yogyakarta", -7.80, 110.36, 7.0),
    ("Jawa Timur", "Surabaya", -7.25, 112.75, 7.0),
    ("Jawa Timur", "Malang", -7.98, 112.63, 7.0),
    ("Jawa Timur", "Kediri", -7.82, 112.01, 7.0),
    ("Jawa Timur", "Madiun", -7.63, 111.52, 7.0),
    ("Jawa Timur", "Jember", -8.17, 113.70, 7.0),
    ("Jawa Timur", "Banyuwangi", -8.22, 114.37, 7.0),
    ("Jawa Timur", "Sidoarjo", -7.45, 112.72, 7.0),
    ("Jawa Timur", "Probolinggo", -7.75, 113.22, 7.0),
    ("Jawa Timur", "Blitar", -8.10, 112.16, 7.0),
    ("Bali", "Denpasar", -8.65, 115.22, 8.0),
    ("Nusa Tenggara Barat", "Mataram", -8.58, 116.12, 8.0),
    ("Nusa Tenggara Barat", "Bima", -8.46, 118.73, 8.0),
    ("Nusa Tenggara Timur", "Kupang", -10.18, 123.61, 8.0),
    ("Nusa Tenggara Timur", "Maumere", -8.62, 122.21, 8.0),
    ("Kalimantan Barat", "Pontianak", -0.03, 109.33, 7.0),
    ("Kalimantan Barat", "Singkawang", 0.91, 108.98, 7.0),
    ("Kalimantan Tengah", "Palangka Raya", -2.21, 113.92, 7.0),
    ("Kalimantan Tengah", "Sampit", -2.53, 112.95, 7.0),
    ("Kalimantan Selatan", "Banjarmasin", -3.32, 114.59, 8.0),
    ("Kalimantan Selatan", "Banjarbaru", -3.44, 114.83, 8.0),
    ("Kalimantan Timur", "Samarinda", -0.50, 117.15, 8.0),
    ("Kalimantan Timur", "Balikpapan", -1.27, 116.83, 8.0),
    ("Kalimantan Timur", "Bontang", 0.13, 117.50, 8.0),
    ("Kalimantan Utara", "Tanjung Selor", 2.84, 117.37, 8.0),
    ("Kalimantan Utara", "Tarakan", 3.30, 117.63, 8.0),
    ("Sulawesi Utara", "Manado", 1.47, 124.84, 8.0),
    ("Sulawesi Utara", "Bitung", 1.44, 125.19, 8.0),
    ("Gorontalo", "Gorontalo", 0.54, 123.06, 8.0),
    ("Sulawesi Tengah", "Palu", -0.90, 119.87, 8.0),
    ("Sulawesi Barat", "Mamuju", -2.68, 118.89, 8.0),
    ("Sulawesi Selatan", "Makassar", -5.15, 119.43, 8.0),
    ("Sulawesi Selatan", "Parepare", -4.01, 119.62, 8.0),
    ("Sulawesi Selatan", "Palopo", -2.99, 120.20, 8.0),
    ("Sulawesi Tenggara", "Kendari", -3.97, 122.51, 8.0),
    ("Sulawesi Tenggara", "Baubau", -5.47, 122.62, 8.0),
    ("Maluku", "Ambon", -3.70, 128.18, 9.0),
    ("Maluku", "Tual", -5.64, 132.75, 9.0),
    ("Maluku Utara", "Sofifi", 0.73, 127.57, 9.0),
    ("Maluku Utara", "Ternate", 0.79, 127.38, 9.0),
    ("Papua", "Jayapura", -2.54, 140.71, 9.0),
    ("Papua Barat", "Manokwari", -0.86, 134.06, 9.0),
    ("Papua Barat Daya", "Sorong", -0.88, 131.26, 9.0),
    ("Papua Tengah", "Nabire", -3.37, 135.50, 9.0),
    ("Papua Tengah", "Timika", -4.55, 136.89, 9.0),
    ("Papua Pegunungan", "Wamena", -4.10, 138.95, 9.0),
    ("Papua Selatan", "Merauke", -8.49, 140.40, 9.0),
];

/// Places outside Indonesia: (country, city, latitude, longitude, usual method, standard UTC
/// offset, summer-time rule). Their times are in that city's own clock, as people there read them.
#[allow(clippy::approx_constant)] // Kuala Lumpur sits at 3.14° N, not π
pub const WORLD: &[(&str, &str, f64, f64, usize, f64, Dst)] = &[
    ("Việt Nam", "Hà Nội", 21.03, 105.85, MWL, 7.0, Dst::No),
    ("Việt Nam", "TP Hồ Chí Minh", 10.82, 106.63, MWL, 7.0, Dst::No),
    ("Việt Nam", "Đà Nẵng", 16.05, 108.20, MWL, 7.0, Dst::No),
    ("Việt Nam", "Cần Thơ", 10.03, 105.78, MWL, 7.0, Dst::No),
    ("Việt Nam", "Châu Đốc", 10.70, 105.12, MWL, 7.0, Dst::No),
    ("Malaysia", "Kuala Lumpur", 3.14, 101.69, JAKIM, 8.0, Dst::No),
    ("Singapore", "Singapore", 1.35, 103.82, MUIS, 8.0, Dst::No),
    ("Brunei", "Bandar Seri Begawan", 4.90, 114.94, JAKIM, 8.0, Dst::No),
    ("Thailand", "Bangkok", 13.75, 100.50, MWL, 7.0, Dst::No),
    ("Philippines", "Manila", 14.60, 120.98, MWL, 8.0, Dst::No),
    ("Saudi Arabia", "Mecca", 21.42, 39.83, UMM_AL_QURA, 3.0, Dst::No),
    ("Saudi Arabia", "Medina", 24.47, 39.61, UMM_AL_QURA, 3.0, Dst::No),
    ("UAE", "Dubai", 25.20, 55.27, GULF, 4.0, Dst::No),
    ("Egypt", "Cairo", 30.04, 31.24, EGYPT, 2.0, Dst::No),
    ("Türkiye", "Istanbul", 41.01, 28.98, DIYANET, 3.0, Dst::No),
    ("Pakistan", "Karachi", 24.86, 67.01, KARACHI, 5.0, Dst::No),
    ("Bangladesh", "Dhaka", 23.81, 90.41, KARACHI, 6.0, Dst::No),
    ("India", "New Delhi", 28.61, 77.21, KARACHI, 5.5, Dst::No),
    ("China", "Beijing", 39.90, 116.40, MWL, 8.0, Dst::No),
    ("Japan", "Tokyo", 35.68, 139.69, MWL, 9.0, Dst::No),
    ("Korea", "Seoul", 37.57, 126.98, MWL, 9.0, Dst::No),
    ("Australia", "Sydney", -33.87, 151.21, MWL, 10.0, Dst::Au),
    ("United Kingdom", "London", 51.51, -0.13, MWL, 0.0, Dst::Eu),
    ("France", "Paris", 48.86, 2.35, UOIF, 1.0, Dst::Eu),
    ("Germany", "Berlin", 52.52, 13.40, MWL, 1.0, Dst::Eu),
    ("United States", "New York", 40.71, -74.01, ISNA, -5.0, Dst::Us),
    ("United States", "Los Angeles", 34.05, -118.24, ISNA, -8.0, Dst::Us),
    ("Canada", "Toronto", 43.65, -79.38, ISNA, -5.0, Dst::Us),
];

/// How a country's authority sets the times: Subuh and Isya by the sun's angle below the horizon
/// (or Isya some minutes after Maghrib), Ashar by shadow length (1 Shafi'i, 2 Hanafi), and the
/// minutes of caution added to each.
#[derive(Clone, Copy)]
pub struct Method {
    pub name: &'static str,
    fajr: f64,
    isha: Isha,
    asr_shadow: f64,
    safety_min: f64,
}

#[derive(Clone, Copy)]
enum Isha {
    Angle(f64),
    AfterMaghrib(f64),
}

pub const KEMENAG: usize = 0;
pub const MWL: usize = 1;
pub const ISNA: usize = 2;
pub const EGYPT: usize = 3;
pub const UMM_AL_QURA: usize = 4;
pub const KARACHI: usize = 5;
pub const JAKIM: usize = 6;
pub const MUIS: usize = 7;
pub const DIYANET: usize = 8;
pub const UOIF: usize = 9;
pub const GULF: usize = 10;

pub const METHODS: &[Method] = &[
    // The original's: Kemenag angles with a 2-minute ihtiyat (0x140031c46).
    Method { name: "Kemenag RI (Indonesia)", fajr: 20.0, isha: Isha::Angle(18.0), asr_shadow: 1.0, safety_min: 2.0 },
    Method { name: "Muslim World League", fajr: 18.0, isha: Isha::Angle(17.0), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "ISNA (North America)", fajr: 15.0, isha: Isha::Angle(15.0), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "Egyptian General Authority", fajr: 19.5, isha: Isha::Angle(17.5), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "Umm al-Qura (Makkah)", fajr: 18.5, isha: Isha::AfterMaghrib(90.0), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "University of Karachi (Hanafi)", fajr: 18.0, isha: Isha::Angle(18.0), asr_shadow: 2.0, safety_min: 0.0 },
    Method { name: "JAKIM (Malaysia)", fajr: 20.0, isha: Isha::Angle(18.0), asr_shadow: 1.0, safety_min: 2.0 },
    Method { name: "MUIS (Singapore)", fajr: 20.0, isha: Isha::Angle(18.0), asr_shadow: 1.0, safety_min: 2.0 },
    Method { name: "Diyanet (Türkiye)", fajr: 18.0, isha: Isha::Angle(17.0), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "UOIF (France)", fajr: 12.0, isha: Isha::Angle(12.0), asr_shadow: 1.0, safety_min: 0.0 },
    Method { name: "Gulf region", fajr: 19.5, isha: Isha::AfterMaghrib(90.0), asr_shadow: 1.0, safety_min: 0.0 },
];

/// Entries in the City list: the Indonesian cities, the world, then "My location" last.
pub fn place_count() -> usize {
    CITIES.len() + WORLD.len() + 1
}

pub fn here_index() -> usize {
    place_count() - 1
}

/// (region, city) for the City list; "My location" is named by the caller.
pub fn place_name(i: usize) -> (&'static str, &'static str) {
    match i {
        i if i < CITIES.len() => (CITIES[i].0, CITIES[i].1),
        i if i < CITIES.len() + WORLD.len() => (WORLD[i - CITIES.len()].0, WORLD[i - CITIES.len()].1),
        _ => ("", ""),
    }
}

/// Latitude and longitude of place `i` (`here` for "My location").
pub fn place_coords(i: usize, here: (f64, f64)) -> (f64, f64) {
    match i {
        i if i < CITIES.len() => (CITIES[i].2, CITIES[i].3),
        i if i < CITIES.len() + WORLD.len() => (WORLD[i - CITIES.len()].2, WORLD[i - CITIES.len()].3),
        _ => here,
    }
}

/// The method a place usually follows: Kemenag in Indonesia, the country's own elsewhere, and
/// the Muslim World League for "My location".
pub fn usual_method(i: usize) -> usize {
    match i {
        i if i < CITIES.len() => KEMENAG,
        i if i < CITIES.len() + WORLD.len() => WORLD[i - CITIES.len()].4,
        _ => MWL,
    }
}

/// The five prayers' names in English, the key i18n translates them by (the original's Indonesian
/// Subuh, Dzuhur, Ashar, Maghrib, Isya are the same five).
pub const NAMES: [&str; 5] = ["Fajr", "Dhuhr", "Asr", "Maghrib", "Isha"];
pub const DEFAULT_CITY: usize = 17; // Jakarta, settings default `prayer_city`
const SUNSET_ANGLE: f64 = 0.833;
const GUESS_HOURS: [f64; 5] = [5.0, 12.0, 13.0, 18.0, 18.0];

fn julian(y: i32, m: u32, d: u32) -> f64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y as f64 + 4716.0)).floor() + (30.6001 * (m as f64 + 1.0)).floor() + d as f64 + b - 1524.5
}

/// (declination in radians, equation of time in hours) at Julian day `jd`.
fn sun(jd: f64) -> (f64, f64) {
    let d = jd - 2451545.0;
    let g = (357.529 + 0.98560028 * d).to_radians();
    let q = 280.459 + 0.98564736 * d;
    let l = (q + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let e = (23.439 - 0.00000036 * d).to_radians();
    let ra = (e.cos() * l.sin()).atan2(l.cos()).to_degrees().rem_euclid(360.0) / 15.0;
    let eqt = q / 15.0 - ra;
    ((e.sin() * l.sin()).asin(), eqt - 24.0 * (eqt / 24.0).round())
}

/// How Ashar is reckoned: as the method does, or by one school for any method.
#[derive(Clone, Copy, PartialEq)]
pub enum Asr { Method, Shafii, Hanafi }

/// Minutes after local midnight for Subuh, Dzuhur, Ashar, Maghrib, Isya (before the per-prayer
/// adjustments) at place `idx` of the City list (`here` is "My location"'s latitude and longitude) with `method`
/// (None: the place's usual one) and Ashar by `asr` (a Hanafi in Europe can keep MWL's other times, say).
/// "My location" is in this PC's clock, other places in their own.
pub fn times_with(date: chrono::NaiveDate, idx: usize, here: (f64, f64), method: Option<usize>, asr: Asr) -> [u32; 5] {
    let mut m = METHODS[method.unwrap_or_else(|| usual_method(idx)).min(METHODS.len() - 1)];
    match asr {
        Asr::Method => {}
        Asr::Shafii => m.asr_shadow = 1.0,
        Asr::Hanafi => m.asr_shadow = 2.0,
    }
    let (lat, lng, tz) = place_at(date, idx, here);
    times_at(date, lat, lng, tz, &m)
}

/// Sunrise (Terbit) that day at place `idx`, in minutes after that place's midnight: shown on the
/// day arc, not reminded of.
pub fn sunrise_for(date: chrono::NaiveDate, idx: usize, here: (f64, f64)) -> u32 {
    let (lat, lng, tz) = place_at(date, idx, here);
    day_at(date, lat, lng, tz, &METHODS[KEMENAG]).1
}

/// The Hijri month names, in English (the key i18n translates them by).
pub const HIJRI_MONTHS: [&str; 12] = [
    "Muharram", "Safar", "Rabi' al-Awwal", "Rabi' al-Thani", "Jumada al-Ula", "Jumada al-Akhirah",
    "Rajab", "Sha'ban", "Ramadan", "Shawwal", "Dhu al-Qi'dah", "Dhu al-Hijjah",
];

/// (year, month 1-12, day) in the tabular Islamic calendar (30-year cycle, civil epoch).
/// shortcut: worked out, not sighted; it can differ by a day from a country's own calendar
/// (Umm al-Qura, Kemenag's rukyat); a sighting table would be needed to match one exactly.
pub fn hijri(date: chrono::NaiveDate) -> (i32, u32, u32) {
    use chrono::Datelike;
    let jd = date.num_days_from_ce() as i64 + 1_721_425; // Julian day number
    let mut l = jd - 1_948_440 + 10_632;
    let n = (l - 1) / 10_631;
    l = l - 10_631 * n + 354;
    let j = ((10_985 - l) / 5_316) * ((50 * l) / 17_719) + (l / 5_670) * ((43 * l) / 15_238);
    l = l - ((30 - j) / 15) * ((17_719 * j) / 50) - (j / 16) * ((15_238 * j) / 43) + 29;
    let m = (24 * l) / 709;
    let d = l - (709 * m) / 24;
    ((30 * n + j - 30) as i32, m as u32, d as u32)
}

/// Latitude, longitude and that day's UTC offset of place `idx`.
fn place_at(date: chrono::NaiveDate, idx: usize, here: (f64, f64)) -> (f64, f64, f64) {
    match idx {
        i if i < CITIES.len() => (CITIES[i].2, CITIES[i].3, CITIES[i].4),
        i if i < CITIES.len() + WORLD.len() => {
            let w = WORLD[i - CITIES.len()];
            let noon_utc = date.and_hms_opt(12, 0, 0).unwrap() - chrono::Duration::minutes((w.5 * 60.0) as i64);
            (w.2, w.3, w.5 + if crate::summer(w.6, w.5, noon_utc) { 1.0 } else { 0.0 })
        }
        _ => (here.0, here.1, pc_offset_hours(date)),
    }
}

/// This PC's UTC offset at noon that day, so summer time is right on the day it applies.
fn pc_offset_hours(date: chrono::NaiveDate) -> f64 {
    use chrono::{Local, Offset, TimeZone};
    let noon = date.and_hms_opt(12, 0, 0).unwrap();
    Local.from_local_datetime(&noon).single().map_or(0.0, |t| t.offset().fix().local_minus_utc() as f64 / 3600.0)
}

fn times_at(date: chrono::NaiveDate, lat: f64, lng: f64, tz: f64, m: &Method) -> [u32; 5] {
    day_at(date, lat, lng, tz, m).0
}

/// The five times, and sunrise (rounded down, with no safety minutes: it isn't a prayer time).
fn day_at(date: chrono::NaiveDate, lat: f64, lng: f64, tz: f64, m: &Method) -> ([u32; 5], u32) {
    use chrono::Datelike;
    let jd = julian(date.year(), date.month(), date.day()) - lng / (15.0 * 24.0);
    let phi = lat.to_radians();
    let mut t = GUESS_HOURS;
    let mut sunrise = 6.0;
    for _ in 0..2 {
        let p = t.map(|h| h / 24.0);
        let noon = |frac: f64| (12.0 - sun(jd + frac).1).rem_euclid(24.0);
        let angle_time = |angle_deg: f64, frac: f64, before_noon: bool| {
            let (decl, _) = sun(jd + frac);
            let h = ((-angle_deg.to_radians().sin() - decl.sin() * phi.sin()) / (decl.cos() * phi.cos())).clamp(-1.0, 1.0).acos().to_degrees() / 15.0;
            noon(frac) + if before_noon { -h } else { h }
        };
        let (decl_asr, _) = sun(jd + p[2]);
        let asr_angle = -(1.0 / (m.asr_shadow + (phi - decl_asr).abs().tan())).atan().to_degrees();
        let maghrib = angle_time(SUNSET_ANGLE, p[3], false);
        sunrise = angle_time(SUNSET_ANGLE, p[0], true);
        t = [
            angle_time(m.fajr, p[0], true),
            noon(p[1]),
            angle_time(asr_angle, p[2], false),
            maghrib,
            match m.isha {
                Isha::Angle(a) => angle_time(a, p[4], false),
                Isha::AfterMaghrib(min) => maghrib + min / 60.0,
            },
        ];
    }
    // Far from the equator in summer the sun may never sink to the Subuh or Isya angle: then they
    // sit no further from sunrise or sunset than angle/60 of the night (the "angle-based" rule).
    let night = 24.0 - (t[3] - sunrise);
    let fajr_most = night * m.fajr / 60.0;
    if !(sunrise - t[0]).is_finite() || sunrise - t[0] > fajr_most {
        t[0] = sunrise - fajr_most;
    }
    if let Isha::Angle(a) = m.isha {
        let isha_most = night * a / 60.0;
        if !(t[4] - t[3]).is_finite() || t[4] - t[3] > isha_most {
            t[4] = t[3] + isha_most;
        }
    }
    let local = |h: f64| (h + tz - lng / 15.0) * 60.0;
    let rise = if sunrise.is_finite() { (local(sunrise).floor() as i64).clamp(0, 1439) as u32 } else { 0 };
    (t.map(|h| ((local(h) + m.safety_min - 1e-6).ceil() as i64).clamp(0, 1439) as u32), rise)
}

#[cfg(test)]
mod tests {
    #[test]
    fn jakarta_is_plausible() {
        let t = super::times_with(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), super::DEFAULT_CITY, (0.0, 0.0), None, super::Asr::Method);
        // Hanafi Ashar with Kemenag's other times: only Ashar moves, and later.
        let h = super::times_with(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), super::DEFAULT_CITY, (0.0, 0.0), None, super::Asr::Hanafi);
        assert!(h[2] > t[2] + 20 && [0, 1, 3, 4].iter().all(|&i| h[i] == t[i]), "{t:?} {h:?}");
        // Kemenag schedule for Jakarta on 1 Jan 2026: Subuh 04:19, Dzuhur 11:58, Maghrib 18:15 (±3 min).
        assert!((256..263).contains(&t[0]), "{t:?}");
        assert!((715..722).contains(&t[1]), "{t:?}");
        assert!((1092..1099).contains(&t[3]), "{t:?}");
        assert!(t.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(super::CITIES.len(), 84);
        // Hijri: 1 Muharram 1446 was 7 July 2024 and 1 Ramadan 1445 was 11 March 2024 (Umm al-Qura);
        // the tabular calendar may be a day out, never more.
        let near = |y, m, d, ymd: (i32, u32, u32)| {
            let g = chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
            (-1..=1).any(|k| super::hijri(g + chrono::Duration::days(k)) == ymd)
        };
        assert!(near(2024, 7, 7, (1446, 1, 1)) && near(2024, 3, 11, (1445, 9, 1)));
        assert_eq!(super::hijri(chrono::NaiveDate::from_ymd_opt(2024, 7, 6).unwrap() - chrono::Duration::days(1)).1, 12);
        // Kemenag Terbit for Jakarta, 1 Jan 2026: 05:39 (±3 min), between Subuh and Dzuhur.
        let rise = super::sunrise_for(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), super::DEFAULT_CITY, (0.0, 0.0));
        assert!((336..343).contains(&rise) && rise > t[0] && rise < t[1], "{rise}");
        assert_eq!(super::CITIES[super::DEFAULT_CITY].1, "Jakarta");
    }

    #[test]
    fn world_methods_give_sensible_days() {
        use super::*;
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        // Mecca by Umm al-Qura: Isya 90 minutes after Maghrib.
        let mecca = CITIES.len() + WORLD.iter().position(|w| w.1 == "Mecca").unwrap();
        let (lat, lng) = (WORLD[mecca - CITIES.len()].2, WORLD[mecca - CITIES.len()].3);
        let t = times_at(date, lat, lng, 3.0, &METHODS[UMM_AL_QURA]);
        assert!((t[4] as i32 - t[3] as i32 - 90).abs() <= 1, "{t:?}");
        // Known table for Mecca, 21 Jun 2026: Fajr ~04:11, Dhuhr ~12:22, Maghrib ~19:07 (±4 min).
        assert!((247..255).contains(&t[0]) && (738..746).contains(&t[1]) && (1143..1151).contains(&t[3]), "{t:?}");
        // Hanafi Ashar comes later than Shafi'i at the same place.
        let karachi = times_at(date, 24.86, 67.01, 5.0, &METHODS[KARACHI]);
        let shafii = times_at(date, 24.86, 67.01, 5.0, &METHODS[MWL]);
        assert!(karachi[2] > shafii[2] + 30, "{karachi:?} {shafii:?}");
        // London at midsummer (MWL 18°): the sun never reaches 18°, so the night rule keeps Subuh
        // after 01:00 and Isya before midnight, in London's summer clock.
        let london = CITIES.len() + WORLD.iter().position(|w| w.1 == "London").unwrap();
        let t = times_with(date, london, (0.0, 0.0), None, super::Asr::Method);
        assert!(t[0] > 60 && t[0] < 4 * 60 && t[4] < 1439 && t[4] > 21 * 60 + 30, "{t:?}");
        assert!((12 * 60 + 55..13 * 60 + 5).contains(&t[1]), "London Dzuhur in BST: {t:?}");
        // Every place keeps the five in order.
        for i in 0..here_index() {
            let t = times_with(date, i, (0.0, 0.0), None, super::Asr::Method);
            assert!(t.windows(2).all(|w| w[0] <= w[1]), "{:?} {t:?}", place_name(i));
        }
    }
}
