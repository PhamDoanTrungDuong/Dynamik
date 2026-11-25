# Mechanics sheet

Original: Nusa 1.2.0.0, an Inno Setup installer carrying one Rust program, `nusa.exe`
("A living notch for Windows"). Addresses are in that `nusa.exe`; everything here was read
from its code and data with the decompiler (nothing was run).

## Settings (table at 0x1400e1320: 50 entries of name + default; file `settings.txt`, `key=value`)
| Setting | Default | Used for |
|---|---|---|
| sound, lyrics, todo_start, todo_end | 1 | reminder sounds, task-start and time-up reminders |
| snooze | 10 | "Later" on a task waits this many minutes (choices 5 / 10 / 15) |
| week_monday | 0 | the week starts on Sunday |
| focus / short_break / long_break | 25 / 5 / 15 min | Pomodoro lengths |
| open_click, shelf_clear | 0 | open on hover; keep the shelf between runs |
| popup_battery | 1 | battery pop-ups |
| prayer | 0 | prayer times start off |
| prayer_city | 17 | Jakarta (index into the 84-city table) |
| prayer_reminder / prayer_early | 1 / 0 | remind at prayer time; not 10 minutes before |
| prayer_adj_subuh … prayer_adj_isya | 10 | per-prayer adjustment = value − 10 minutes (0x1400863af) |
| clipboard, clipboard_keep | 0 | copied-text history starts **off** |
| status_ring | 0 | ring shows water |
| widget_city1 / widget_city2 | 5 / 13 | Tokyo / London |
| widget_event | 1 | countdown to Ramadan |

## Water (state set up at 0x140049847, file `water.txt`)
- Keys (format pieces at 0x1400e31e0): `bottle, goal, start, end, day, taken, inb, fired, vis, hours, on`.
- Defaults: goal **1500 ml**, bottle 500 ml, bottle starts full, active **08:00–17:00**, reminders on.
- Goal choices: 1.5 / 2.0 / 2.5 / 3.0 L (labels in 0x140048f51).
- Bottle: steps of 50 ml, 100–2000 ml (0x140053ace, 0x140053bfc; loader 0x14004a7b2).
- Active hours: steps of 30 min; "from" ≤ 23:00; "until" ≥ from + 60 and ≤ 24:00 (0x140053be3…0x140053c78, loader 0x14004aa9e).
- Plan (0x14004ab75):
  - span = max(until − from, 60)
  - reminders = min(clamp(round(goal / 25), 1, 64), max(floor(span / 15), 1))
  - ml each = max(round(goal / reminders), 1); interval = span / reminders
  - refills = max(ceil(goal / bottle) − 1, 0)
  - Defaults give 36 reminders, every 15 min, 42 ml each, 2 refills.
- Schedule (0x140001fe5, re-armed every 15 s): reminder i is due at from + (i + 0.5) × interval; it shows if
  noticed within 2 minutes and only while today's total is below the goal; each is shown once a day (bit mask `fired`).
- Refill instead of drink when the bottle holds less than 0.75 × one drink (0x140002222).
- Snooze: now + 10 minutes, capped at 23:59; shows if noticed within 3 minutes (0x1400583ab).
- Drank (0x140053cc5): total += ml each, this hour's amount += ml each, bottle −= ml each (not below 0).
- Refilled (0x1400583ab): bottle full, then "Time to Drink" is asked again.
- Reset (0x140053982): a second press within 3.0 s ("Sure?"); clears today's total, the shown reminders and the
  hourly amounts, and fills the bottle. Changing any water setting also clears the shown reminders (0x140053c97).
- Status ring = taken ÷ goal, capped at full (0x14007e036).

## Focus timer
- Lengths 25 / 5 / 15 minutes (settings table).
- Round counter shows min(rounds + 1, 4); the long break shows as round 4, so a long break follows 4 focus
  rounds (0x140063cab in 0x14005eaab).

## Prayer times (0x140031c46, with 0x1400863af)
- 84 cities with latitude, longitude and UTC offset from the table at 0x1400df470 (56-byte entries).
- Method: PrayTimes with Subuh 20°, Isya 18°, sunset 0.833°, Ashar shadow factor 1 (Shafi'i).
  Starting guesses 5, 12, 13, 18, 18 h; two refinement passes; Julian day by the standard formula minus
  longitude / 360.
- Minutes = ceil(time × 60 + 2 − 0.000001), i.e. a 2-minute safety margin, then the per-prayer adjustment.
- The day follows the city's own time (UTC + offset, 0x1400863af). After Isya, the next prayer is tomorrow's
  Subuh ("Next, tomorrow", 0x140085ea4).

## World clock (table at 0x1400e40d8)
Jakarta, Makassar, Jayapura, Singapore, Kuala Lumpur, Tokyo, Seoul, Beijing, Mumbai, Dubai, Mecca, Istanbul,
Cairo, London, Paris, Berlin, New York, Los Angeles, Sydney. The original uses Windows time zones; this
version uses fixed offsets with simple daylight-saving rules (estimated).

## Other values from the program's text
- Size: Small / Medium / Large / Extra large = 100 / 115 / 130 / 150 %.
- Clipboard history (free): up to 5 items.
- Prayer early reminder: 10 minutes before.
- Battery pop-ups: Low battery, Battery critical, Fully charged, Charging, Charger unplugged.

## Still estimated (looked for, not found)
- Islamic holiday dates for the countdown (2026–2030).
- Moon phase from the mean lunar month.

## Look and motion
From the code:
- Notch states (0x14007dae7 sets them, 0x140056ff1 sizes the window for each; all × the screen scale):
  resting pill 160 × 34, minimal pill 150 × 30, peek 56 × 8, HUD 310 × 60, notification 320 × 36,
  open panel 520 wide (660 with lyrics, 420 in another mode) and 140 high (+30 with an extra row); side
  margin 14 for the pill states (used here for the "ears"), 20–46 for the larger ones.
- Springs set on each state change (0x14007dae7, values at 0x1400dbb50/0x1400dabfc, 0x1400db610/0x1400dadc8,
  0x1400daa6c/0x1400db3e0): stiffness 230 / damping 19 for opening, closing and pop-ups (a small bounce);
  110 / 15.5 when a pop-up goes back to the pill; 200 / 29 into the peek state (no bounce).
- Hide in full screen (0x140051a46, setting `hide_fullscreen`, default on): hide while Windows reports a
  full-screen game or presentation, or the front window covers its monitor — except our own window, the
  desktop (Progman/WorkerW), the taskbar (Shell_TrayWnd) and maximised windows with a title bar; checked
  each second here.

From the 15 reference pictures (the drawn look):
- Pure black notch hanging from the top edge with concave ears and rounded bottom corners; drawn sizes
  resting 196 × 38, open 548 × 184, pop-up 404 × 80 (the window sizes above include shadow margins, so the
  pictures are used for drawing).
- Header: page title (second word coloured: Water **Reminder** blue, Todo **List** green), status ring,
  "Mon 9 Nov 2026" and the time in bold, level bars on the right (Shelf shows its Files/Clipboard switch
  there). Grey round arrows on both sides; page dots, the current one a white pill.
- Page order: Pomodoro, Water, Shelf, To-do, Prayer, Widgets (the original's first page, Music, is not in
  this version).

Still estimated: pages slide 120 pt sideways and fade, content fades in when the shape has nearly arrived.


## Music
- First page, as in the original's page order. Reads what Windows is playing through the same media-session
  API the original imports (Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager), every 0.5 s;
  play/pause, previous and next are sent to that session.
- Lyrics (setting `lyrics`, default on, settings table 0x1400e1320), as the original fetches them (0x14000de39):
  - title first cut at " (", " [" or " - " (cut marks at 0x1400e1d9c);
  - `https://lrclib.net/api/search?track_name=…&artist_name=…`, then `…/api/search?q=title artist` if that finds
    nothing (query pieces at 0x1400e1dd0–0x1400e1e27);
  - of the answers that have `syncedLyrics`, the one whose `duration` is nearest the song's wins
    (|duration − song| compared at 0x14000f594; with no known length the first one);
  - lines `[mm:ss.xx]` → minutes × 60 + seconds; an `[offset:ms]` tag shifts them (read at 0x14000fa88;
    its sign follows the LRC format, positive = earlier — the sign in the code was not settled).
  The lyrics button widens the panel to show them beside the player (reference picture 12); the current line is
  bold white in the middle.
- Album art is a drawn placeholder (the cover image is not loaded yet).

## Indicators and mascot
- Volume, Caps Lock and input-language indicators (settings `hud_volume`, `hud_caps`, `hud_language`, default on,
  table 0x1400e1320; labels "Volume", "Caps Lock", "Input language" at 0x1400de2e0/0x1400de306). They use the
  original's indicator state (state 4, 310 × 60 window in 0x140056ff1) and hide after 1.4 s (timer 6, 1400 ms,
  set at 0x14007df9e and 0x140035d50). Volume and mute are read from the default speakers every 150 ms; Caps Lock
  and the keyboard language of the window in front are checked about five times a second (both estimated rates).
- Network pop-up (setting `popup_wifi`, default on): a change is confirmed after 1.5 s (timer 16) and shown for 3.2 s
  (timer 4) with "Connected", "Disconnected" or "No internet access" (sub_14000311f).
- Mascot (setting `mascot`; not in the original's defaults table, on here because the original's resting pill
  shows it): two white eyes with a soft blue halo on the resting pill and a cyan-to-purple glow along the bottom
  (reference picture 15). Every 4.2 s the eyes close into a thin line and open again over 180 ms, drawn frame by
  frame only during the blink (timings estimated).

## Battery pop-ups (0x140056c76, shown by 0x14007f665)
- A charger change wins: "Charging · Charger connected", or "Charger unplugged" with the level in %.
- Otherwise, on battery: "Battery critical · Plug in now" when the level goes from 11 % or more to below 11 %;
  "Low battery · Plug in soon" when it goes from 21 % or more to below 21 %. While charging: "Fully charged ·
  You can unplug" when it reaches 100 % from below. Each is shown once, on the crossing.
- Shown for 4.5 s (timer 4, 4500 ms, in 0x14007f665). Here the level is read every 2 s (the original listens
  for Windows' power notifications).

## Prayer reminders (0x140002345)
- Two moments per prayer: at the time, and 10 minutes before (value 10.0) when "10 minutes before" is on.
- Due from 0.05 min (3 s) before until 2 min after the moment; each fires once a day (bit mask).
- The pop-up stays 10 s at prayer time (timer 4, 10000 ms, with a sound when `prayer_sound` is on) and
  6 s for the early one (6000 ms).
- When the notch is busy (focus timer, an alert), the original retries every 15 s (timer 26); here a
  reminder waits until the current pop-up is answered.

## Focus timer pop-ups (0x14000143f)
- Focus ends, rounds 1–3: "Focus done · Time for a {short break} min break"; the 4th: "Round complete ·
  Time for a {long break} min break"; a break ends: "Break over · Back to focus, {focus} min".
- No close timer: the pop-up waits, with one button ("Break" or "Focus") that starts the next part.

## Bluetooth headset, notifications and brightness (setting defaults from the table at 0x1400e1320)
- Settings: `hud_brightness` 1, `popup_bluetooth` 1, `notifications` 1, `notification_preview` 0.
- Brightness: the original registers `GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS` (bytes at 0x1400e106c)
  with RegisterPowerSettingNotification; each change shows the indicator (same 1.4 s HUD as volume).
  The value Windows sends at registration is not a change.
- Bluetooth (sub_14000b027): device ids from cfgmgr32 (`CM_Get_Device_ID_ListW` on enumerator `BTHENUM`),
  battery from DEVPKEY {104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2 (the Hands-Free nodes carry it),
  connection state and name from Windows.Devices.Bluetooth.BluetoothDevice. The pop-up text is
  "Connected · {battery}%" (string at 0x1400e28e5); without a battery it reads "Connected".
  Here only audio devices (major class AudioVideo) count, polled every 2 s; how long the pop-up stays
  is estimated (3.2 s, like Wi-Fi).
- Notifications (sub_14000d049): UserNotificationListener; access is asked once, then every 3 s
  (sleep 3 s) the toast list is read and ids not seen before are new (PostMessage 0x8002). Toasts
  already there at start are not shown. The pop-up shows the app name and the title; the text only
  with `notification_preview` on. How long it stays is estimated (4.5 s).

## Seeking, prayer pause, drag out and the Today page

- Seeking: click or drag the progress bar, or click a lyric line; sent to the media session with
  TryChangePlaybackPositionAsync (100 ns ticks). Players that don't allow seeking ignore it.
- `prayer_pause_media` ("Pause media at prayer time", off by default in the settings table at
  0x1400e1320) pauses the playing session at the prayer time itself (not the 10-minute early
  one). It does not resume afterwards (no resume found in the original).
- Drag out: the original imports DoDragDrop. Here a Shell data object (SHCreateDataObject) holds
  CF_HDROP for Shelf files or CF_UNICODETEXT for a clipboard item, and SHDoDragDrop drags it with
  the Shell's drop source. Files allow copy or link only, never move.
- `today_page` (off by default, as in the original): an eighth page after Widgets with the last
  notifications (newest first, up to 20, kept in memory only) and the week. How many it keeps is
  estimated.

## Performance: drawing only when something changes

The original (Direct2D + DirectComposition, ~30 MB, 0 % CPU idle) waits for Windows instead of
checking. This rebuild keeps egui/glow but follows the same rule:
- No frame while still. A closed notch draws again only when Windows reports a change, at the next
  minute (clock, reminders) or at the mascot's blink; an open one every 0.5 s (short-lived states),
  4 per second while a song plays on the Music page, and continuously while something moves.
- Events, on the tray thread's hidden window (as the original imports them): WM_CLIPBOARDUPDATE
  (AddClipboardFormatListener), power source and battery level (RegisterPowerSettingNotification),
  the front window changing or moving (SetWinEventHook; a move is checked after 0.2 s still), and
  Caps Lock / language keys (WH_KEYBOARD_LL, keys pass on unchanged).
- Watchers wake the notch only when their value changes. Volume comes from
  IAudioEndpointVolumeCallback; music from the session's change events; the network from
  NetworkStatusChanged; Bluetooth from ConnectionStatusChanged (plus a look every 60 s for new
  pairings and battery). Notifications are read every 3 s, as the original does (an unpackaged app
  can't subscribe to them).
- While hidden for full screen it looks again every second, so it returns even if an event is missed.
  "Hidden" means drawing nothing with clicks passing through, not hiding the window: eframe never
  calls update for a hidden window (so it could not come back) and its event loop then spins a core.
- Tray icon: a left click opens the notch, a right click gives the Settings / Quit menu.
- Release build: LTO, one codegen unit, stripped (exe 15 MB → 4.6 MB).
Measured idle (Task Manager figures): CPU 0.03 % of the machine, private working set ~36 MB; most
of the larger "working set" is the graphics driver's shared DLLs.
Measured again on 2026-11-24 (release build, after 6.5 h running, one PC): shrunk 0.08 % CPU and
22 MB private working set (45 MB working set); open 0.4 % CPU, up to 63 MB private (87 MB working
set), back to ~32 MB within 20 s of closing. Cover art and app logos account for the growth.

## Languages (not in the original)
- English, Tiếng Việt, 中文 (简体), Français; setting `language`, first set from Windows' display
  language (GetUserDefaultUILanguage), changed from Settings → General → Language at once.
- `i18n.rs`: the code keeps the English phrase and `t` looks up the chosen language's; a phrase
  missing from the table stays English. Phrases with numbers use `{}` gaps filled by `tr` in order.
  Dates and weekday heads come from its own name tables. City, prayer and song names are not translated.
- Chinese needs Microsoft YaHei (msyh.ttc, part of Windows); it is loaded only while Chinese is chosen.
- `--lang N` with `--screenshot` checks a language's look.

## Nusa 1.3.1 features, part A (offline)
- Date and time switch (`show_clock`): hides the clock in the pill and on each page; a running
  Pomodoro countdown still shows.
- Pages (`page_order`, `pages_hidden`): Settings › Pages reorders with ↑ ↓ and hides with the
  switch; one page always stays. Dots, arrows and the slide follow the shown order; Today keeps its
  own `today_page` switch. A saved order that isn't every page once falls back to the usual order.
- Calmer notifications: a 380 × 44 pill (the original's 320 × 36 state) with the app's initial, who
  sent it and, with preview on, what they said. A click opens the sending app through
  `explorer.exe shell:AppsFolder\<AUMID>`; if that fails the notch opens instead.
- Ask Nusa box: Ctrl + Alt + Space (RegisterHotKey, like the original). A sum ending in "=" answers
  as you type (`calc.rs`: + − × ÷ ^ %, brackets, decimal comma); questions need Gemini (part C).
- Prayer times worldwide (`prayer.rs`): the 84 Indonesian cities as before (Kemenag, fixed zones),
  plus world cities in their own clock with summer time (shared `summer()` rules), plus "My location"
  from Windows' location service (15 s limit) in this PC's clock. Methods: Kemenag, MWL, ISNA,
  Egypt, Umm al-Qura (Isya +90 min), Karachi (Hanafi Ashar), JAKIM, MUIS, Diyanet, UOIF, Gulf;
  "Usual for the city" by default. Far from the equator Subuh/Isya follow the angle-based night rule.
  No reminders while "My location" has no fix yet.
- Smart OCR: Ctrl + Alt + T freezes the screen (GDI), drag an area (Esc or right-click cancels),
  Windows.Media.Ocr reads it offline in the user's languages (large areas shrunk to its limit), and
  the text is copied. One read at a time. Nothing is written to disk.
- Tray menu as the original: Start with Windows, Mascot, Minimal notch (ticked), About, Exit; a left
  click opens the notch.

## Nusa 1.3.1 features, part B (network, free, no key)
- Weather widget (widget 8, `weather_city` from the prayer City list): MET Norway's locationforecast
  compact, fetched only while a Weather widget is chosen, every 30 minutes or at once when the place
  changes (a failure is retried after a minute). Coordinates are rounded to 2 decimals (~1 km).
  Shows the date, temperature, the next hour's sky and a soft light in the sky's colour.
  MET requires an identifying User-Agent; it is added unvalidated because WinRT's typed parser
  turns some valid names down (MET then answers 403).
- Currency in the Ask box: "100usd to vnd=" or "1,5 eur in jpy=" (Enter adds the "="). Rates
  against USD from open.er-api.com, fetched on first use and kept 12 hours; offline is said so.
- Every thread that uses WinRT (weather, rates, location, OCR) calls RoInitialize first.
- `--weather SYMBOL` fills a screenshot's Weather widget without the network.

## Nusa 1.3.1 features, part C (the user's own key and calendar)
- Secrets (the Gemini key in `gemini.key`, the calendar address in `calendar.key`) are sealed with
  Windows DPAPI (CryptProtectData, this account on this PC) and never written to settings.txt.
  Screenshots never read them.
- Ask Nusa: Enter on anything that isn't a sum or a currency sends it to Gemini
  (`gemini-flash-latest`, generateContent) with the key in the `x-goog-api-key` header. Asked for at
  most three short sentences, plain text, in the question's language. The answer wraps, scrolls and
  can be selected; an answer to an older question is dropped. 400/401/403 = key, 429 = busy.
- Translate: Tab in the Ask box (the box keeps focus) translates into the app's language, or into
  English when the text is already in it.
- Google Calendar: the secret iCal address is read every 15 minutes (a minute after a failure).
  RRULE DAILY/WEEKLY/MONTHLY/YEARLY with INTERVAL, UNTIL, COUNT, BYDAY (incl. 2TU, -1FR),
  BYMONTHDAY; EXDATE; RECURRENCE-ID copies replace their day; cancelled events are left out.
  UTC times become local; TZID times are read as the PC's zone (shortcut). Today's events show
  first in To-do (read only) and remind at their start, following "When a task starts".
- `--ask --demo` and `--page 4 --demo` fill screenshots with a made-up answer and events.
- Live checks: `cargo test live -- --ignored` (MET, rates, Google holidays iCal, Gemini bad key).

## Getting around (after part C)
- Page turns: the wheel or a touchpad swipe (down or left = next) and the ← → keys while nothing is
  being typed. Read unsmoothed; 30 points (under one notch, which egui counts as 40) turn one page,
  and the rest of that flick is ignored for 0.35 s. Over the To-do list only sideways scroll turns.
- Shortcuts: RegisterHotKey's result is kept (tray::hotkeys_taken); a taken Ctrl+Alt+Space or
  Ctrl+Alt+T is said once in a 12 s pop-up, and Settings › General shows Ready or which is taken.
- First start: `welcomed` in settings.txt; while false, a 12 s pop-up says how to open the notch
  (hover or click, following "Open the notch on") and that right-click opens Settings.
- `--popup welcome` and `--popup taken` show those pop-ups in a screenshot.

## Notifications, reworked
- Icon: the app's own logo (AppInfo.DisplayInfo.GetLogo, 48 px, transparent margins trimmed, read
  once per app and kept in memory), on a dark plate; without one, the initial in a dot whose colour
  comes from the app name (FNV-1a over 8 colours, the same every run).
- With the message text: two lines (420 x 60): who sent it with the app's name dim on the right,
  then the message across the full width. Without it: one line (380 x 44). Text is cut by real
  width with "…".
- Shows 4.5 s, plus 1 s per 30 characters past 40, up to 8 s; never closes while the pointer is on
  it, and closes 1 s after it leaves.
- Several in a row: the newest shows with "+N" for those it covers (all are on the Today page).
- Arrival: the icon pops from half size with a small overshoot (0.45 s) and a cyan-to-violet light
  runs along the bottom once (0.9 s).
- Screenshots: `--popup notice`, `notice-long`, `notice-logo` (Windows Settings' real logo, +2).

## Music, reworked
- Cover: the session's Thumbnail, cut to a centred square, 156 px, read once per song. A song
  without one is read again on its next few changes (up to 4), as Spotify sends it a moment late.
  The drawn placeholder stays when the player gives none.
- Text is cut by real width (to the lyrics column, or short of the page arrow).
- Player: SourceAppUserModelId; its name from AppInfo, else the readable part of the id; its logo
  as for notifications, on the cover's corner (else its initial in its own colour).
- Colour: the cover's colourful pixels averaged (weighted by saturation squared) and lifted to 230;
  used for the progress bar and the level bars while playing. A grey cover keeps white.
- Collapsed while playing (no session counting down): small cover, clock, bars in the cover's
  colour; it takes the mascot's place while the music plays.
- Several players: the icon (ringed) switches to the next in Windows' list. The pick holds until
  that player is gone, or is paused while Windows' own pick plays something else.
- `--page 0 --demo` fills a screenshot with a made-up song and cover (no one's real music).

## Pomodoro, reworked
- `next_part(skipped)` moves on: a break after focus (long every 4th round), focus after a break.
  Run out: stops and the pop-up (Kind::Info) waits for its button. With `auto_next`: goes on at
  once and says so in a pop-up that closes itself. Skipped (▶|): goes on as the skipped part did,
  running or not, with no pop-up; a skipped focus still counts as a round.
- ↺ starts the part over; on a part not started yet (and past round 1 or on a break) it starts the
  4 rounds over. A new day does the same, unless a part is under way (`rounds_day`).
- `focus_quiet`: while a focus part runs, new notifications wait (the newest kept, with a count);
  the newest shows with "+N" once focus ends and nothing else is up.
- Space starts / pauses on the Pomodoro page when nothing is being typed.
- A gap of more than 5 s between looks while running means the PC slept: Pomodoro pauses, the gap
  counts for nothing, and a pop-up says so.

## Water, reworked
- Drink adds the plan's share (goal / reminders); "+250 ml" adds a glass (GLASS_ML). Each drink is
  remembered (ml, hour, ml from the bottle) and the status line offers Undo for 10 s.
- Catch-up: a slot not shown at its time (another pop-up up, the PC asleep, a full-screen game)
  shows later as long as the next slot isn't due yet, so at most one catch-up. A settings change or
  Reset counts the slots already past as shown, so neither sets off a reminder at once.
- History: water_log.txt, one "YYYY-MM-DD ml" line a day, the last 60 days, today's as it stands;
  water.txt from an earlier day adds that day's total at start. The Water widget shows 7 days
  against the goal.
- Snooze waits water_snooze minutes (5, 10 or 15; Settings › Water reminders).
- Goal: 1 to 4 L in 0.25 L steps (goal_step rounds a saved goal to the nearest step).
- `--page 2 --demo` shows the Water page just after a 250 ml drink, with its Undo.

## Shelf, clipboard, To-do and calendar, reworked
- Clipboard text marked "don't keep" is never recorded: "ExcludeClipboardContentFromMonitorProcessing",
  "Clipboard Viewer Ignore", or "CanIncludeInClipboardHistory" of 0 (as Windows' own clipboard history
  reads them; tray::clipboard_private). A mark that can't be read counts as private.
- Files and clipboard cards scroll sideways when they don't fit (the wheel scrolls them; no page turns
  over them). Files are 82 wide, so an edge of the sixth shows. A file gone from its place is drawn
  faded with "Not there any more" and only removing works. Clipboard: right-click removes one, Clear
  empties the history (and clipboard.txt).
- To-do: "done" counts for the day it was done; tasks come back each day. Reminders shown are kept in
  tasks.txt per day ("done;fired=DAY:se|start|dur|title"), so a restart doesn't repeat them and the
  next day reminds again; old lines without ";fired=" still read.
- Calendar: also 10 minutes before (cal_early, on by default; Settings › Google Calendar). Timed
  events from yesterday still running after midnight show today as "until HH:MM" (no reminder).
- `--page 3 --clips` opens a screenshot on the clipboard side.

## Prayer, reworked
- "My location" without a fix: the page says so with a Find button and the Next prayer widget says
  so too (no times for 0°, 0°); reminders were already held back (`prayer_placed`).
- Names: prayer::NAMES are the English keys (Fajr, Dhuhr, Asr, Maghrib, Isha), translated by i18n.
- Catch-up: a missed reminder still shows later; the early one until prayer time, the prayer itself
  for 30 minutes. "Pause media" acts only in the original 2-minute window.
- Sunrise (Terbit) comes from the same pass as the five times, rounded down with no safety minutes;
  it is a hollow sun on the arc. Imsak = Fajr - 10 min. Arc labels are spread at least 42 points apart.
- Asr school (prayer_asr): as the method does, Shafi'i (shadow x1) or Hanafi (x2), for any method.
- Play again (prayer_resume_media): music the app paused at prayer time (only if it was playing) plays
  10 minutes later, unless someone played it meanwhile.
- Hijri date: the tabular (30-year cycle) calendar, worked out on the PC; it can be a day from a
  sighted one (Umm al-Qura, Kemenag's rukyat).

## Widgets, reworked
- World clock: 27 cities (Hà Nội, TP. Hồ Chí Minh, Bangkok, Manila, Hong Kong, Taipei, Moscow and
  Toronto added at the end, so saved choices keep their city), each with its distance from this PC's
  clock, rounded to the quarter hour.
- Countdown: Ramadan and the two Eids use the announced dates to 2030, then the tabular Hijri
  calendar (within a day of every announced date, by test). Added Lunar New Year (Tết, a table to
  2040), Vietnam's National Day (2/9) and "My own date" (widget_my_name, widget_my_date: "DD/MM"
  every year, "DD/MM/YYYY" once; 29/02 waits for a leap year). "Independence Day" is named as
  Indonesia's.
- Clicking a widget opens its page when it has one and the page is shown: This week → To-do,
  Next prayer → Prayer, Focus today → Pomodoro, Water → Water.
- Weather: today's high and low over the forecast steps on this PC's day, the next 6 hours'
  precipitation_amount (shown from 0.1 mm), and °F as an option (fahrenheit).

## Today, reworked
- Recent notifications (up to RECENT_MAX = 20, in memory only) scroll; one row per app (group_by_app:
  same name and id, newest first, its newest notification shown, "App · n"), with the app's logo or
  its initial in its own colour. Text is cut by real width.
- A row's click opens the sending app (open_app: shell:AppsFolder\<AUMID>, only for an id with no
  quote or control character); right-click removes that app's notifications from the list.
- This week counts today's calendar events beside the open tasks, and "Next" is the earliest timed
  task or event still to come.
- `--page 7 --demo` fills a screenshot with made-up notifications and events.

## System pop-ups, reworked
- Battery news (charger in/out, low, critical, full) is kept in battery_news until it can show: it
  takes the place of a pop-up that needs no answer (a bar, a notification, network, Bluetooth,
  "Now playing", About, a hint) and waits behind one that does (Drink, a task, a prayer, Pomodoro).
- Low battery: at or under battery_low (15, 20 or 30 %; 20 matches the original's "below 21 %").
- "Now playing" (popup_song): a new song while the notch is closed shows its cover, name, artist and
  bars in the cover's colour for 3 s; not in the app's first 10 s, so the song already playing at
  start isn't news.
- Bluetooth (popup_headset_off): a headset that goes away says "Disconnected" by the name it had;
  one at or under 20 % battery says so once, until it is back above 30 %.
- `--popup song --demo` shows "Now playing" in a screenshot.

## Ask box, reworked
- Numbers (calc::number): both marks → the last one is the decimal (1.000,5 / 1,000.5); one mark more
  than once → thousands (1.000.000); one mark once with exactly three digits after, not after a lone
  0 → thousands (1,000); otherwise the decimal (1,5 / 0.125 / 3.14). Shorthand after a number: k,
  tr / triệu (millions), tỷ / tỉ (billions).
- Gemini replies (online::gemini_answer): 200 → the text (or "no answer"), 429 → too many questions,
  400/401/403 → the key, anything else → offline; tested without the network.
- Copy: sums, conversions and Gemini's answers (not "Thinking…" or errors) get a Copy button.
- ↑ / ↓: the last 20 questions, in memory only.

## Settings, reworked
- Start with Windows reads what Windows will really do at start (`reg query` of the Run value and of
  Explorer\StartupApproved\Run, where Task Manager's switch keeps a first byte of 03 for "off"); an
  entry pointing at an exe that no longer exists points at this one again. Turning it on clears a
  Task Manager "off"; a change Windows refuses is undone in Settings and said in a pop-up.
- Data card: Open (explorer on %LOCALAPPDATA%\MyNotch) and Restore (two clicks within 3 s): the
  defaults, keeping the language, the water plan, Start with Windows, the location found and the
  welcome flag; tasks, history, keys and the calendar address are other files and stay.
- Esc closes Settings.
- Layout (2026-11-25): the 11 sections are listed down the left, each with a coloured icon tile
  (General, Pomodoro, Water, To-do, Prayer, Shelf, Popups, Music, Ask Nusa, Widgets, Data); the
  chosen one shows on the right under its heading, rows split by thin lines. General holds Pages,
  To-do holds Google Calendar, Popups holds Notifications. The panel is 660 x 450.

## Note page (2026-11-25, not in the original)

- Page id 8 (ids are saved in `page_order`, so the new page takes the next id) shown after To-do.
  A `page_order` saved before it: the untouched usual order becomes the new usual order; a
  rearranged one keeps its order with Note at the end; hidden pages stay hidden.
- `notes.txt`, one note per line: `created|flags|text`, flags `p` (pinned) and `d2026-11-25` (done
  that day), text escaped like the clipboard history (`\n`, `\\`). Plain text, by choice.
- After every change (and at start): notes are cut to 500 characters, ones done longer ago than
  Settings › Note says (1, 3 or 7 days, or never; 3 by default) are dropped, at most 200 are kept
  (oldest done first, then the oldest unpinned open one), sorted pinned, open, done, newest first.
  A line of `notes.txt` that doesn't read sends the whole file to `notes.bak` before the next save.
- Each note is a card: done circle, a push pin when pinned, the text on one line, and when it was
  written on the right (Settings › Note can hide it). "Copied" marks the clicked note by its place,
  since texts can repeat. "Select" ticks notes; "Delete (N)" needs a second click within 3 s.
- Enter in the box saves (and keeps the cursor there); the clipboard button adds the clipboard's
  text unless it is marked private (`tray::clipboard_private`). Click copies ("Copied" for 2 s),
  drag goes out through `dragout::text`, the circle toggles done, right-click pins / finishes / deletes.
- Ctrl+Alt+N (`RegisterHotKey`, like Ask and OCR) opens the notch on Note with the cursor in the
  box, un-hiding the page if needed; when another app owns it, the "shortcut used" hint lists it.

## Code layout (2026-11-25)

`src/main.rs` starts the app and holds its state, the clock-driven reminders (`tick`) and the
update loop. The rest sits in folders by role; `#[path]` on each `mod` keeps the module names
(`crate::tray`, `crate::media`...) independent of the folders.

| Folder | Files | What |
|---|---|---|
| `ui/` | `draw.rs`, `controls.rs`, `popup.rs`, `demo.rs` | shapes and icons; Settings rows; the resting notch and pop-ups; screenshot demo data |
| `ui/pages/` | `mod.rs` + one file per page | Music, Pomodoro, Water, Shelf, To-do, Prayer, Widgets, Today, Ask, Settings |
| `platform/` | `tray.rs`, `system.rs`, `media.rs`, `dragout.rs`, `ocr.rs` | Windows: tray and hotkeys, full screen / battery / startup, media and notifications, drag-out, OCR |
| `net/` | `online.rs`, `calendar.rs` | weather, rates and Gemini; the iCal calendar |
| `logic/` | `data.rs`, `calc.rs`, `prayer.rs`, `i18n.rs` | settings and saved files; the calculator; prayer times; translations |

The Prayer page's module is `prayer_page` (file `ui/pages/prayer.rs`) so it doesn't hide
`crate::prayer`. The i18n test reads every `.rs` under `src/`, in any folder.
