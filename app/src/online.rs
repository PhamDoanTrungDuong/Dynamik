//! Part B's two things that need the network: the weather (MET Norway, free, no key) and
//! currency rates (open.er-api.com, free, no key). Both are fetched only when used and kept in memory.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use windows::{core::HSTRING, Foundation::Uri, Web::Http::HttpClient};

/// MET Norway asks every app to name itself and how to reach its maker.
const AGENT: &str = "MyNotch/0.1 (+https://github.com/PhamDoanTrungDuong/Dynamik)";
const WEATHER_EVERY: Duration = Duration::from_secs(30 * 60);
const RATES_FOR: Duration = Duration::from_secs(12 * 3600);
const RETRY_AFTER: Duration = Duration::from_secs(60);

pub fn get(url: &str) -> Option<String> {
    // WinRT needs this once on each thread that uses it; again is harmless.
    let _ = unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) };
    let client = HttpClient::new().ok()?;
    // Added as is: the typed UserAgent parser turns down some valid names, and MET then answers 403.
    let _ = client.DefaultRequestHeaders().ok()?.TryAppendWithoutValidation(&HSTRING::from("User-Agent"), &HSTRING::from(AGENT));
    let uri = Uri::CreateUri(&HSTRING::from(url)).ok()?;
    Some(client.GetStringAsync(&uri).ok()?.get().ok()?.to_string())
}

/// Why Gemini gave no answer.
#[derive(Debug, PartialEq)]
pub enum AskError {
    Key,     // turned down: the key is wrong or not allowed
    Busy,    // too many questions for the free tier just now
    Offline, // no answer at all
    Empty,   // answered, but with no text (e.g. blocked)
}

/// "Latest" follows Google's current Flash model, so the app keeps working when one is retired.
const GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent";

/// One question to Gemini with the user's own key. `rule` tells it how to answer.
pub fn gemini(key: &str, rule: &str, question: &str) -> Result<String, AskError> {
    let _ = unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) };
    let body = format!(
        r#"{{"systemInstruction":{{"parts":[{{"text":{}}}]}},"contents":[{{"role":"user","parts":[{{"text":{}}}]}}]}}"#,
        json_str(rule),
        json_str(question)
    );
    let send = || -> windows::core::Result<(i32, String)> {
        use windows::{Storage::Streams::UnicodeEncoding, Web::Http::HttpStringContent};
        let client = HttpClient::new()?;
        let headers = client.DefaultRequestHeaders()?;
        // In a header, not the address, so the key never shows up in logs of the URL.
        headers.TryAppendWithoutValidation(&HSTRING::from("x-goog-api-key"), &HSTRING::from(key))?;
        let content = HttpStringContent::CreateFromStringWithEncodingAndMediaType(&HSTRING::from(body.as_str()), UnicodeEncoding::Utf8, &HSTRING::from("application/json"))?;
        let reply = client.PostAsync(&Uri::CreateUri(&HSTRING::from(GEMINI))?, &content)?.get()?;
        let status = reply.StatusCode()?.0;
        Ok((status, reply.Content()?.ReadAsStringAsync()?.get()?.to_string()))
    };
    match send() {
        Err(_) => Err(AskError::Offline),
        Ok((200, text)) => gemini_text(&text).ok_or(AskError::Empty),
        Ok((429, _)) => Err(AskError::Busy),
        Ok((400 | 401 | 403, _)) => Err(AskError::Key),
        Ok(_) => Err(AskError::Offline),
    }
}

/// The answer's text: every text part of the first candidate, joined.
pub fn gemini_text(body: &str) -> Option<String> {
    let mut rest = &body[body.find("\"candidates\"")?..];
    let mut out = String::new();
    while let Some(i) = rest.find("\"text\"") {
        rest = &rest[i..];
        out += &crate::media::json_string(rest, "text")?;
        rest = &rest[6..];
    }
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

/// A JSON string literal.
fn json_str(s: &str) -> String {
    let mut o = String::from('"');
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            '\r' => o += "\\r",
            '\t' => o += "\\t",
            c if (c as u32) < 0x20 => o += &format!("\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[derive(Clone)]
pub struct Weather {
    pub temp: f64,      // °C now
    pub symbol: String, // MET symbol for the next hour, e.g. "partlycloudy_day"
}

/// Keeps the weather for `place` fresh (every 30 minutes, at once when the place changes).
/// `place` is None while no weather widget is shown, and then nothing is fetched.
pub fn weather_watch(place: Arc<Mutex<Option<(f64, f64)>>>) -> Arc<Mutex<Option<Weather>>> {
    let shared = Arc::new(Mutex::new(None));
    let out = shared.clone();
    std::thread::spawn(move || {
        // (place, when it was last fetched or tried, whether that worked)
        let mut last: Option<((f64, f64), Instant, bool)> = None;
        loop {
            let want = *place.lock().unwrap();
            if let Some(at) = want {
                let due = last.is_none_or(|(p, when, ok)| p != at || when.elapsed() > if ok { WEATHER_EVERY } else { RETRY_AFTER });
                if due {
                    // Two decimals (about 1 km) is plenty for the weather and doesn't hand out "My location" exactly.
                    let url = format!("https://api.met.no/weatherapi/locationforecast/2.0/compact?lat={:.2}&lon={:.2}", at.0, at.1);
                    let got = get(&url).as_deref().and_then(parse_weather);
                    let ok = got.is_some();
                    if ok || last.is_some_and(|(p, ..)| p != at) {
                        *out.lock().unwrap() = got; // a new place never shows the old place's weather
                        crate::tray::wake();
                    }
                    last = Some((at, Instant::now(), ok));
                }
            }
            std::thread::sleep(Duration::from_secs(20));
        }
    });
    shared
}

/// The first forecast step: its temperature and the next hour's symbol.
pub fn parse_weather(body: &str) -> Option<Weather> {
    let series = &body[body.find("\"timeseries\"")?..];
    let temp = crate::media::json_number(series, "air_temperature")?;
    let next = &series[series.find("\"next_1_hours\"").or_else(|| series.find("\"next_6_hours\""))?..];
    let symbol = crate::media::json_string(next, "symbol_code")?;
    Some(Weather { temp, symbol })
}

/// Rates against the US dollar, fetched on first use and kept for 12 hours.
#[derive(Default)]
pub struct Rates {
    per_usd: HashMap<String, f64>,
    got: Option<Instant>,   // last success
    tried: Option<Instant>, // last attempt, so a failure is retried a minute later, not every frame
    busy: bool,
}

pub type SharedRates = Arc<Mutex<Rates>>;

/// `amount` of `from` in `to`, when the rates are here; otherwise starts fetching them (the
/// notch is woken when they arrive) and gives None.
pub fn convert(rates: &SharedRates, amount: f64, from: &str, to: &str) -> Option<f64> {
    let mut r = rates.lock().unwrap();
    let fresh = r.got.is_some_and(|t| t.elapsed() < RATES_FOR);
    let may_try = r.tried.is_none_or(|t| t.elapsed() > RETRY_AFTER);
    if !fresh && !r.busy && may_try {
        r.busy = true;
        r.tried = Some(Instant::now());
        let rates = rates.clone();
        std::thread::spawn(move || {
            let got = get("https://open.er-api.com/v6/latest/USD").map(|b| parse_rates(&b)).unwrap_or_default();
            let mut r = rates.lock().unwrap();
            if !got.is_empty() {
                r.per_usd = got;
                r.got = Some(Instant::now());
            }
            r.busy = false;
            crate::tray::wake();
        });
    }
    let (a, b) = (r.per_usd.get(&from.to_uppercase())?, r.per_usd.get(&to.to_uppercase())?);
    Some(amount / a * b)
}

/// What became of the rates: still coming, here (and whether a code is known), or unreachable.
pub enum RatesState {
    Coming,
    Here, // here, but without one of the two codes
    Offline,
}

pub fn rates_state(rates: &SharedRates) -> RatesState {
    let r = rates.lock().unwrap();
    if r.busy {
        RatesState::Coming
    } else if r.per_usd.is_empty() {
        if r.tried.is_some() { RatesState::Offline } else { RatesState::Coming }
    } else {
        RatesState::Here
    }
}

/// `"rates":{"USD":1,"VND":25400.5,...}` into a map.
pub fn parse_rates(body: &str) -> HashMap<String, f64> {
    let Some(start) = body.find("\"rates\"") else { return HashMap::new() };
    let rest = &body[start..];
    let Some(open) = rest.find('{') else { return HashMap::new() };
    let close = rest[open..].find('}').map_or(rest.len(), |c| open + c);
    rest[open + 1..close]
        .split(',')
        .filter_map(|pair| {
            let (k, v) = pair.split_once(':')?;
            Some((k.trim().trim_matches('"').to_string(), v.trim().parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_first_forecast_step() {
        // Trimmed from a MET Norway compact answer (units first, then the time steps).
        let body = r#"{"type":"Feature","properties":{"meta":{"units":{"air_temperature":"celsius"}},
            "timeseries":[{"time":"2026-11-21T03:00:00Z","data":{"instant":{"details":{"air_pressure_at_sea_level":1009.3,"air_temperature":27.4}},
            "next_12_hours":{"summary":{"symbol_code":"rainshowers_day"}},"next_1_hours":{"summary":{"symbol_code":"partlycloudy_day"},"details":{}}}},
            {"time":"2026-11-21T04:00:00Z","data":{"instant":{"details":{"air_temperature":28.1}}}}]}}"#;
        let w = parse_weather(body).unwrap();
        assert_eq!(w.temp, 27.4);
        assert_eq!(w.symbol, "partlycloudy_day");
        assert!(parse_weather("{}").is_none());
    }

    #[test]
    fn reads_gemini_answers() {
        // Shaped like a generateContent answer, with the answer split over two parts.
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"Hà Nội là \"thủ đô\"\n"},{"text":"của Việt Nam."}],"role":"model"},"finishReason":"STOP"}],
            "usageMetadata":{"promptTokenCount":9,"promptTokensDetails":[{"modality":"TEXT","tokenCount":9}]},"modelVersion":"gemini-flash"}"#;
        assert_eq!(gemini_text(body).unwrap(), "Hà Nội là \"thủ đô\"\ncủa Việt Nam.");
        assert!(gemini_text(r#"{"candidates":[{"finishReason":"SAFETY"}]}"#).is_none());
        assert!(gemini_text(r#"{"error":{"message":"API key not valid"}}"#).is_none());
        assert_eq!(json_str("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
    }

    #[test]
    fn reads_rates() {
        let body = r#"{"result":"success","base_code":"USD","rates":{"USD":1,"VND":25400.5,"EUR":0.92,"JPY":155.1},"time_eol_unix":0}"#;
        let r = parse_rates(body);
        assert_eq!(r["VND"], 25400.5);
        assert_eq!(r.len(), 4);
        assert!(parse_rates("no rates").is_empty());
    }

    /// The real services, once by hand: cargo test live -- --ignored
    #[test]
    #[ignore]
    fn live_met_norway_and_rates() {
        let body = get("https://api.met.no/weatherapi/locationforecast/2.0/compact?lat=21.0300&lon=105.8500").expect("MET answered");
        let w = parse_weather(&body).expect("MET answer parsed");
        assert!((-40.0..60.0).contains(&w.temp) && !w.symbol.is_empty(), "{} {}", w.temp, w.symbol);
        let rates = parse_rates(&get("https://open.er-api.com/v6/latest/USD").expect("rates answered"));
        assert!(rates["VND"] > 10_000.0 && rates["EUR"] < 2.0, "{:?} {:?}", rates.get("VND"), rates.get("EUR"));
        println!("Hanoi now: {} °C, {}; 1 USD = {} VND", w.temp, w.symbol, rates["VND"]);
    }

    /// Gemini answers a made-up key with "not valid", which proves the request itself is well formed.
    #[test]
    #[ignore]
    fn live_gemini_turns_a_bad_key_down() {
        assert_eq!(gemini("not-a-real-key", "Answer briefly.", "Hi"), Err(AskError::Key));
    }
}
