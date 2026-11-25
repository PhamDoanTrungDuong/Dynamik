# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Dynamik / My Notch**: a Dynamic Island–style notch for Windows 10/11, written in Rust with
eframe/egui 0.31 plus the `windows` / `windows-sys` crates. It is an independent rebuild of the
Nusa app: comments like `(0x14004aa9e)` point at addresses in the original `nusa.exe`, whose
behaviour the code follows. The original binary and reverse-engineering material (`reference/`,
`capture/`, `checks/`, `investigation.cany/`) are gitignored. `app/MECHANICS.md` records how every
feature works and why; update it when behaviour changes.

## Commands (run in `app/`)

```sh
cargo build --release                         # app/target/release/notch.exe
cargo test --release                          # all tests
cargo test --release notes_tidy               # one test (name or module path, e.g. data::)
cargo test --release live -- --ignored        # tests that hit real services / devices
cargo clippy --release
```

- Tests write to `%LOCALAPPDATA%\MyNotch`. Point `LOCALAPPDATA` at a scratch folder first so the
  user's real settings, notes and tasks are never touched.
- A running notch locks `app/target/release/notch.exe`, so `cargo build --release` fails with
  "Access is denied". Build into another folder with `CARGO_TARGET_DIR` instead of killing it.
- The exe icon comes from `build.rs` via `windres` when it is on PATH; without it the build still works.

### Screenshots without touching the screen

`notch.exe --screenshot <file.png>` renders one frame and exits. Add `--page N` (page id),
`--demo` (sample data for Music, Water, To-do, Today, Note, Ask), `--resting`, `--settings [section]`,
`--ask`, `--clips`, `--select` (with `--page 8 --demo`: choosing notes), `--lang N` (0 EN, 1 VI, 2 ZH, 3 FR), `--weather SYMBOL`, or
`--popup welcome|taken|notice|notice-long|notice-logo|song|bluetooth|brightness`. Compare pages
before and after a change this way; only the clock text should differ between runs.

### Release package

Copy the built exe as `dist/MyNotch/MyNotch.exe` next to the three files in `packaging/`
(`install.cmd`, `uninstall.cmd`, `HUONG-DAN.txt`), zip the `MyNotch` folder as
`dist/MyNotch-<version>-windows-x64.zip`, and attach it to a `v<version>` GitHub release. The
version lives in `app/Cargo.toml`. `dist/` is never committed.

## Architecture

- **`app/src/main.rs`** holds `App` (all state), `App::new`, the clock-driven `tick()` (Pomodoro,
  water, tasks, calendar and prayer reminders, Windows events, pop-up timeouts) and
  `eframe::App::update`. Everything else lives in folders by role: `ui/` (drawing, Settings controls,
  pop-ups, demo data, `ui/pages/` with one file per page), `platform/` (tray, hotkeys, Windows calls,
  media/notifications/Bluetooth watchers, drag-out, OCR), `net/` (weather, rates, Gemini, iCal),
  `logic/` (settings and data files, calculator, prayer times, translations).
- **Module wiring:** each `mod` in `main.rs` has a `#[path = "folder/file.rs"]`, so module names
  stay flat (`crate::tray`, `crate::media`, ...). The UI modules start with `use super::*;` and
  `main.rs` glob-imports them back (`use draw::*;`), so their items are `pub(crate)`. Page files are
  `impl App` blocks; the Prayer page's module is `prayer_page` so it does not hide `crate::prayer`.
- **Pages:** ids are fixed because they are saved in `settings.txt` (`page_order`, `pages_hidden`).
  `PAGES` in `main.rs`, `PAGE_COUNT` / `DEFAULT_ORDER` / `page_order()` in `logic/data.rs`, and the
  routing in `ui/pages/mod.rs` must agree. A new page takes the next id, and `page_order()` must
  migrate orders saved before it existed.
- **Redraw only on change:** the notch draws nothing while still. Background threads (media, network,
  Bluetooth, weather, rates, calendar) update `Arc<Mutex<..>>` state and call `tray::wake()`; the tray
  thread's hidden window turns Windows messages into `EV_*` flags read by `tray::take_events()`, and
  hotkey / tray-menu picks into `tray::take()`. New background work should follow this pattern
  rather than poll every frame.
- **Settings:** `logic/data.rs` lists every simple setting once in the `simple_settings!` table
  (key = field name, default, load-time check). A new setting needs the struct field plus one table
  line; the golden test `settings_text_is_unchanged` pins the saved text. Water settings live in
  `water.txt`; the Gemini key and the iCal address are DPAPI-sealed in `gemini.key` / `calendar.key`.
  Data files are written write-then-rename.
- **Translations:** UI text is the English phrase passed to `t("...")` / `tr("...", &[..])`; the
  `TABLE` in `logic/i18n.rs` gives Vietnamese, Chinese and French. The test
  `every_phrase_the_app_asks_for_is_in_the_table` scans every `.rs` under `src/`, so a new phrase
  without its row fails the tests. Settings labels written `"Name — hint"` are split on ` — ` and
  every translation must keep that separator.

## Repo conventions

- `README.md` is English; `README.vi.md`, `README.zh-CN.md` and `README.fr.md` mirror it with a
  language bar at the top. Change all four together. Page screenshots live in `app/shots/`.
- Commit messages in English, no emoji ending, no AI attribution. Commit and push to `main` after
  each finished part, once `cargo test --release` passes.
- Design specs and plans go in `docs/superpowers/specs/` and `docs/superpowers/plans/` and are not
  committed.
