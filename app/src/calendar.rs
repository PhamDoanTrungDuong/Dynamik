//! Part C: today's events from a calendar's secret iCal address (Google Calendar › Settings ›
//! the calendar › "Secret address in iCal format"). Read only; fetched every 15 minutes.
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc, Weekday};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Instant,
};

const EVERY: std::time::Duration = std::time::Duration::from_secs(15 * 60);
const RETRY: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub title: String,
    pub start: Option<u32>, // minutes after midnight; None for an all-day event (or one carried over)
    pub dur: u32,           // minutes; for one carried over from yesterday, when it ends today
    pub carried: bool,      // began yesterday and runs past midnight into today
}

#[derive(Clone, Copy, PartialEq)]
pub enum Status { Off, Loading, Ready, Failed }

#[derive(Clone)]
pub struct Today {
    pub status: Status,
    pub events: Vec<Event>,
}

/// Keeps today's events for the address in `url` (empty = off) fresh.
pub fn watch(url: Arc<Mutex<String>>) -> Arc<Mutex<Today>> {
    let shared = Arc::new(Mutex::new(Today { status: Status::Off, events: vec![] }));
    let out = shared.clone();
    std::thread::spawn(move || {
        let mut have: Vec<Item> = vec![];
        // (address, when last tried, whether it worked), and the day the list was made for
        let mut last: Option<(String, Instant, bool)> = None;
        let mut day = None;
        loop {
            let want = url.lock().unwrap().trim().replacen("webcal://", "https://", 1);
            if want.is_empty() || !want.starts_with("https://") {
                if last.take().is_some() || day.is_some() {
                    have.clear();
                    day = None;
                    *out.lock().unwrap() = Today { status: Status::Off, events: vec![] };
                    crate::tray::wake();
                }
            } else {
                let due = last.as_ref().is_none_or(|(u, when, ok)| *u != want || when.elapsed() > if *ok { EVERY } else { RETRY });
                let mut fresh = false;
                if due {
                    if last.as_ref().is_none_or(|(u, ..)| *u != want) {
                        have.clear();
                        out.lock().unwrap().status = Status::Loading;
                        crate::tray::wake();
                    }
                    let got = crate::online::get(&want).filter(|b| b.contains("BEGIN:VCALENDAR"));
                    let ok = got.is_some();
                    if let Some(body) = got {
                        have = parse(&body);
                    }
                    last = Some((want, Instant::now(), ok));
                    fresh = true;
                }
                let today = Local::now().date_naive();
                if fresh || day != Some(today) {
                    day = Some(today);
                    let ok = last.as_ref().is_some_and(|l| l.2) || !have.is_empty();
                    *out.lock().unwrap() = Today { status: if ok { Status::Ready } else { Status::Failed }, events: on(&have, today) };
                    crate::tray::wake();
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(20));
        }
    });
    shared
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Freq { Daily, Weekly, Monthly, Yearly }

#[derive(Clone, Debug)]
struct Rule {
    freq: Freq,
    interval: i64,
    until: Option<NaiveDate>,
    count: Option<u32>,
    by_day: Vec<(i32, Weekday)>, // (0 = every, n = nth in the month, -1 = last)
    by_month_day: Vec<i32>,
}

/// One VEVENT, its times already in this PC's local time.
#[derive(Clone, Debug)]
struct Item {
    title: String,
    start: NaiveDateTime,
    all_day: bool,
    minutes: i64, // length; for all-day events a multiple of a day
    rule: Option<Rule>,
    skip: HashSet<NaiveDate>, // EXDATE, and days moved by a RECURRENCE-ID copy
}

/// "20261121", "20261121T090000" or "20261121T020000Z" into local time (and whether it is a date).
/// shortcut: a TZID time is read as this PC's time zone; right when the calendar's zone is the PC's,
/// which is the usual case; a full zone table would be needed otherwise.
fn when(v: &str) -> Option<(NaiveDateTime, bool)> {
    let v = v.trim();
    if v.len() == 8 {
        return Some((NaiveDate::parse_from_str(v, "%Y%m%d").ok()?.and_hms_opt(0, 0, 0)?, true));
    }
    let utc = v.ends_with('Z');
    let t = NaiveDateTime::parse_from_str(v.trim_end_matches('Z'), "%Y%m%dT%H%M%S").ok()?;
    Some((if utc { Utc.from_utc_datetime(&t).with_timezone(&Local).naive_local() } else { t }, false))
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "MO" => Weekday::Mon, "TU" => Weekday::Tue, "WE" => Weekday::Wed, "TH" => Weekday::Thu,
        "FR" => Weekday::Fri, "SA" => Weekday::Sat, "SU" => Weekday::Sun, _ => return None,
    })
}

fn rule(v: &str) -> Option<Rule> {
    let mut r = Rule { freq: Freq::Daily, interval: 1, until: None, count: None, by_day: vec![], by_month_day: vec![] };
    let mut freq = None;
    for part in v.split(';') {
        let (k, val) = part.split_once('=')?;
        match k {
            "FREQ" => freq = Some(match val { "DAILY" => Freq::Daily, "WEEKLY" => Freq::Weekly, "MONTHLY" => Freq::Monthly, "YEARLY" => Freq::Yearly, _ => return None }),
            "INTERVAL" => r.interval = val.parse::<i64>().ok()?.max(1),
            "UNTIL" => r.until = when(val).map(|w| w.0.date()),
            "COUNT" => r.count = val.parse().ok(),
            "BYDAY" => {
                for d in val.split(',') {
                    let (n, code) = d.split_at(d.len().saturating_sub(2));
                    r.by_day.push((if n.is_empty() || n == "+" { 0 } else { n.trim_start_matches('+').parse().ok()? }, weekday(code)?));
                }
            }
            "BYMONTHDAY" => r.by_month_day = val.split(',').filter_map(|d| d.parse().ok()).collect(),
            _ => {} // WKST and the rest: the usual Monday start is assumed
        }
    }
    r.freq = freq?;
    Some(r)
}

/// The VEVENTs of an .ics file. Cancelled events are left out.
fn parse(ics: &str) -> Vec<Item> {
    // Long lines are folded: a line starting with a space or tab continues the one before.
    let mut lines: Vec<String> = vec![];
    for l in ics.lines() {
        match (l.strip_prefix(' ').or_else(|| l.strip_prefix('\t')), lines.last_mut()) {
            (Some(more), Some(prev)) => prev.push_str(more),
            _ => lines.push(l.to_string()),
        }
    }
    let mut items = vec![];
    let mut moved: Vec<(String, NaiveDate)> = vec![];
    let mut uids: Vec<String> = vec![];
    // (item, end, uid, cancelled, the series day it replaces)
    let mut cur: Option<(Item, Option<NaiveDateTime>, String, bool, Option<NaiveDate>)> = None;
    for l in &lines {
        if l == "BEGIN:VEVENT" {
            let blank = Item { title: String::new(), start: NaiveDateTime::MIN, all_day: false, minutes: 0, rule: None, skip: HashSet::new() };
            cur = Some((blank, None, String::new(), false, None));
            continue;
        }
        let Some((it, end, uid, cancelled, replaces)) = cur.as_mut() else { continue };
        if l == "END:VEVENT" {
            let (mut it, end, uid, cancelled, replaces) = cur.take().unwrap();
            if let Some(day) = replaces {
                moved.push((uid.clone(), day)); // a cancelled copy still takes its day out of the series
            }
            if cancelled || it.start == NaiveDateTime::MIN {
                continue;
            }
            it.minutes = match end {
                Some(e) => (e - it.start).num_minutes().max(0),
                None if it.all_day => 24 * 60,
                None => 0,
            };
            uids.push(uid);
            items.push(it);
            continue;
        }
        let Some((head, value)) = l.split_once(':') else { continue };
        let name = head.split(';').next().unwrap_or("");
        match name {
            "SUMMARY" => it.title = value.replace("\\n", " ").replace("\\N", " ").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\"),
            "DTSTART" => if let Some((t, d)) = when(value) { it.start = t; it.all_day = d },
            "DTEND" => *end = when(value).map(|w| w.0),
            "RRULE" => it.rule = rule(value),
            "EXDATE" => it.skip.extend(value.split(',').filter_map(when).map(|w| w.0.date())),
            "UID" => *uid = value.to_string(),
            "RECURRENCE-ID" => *replaces = when(value).map(|w| w.0.date()),
            "STATUS" => *cancelled = value == "CANCELLED",
            _ => {}
        }
    }
    // A RECURRENCE-ID copy replaces one day of its series (it is listed on its own day instead).
    for (uid, day) in moved {
        for (it, u) in items.iter_mut().zip(&uids) {
            if *u == uid && it.rule.is_some() {
                it.skip.insert(day);
            }
        }
    }
    items
}

fn days_in_month(d: NaiveDate) -> u32 {
    let next = if d.month() == 12 { NaiveDate::from_ymd_opt(d.year() + 1, 1, 1) } else { NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1) };
    next.unwrap().pred_opt().unwrap().day()
}

/// Whether the rule (ignoring COUNT, UNTIL and skipped days) starts an occurrence on `d`.
fn rule_hits(r: &Rule, start: NaiveDate, d: NaiveDate) -> bool {
    let day_ok = |d: NaiveDate| {
        let dim = days_in_month(d) as i32;
        let mday = d.day() as i32;
        let by_day = r.by_day.iter().any(|&(n, w)| {
            w == d.weekday() && (n == 0 || (n > 0 && (mday - 1) / 7 + 1 == n) || (n < 0 && (dim - mday) / 7 + 1 == -n))
        });
        let by_mday = r.by_month_day.iter().any(|&m| m == mday || (m < 0 && dim + m + 1 == mday));
        match (r.by_day.is_empty(), r.by_month_day.is_empty()) {
            (true, true) => None,
            (false, true) => Some(by_day),
            (true, false) => Some(by_mday),
            (false, false) => Some(by_day && by_mday),
        }
    };
    match r.freq {
        Freq::Daily => (d - start).num_days() % r.interval == 0 && day_ok(d).unwrap_or(true),
        Freq::Weekly => {
            let monday = |x: NaiveDate| x - Duration::days(x.weekday().num_days_from_monday() as i64);
            (monday(d) - monday(start)).num_days() / 7 % r.interval == 0 && day_ok(d).unwrap_or(d.weekday() == start.weekday())
        }
        Freq::Monthly => {
            let months = (d.year() - start.year()) as i64 * 12 + d.month() as i64 - start.month() as i64;
            months % r.interval == 0 && day_ok(d).unwrap_or(d.day() == start.day())
        }
        Freq::Yearly => {
            (d.year() - start.year()) as i64 % r.interval == 0 && d.month() == start.month() && day_ok(d).unwrap_or(d.day() == start.day())
        }
    }
}

/// Whether one occurrence of `it` starts on `d`.
fn starts_on(it: &Item, d: NaiveDate) -> bool {
    let start = it.start.date();
    if d < start || it.skip.contains(&d) {
        return false;
    }
    let Some(r) = &it.rule else { return d == start };
    if d == start {
        return true; // DTSTART is always the first occurrence
    }
    if r.until.is_some_and(|u| d > u) || !rule_hits(r, start, d) {
        return false;
    }
    // COUNT counts occurrences from the first one (skipped days still count, as RFC 5545 says).
    r.count.is_none_or(|n| {
        let mut seen = 0;
        let mut x = start;
        while x < d && seen < n {
            seen += (x == start || rule_hits(r, start, x)) as u32;
            x = x.succ_opt().unwrap();
        }
        seen < n
    })
}

/// The events on `day`, all-day ones first, then by start time.
fn on(items: &[Item], day: NaiveDate) -> Vec<Event> {
    let mut out: Vec<Event> = items
        .iter()
        .filter(|it| {
            if it.all_day {
                // A several-day event shows on each of its days (DTEND is the day after the last).
                let span = (it.minutes / (24 * 60)).clamp(1, 31);
                (0..span).any(|k| starts_on(it, day - Duration::days(k)))
            } else {
                starts_on(it, day)
            }
        })
        .map(|it| Event {
            title: it.title.clone(), // empty when the event has none
            start: (!it.all_day).then(|| it.start.time().num_seconds_from_midnight() / 60),
            dur: if it.all_day { 0 } else { it.minutes.min(24 * 60) as u32 },
            carried: false,
        })
        .collect();
    // Timed events from yesterday still going after midnight show too, with when they end.
    let yesterday = day - Duration::days(1);
    out.extend(items.iter().filter(|it| !it.all_day && starts_on(it, yesterday)).filter_map(|it| {
        let ends = (it.start.time().num_seconds_from_midnight() / 60) as i64 + it.minutes - 24 * 60;
        (ends > 0).then(|| Event { title: it.title.clone(), start: None, dur: ends.min(24 * 60) as u32, carried: true })
    }));
    out.sort_by_key(|e| e.start.map_or(-1, |s| s as i64));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate { NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap() }

    const ICS: &str = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\nDTSTART;TZID=Asia/Ho_Chi_Minh:20261102T090000\r\nDTEND;TZID=Asia/Ho_Chi_Minh:20261102T093000\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR\r\nEXDATE;TZID=Asia/Ho_Chi_Minh:20261111T090000\r\nUID:standup\r\nSUMMARY:Standup\\, team\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART;TZID=Asia/Ho_Chi_Minh:20261120T100000\r\nDTEND;TZID=Asia/Ho_Chi_Minh:20261120T103000\r\n\
RECURRENCE-ID;TZID=Asia/Ho_Chi_Minh:20261120T090000\r\nUID:standup\r\nSUMMARY:Standup (moved)\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20261120\r\nDTEND;VALUE=DATE:20261123\r\nSUMMARY:Trip to Đà\r\n  Lạt\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260315\r\nRRULE:FREQ=YEARLY\r\nSUMMARY:Birthday\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART:20261124T090000\r\nRRULE:FREQ=MONTHLY;BYDAY=-1TU;COUNT=2\r\nSUMMARY:Review\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART:20261121T140000\r\nSTATUS:CANCELLED\r\nSUMMARY:Gone\r\nEND:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn weekly_series_with_skipped_and_moved_days() {
        let items = parse(ICS);
        let titles = |day: &str| on(&items, d(day)).into_iter().map(|e| e.title).collect::<Vec<_>>();
        assert_eq!(on(&items, d("2026-11-02")), vec![Event { title: "Standup, team".into(), start: Some(540), dur: 30, carried: false }]);
        assert!(titles("2026-11-03").is_empty()); // a Tuesday
        assert!(titles("2026-11-11").is_empty()); // EXDATE
        assert_eq!(titles("2026-11-13"), vec!["Standup, team"]);
        // The 20th's standup moved to 10:00; the trip (folded title) shows first, as an all-day event.
        let fri = on(&items, d("2026-11-20"));
        assert_eq!(fri.iter().map(|e| (e.title.as_str(), e.start)).collect::<Vec<_>>(), vec![("Trip to Đà Lạt", None), ("Standup (moved)", Some(600))]);
        assert_eq!(titles("2026-11-22"), vec!["Trip to Đà Lạt"]); // still on the trip
        assert_eq!(titles("2026-11-23"), vec!["Standup, team"]); // a Monday; the trip is over (DTEND is the day after)
        assert!(titles("2026-11-21").iter().all(|t| t != "Gone"));
    }

    #[test]
    fn yearly_and_monthly_rules() {
        let items = parse(ICS);
        let has = |day: &str, t: &str| on(&items, d(day)).iter().any(|e| e.title == t);
        assert!(has("2027-03-15", "Birthday") && !has("2027-03-16", "Birthday"));
        assert!(has("2026-11-24", "Review")); // last Tuesday of November
        assert!(has("2026-12-29", "Review")); // last Tuesday of December
        assert!(!has("2027-01-26", "Review")); // COUNT=2 is used up
        assert!(!has("2026-12-22", "Review"));
    }

    const MORE: &str = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\nUID:gym\r\nDTSTART:20261102T060000\r\nRRULE:FREQ=DAILY;INTERVAL=3;UNTIL=20261114T235959Z\r\nSUMMARY:Gym\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:rent\r\nDTSTART;VALUE=DATE:20261015\r\nDTEND;VALUE=DATE:20261016\r\nRRULE:FREQ=MONTHLY;BYMONTHDAY=15\r\nSUMMARY:Rent\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:class\r\nDTSTART:20261103T013000Z\r\nDTEND:20261103T030000Z\r\nRRULE:FREQ=WEEKLY\r\nEXDATE:20261110T013000Z\r\nSUMMARY:Class\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:class\r\nRECURRENCE-ID:20261117T013000Z\r\nDTSTART:20261117T013000Z\r\nSTATUS:CANCELLED\r\nSUMMARY:Class\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:last\r\nDTSTART:20261130T090000\r\nRRULE:FREQ=MONTHLY;BYMONTHDAY=-1;COUNT=3\r\nSUMMARY:Month end\r\nEND:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn overnight_events_carry_into_the_next_day() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART:20261121T223000\r\nDTEND:20261122T013000\r\nSUMMARY:Night shift\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let items = parse(ics);
        assert_eq!(on(&items, d("2026-11-21")), vec![Event { title: "Night shift".into(), start: Some(1350), dur: 180, carried: false }]);
        // The next day: carried over, ending at 01:30, with no start to remind of.
        assert_eq!(on(&items, d("2026-11-22")), vec![Event { title: "Night shift".into(), start: None, dur: 90, carried: true }]);
        assert!(on(&items, d("2026-11-23")).is_empty());
    }

    #[test]
    fn daily_interval_until_and_month_days() {
        let items = parse(MORE);
        let has = |day: &str, t: &str| on(&items, d(day)).iter().any(|e| e.title == t);
        // Every third day from the 2nd, until the 14th.
        assert!(has("2026-11-02", "Gym") && has("2026-11-05", "Gym") && has("2026-11-14", "Gym"));
        assert!(!has("2026-11-03", "Gym") && !has("2026-11-17", "Gym"));
        // The 15th of every month, all day.
        assert!(has("2026-11-15", "Rent") && has("2027-02-15", "Rent") && !has("2026-11-16", "Rent"));
        // The last day of the month, 3 times: Nov 30, Dec 31, Jan 31; not Feb 28.
        assert!(has("2026-11-30", "Month end") && has("2026-12-31", "Month end") && has("2027-01-31", "Month end"));
        assert!(!has("2027-02-28", "Month end"));
    }

    #[test]
    fn utc_series_with_a_skipped_and_a_cancelled_day() {
        let items = parse(MORE);
        // Weekly from Tuesday 3 Nov 01:30 UTC, read on this PC's own calendar day.
        let local = |s: &str| when(s).unwrap().0.date();
        let titles = |day: NaiveDate| on(&items, day).into_iter().map(|e| e.title).collect::<Vec<_>>();
        assert_eq!(titles(local("20261103T013000Z")), vec!["Class"]);
        assert!(titles(local("20261110T013000Z")).is_empty(), "EXDATE in UTC");
        assert!(titles(local("20261117T013000Z")).is_empty(), "a cancelled copy takes its day out");
        assert_eq!(titles(local("20261124T013000Z")), vec!["Class"]);
        let e = &on(&items, local("20261124T013000Z"))[0];
        assert_eq!(e.dur, 90);
    }

    /// A real Google Calendar iCal file (the public Vietnamese holidays): cargo test live -- --ignored
    #[test]
    #[ignore]
    fn live_google_holidays() {
        let url = "https://calendar.google.com/calendar/ical/en.vietnamese%23holiday%40group.v.calendar.google.com/public/basic.ics";
        let body = crate::online::get(url).expect("Google answered");
        let items = parse(&body);
        assert!(items.len() > 10, "{} events", items.len());
        let new_year = on(&items, d("2027-01-01"));
        assert!(new_year.iter().any(|e| e.start.is_none()), "{new_year:?}");
        println!("{} events; 1 Jan 2027: {:?}", items.len(), new_year);
    }

    #[test]
    fn utc_times_become_local() {
        let (t, all_day) = when("20261121T020000Z").unwrap();
        let expect = Utc.with_ymd_and_hms(2026, 11, 21, 2, 0, 0).unwrap().with_timezone(&Local).naive_local();
        assert_eq!((t, all_day), (expect, false));
        assert!(when("20261121").unwrap().1);
        assert!(when("nonsense").is_none());
    }
}
