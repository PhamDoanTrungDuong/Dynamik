//! Prayer times as the original computes them (0x140031c46): PrayTimes method with Kemenag
//! angles (Subuh 20°, Isya 18°, sunset 0.833°, Shafi'i Ashar), two refinement passes, then
//! minutes = ceil(time*60 + 2 - 1e-6). City table copied from the original's data (0x1400df470).

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

pub const NAMES: [&str; 5] = ["Subuh", "Dzuhur", "Ashar", "Maghrib", "Isya"];
pub const DEFAULT_CITY: usize = 17; // Jakarta, settings default `prayer_city`
const FAJR_ANGLE: f64 = 20.0;
const SUNSET_ANGLE: f64 = 0.833;
const ISHA_ANGLE: f64 = 18.0;
const SAFETY_MIN: f64 = 2.0;
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

/// Minutes after local midnight for Subuh, Dzuhur, Ashar, Maghrib, Isya on that date in city `idx`
/// (before the per-prayer adjustments).
pub fn times(date: chrono::NaiveDate, idx: usize) -> [u32; 5] {
    use chrono::Datelike;
    let (_, _, lat, lng, tz) = CITIES[idx.min(CITIES.len() - 1)];
    let jd = julian(date.year(), date.month(), date.day()) - lng / (15.0 * 24.0);
    let phi = lat.to_radians();
    let mut t = GUESS_HOURS;
    for _ in 0..2 {
        let p = t.map(|h| h / 24.0);
        let noon = |frac: f64| (12.0 - sun(jd + frac).1).rem_euclid(24.0);
        let angle_time = |angle_deg: f64, frac: f64, before_noon: bool| {
            let (decl, _) = sun(jd + frac);
            let h = ((-angle_deg.to_radians().sin() - decl.sin() * phi.sin()) / (decl.cos() * phi.cos())).clamp(-1.0, 1.0).acos().to_degrees() / 15.0;
            noon(frac) + if before_noon { -h } else { h }
        };
        let (decl_asr, _) = sun(jd + p[2]);
        let asr_angle = -(1.0 / (1.0 + (phi - decl_asr).abs().tan())).atan().to_degrees();
        t = [
            angle_time(FAJR_ANGLE, p[0], true),
            noon(p[1]),
            angle_time(asr_angle, p[2], false),
            angle_time(SUNSET_ANGLE, p[3], false),
            angle_time(ISHA_ANGLE, p[4], false),
        ];
    }
    t.map(|h| (((h + tz - lng / 15.0) * 60.0 + SAFETY_MIN - 1e-6).ceil() as i64).clamp(0, 1439) as u32)
}

#[cfg(test)]
mod tests {
    #[test]
    fn jakarta_is_plausible() {
        let t = super::times(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), super::DEFAULT_CITY);
        // Kemenag schedule for Jakarta on 1 Jan 2026: Subuh 04:19, Dzuhur 11:58, Maghrib 18:15 (±3 min).
        assert!((256..263).contains(&t[0]), "{t:?}");
        assert!((715..722).contains(&t[1]), "{t:?}");
        assert!((1092..1099).contains(&t[3]), "{t:?}");
        assert!(t.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(super::CITIES.len(), 84);
        assert_eq!(super::CITIES[super::DEFAULT_CITY].1, "Jakarta");
    }
}
