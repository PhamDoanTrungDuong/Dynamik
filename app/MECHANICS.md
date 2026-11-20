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
- Mascot (setting `mascot`; not in the original's defaults table, so off here): two white eyes on the resting pill
  with a cyan-to-purple glow along the bottom (reference picture 15); blinks every 4.2 s (estimated).

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
- Release build: LTO, one codegen unit, stripped (exe 15 MB → 4.6 MB).
Measured idle (Task Manager figures): CPU 0.03 % of the machine, private working set ~36 MB; most
of the larger "working set" is the graphics driver's shared DLLs.
