# ArDali Gaming

ArDali Gaming is a Linux-first Tauri application planned as a game launcher for portable Wine, Proton, OpenRA, DOSBox-X, Steam integration, and per-game compatibility management.

## Stack

- Tauri 2
- Rust backend
- HTML/CSS/JS frontend
- Linux package targets: AppImage, deb, rpm

## Development

Install Tauri's Linux system dependencies first. On Arch Linux, the missing package detected during setup was:

```sh
sudo pacman -S --needed webkit2gtk-4.1
```

Then run:

```sh
cd src-tauri
cargo check
cargo tauri dev
```

In the app, use `Portable Runtime > Wine Kur` to install Wine as an isolated portable runtime under `~/.local/share/ardali-gaming/wine/current`. This does not install or change the system Wine package.

## Packaging

AppImage build:

```sh
./packaging/build-appimage.sh
```

Arch/AUR packaging files are under `packaging/aur/`, and pacman repository notes are in `packaging/pacman-repo.md`.

The release binary was verified at `src-tauri/target/release/ardali-gaming`. AppImage bundling can require a writable AppImage/tooling environment; if bundling fails with a read-only filesystem error, retry from a normal writable shell/session.

## Runtime Layout

ArDali Gaming keeps portable runtimes, game data, prefixes, metadata, updates, and logs isolated under:

```text
~/.local/share/ardali-gaming/
```

Important subdirectories:

- `wine/`: portable Wine, Proton, DXVK, and VKD3D components.
- `emulators/`: portable OpenRA and DOSBox-X components.
- `prefixes/`: one isolated Wine prefix per Windows game.
- `games/`: default per-game install directories.
- `compatibility/`: per-game Wine/DLL/troubleshooting files.
- `covers/`: downloaded or manually referenced cover metadata.
- `updates/`: portable component update state.
- `logs/`: backend logs.

Current backend commands:

- `initialize_runtime`: creates app data directories and reports runtime status.
- `download_portable_runner`: downloads a Wine, Proton, DXVK, or VKD3D archive into isolated app data.
- `initialize_emulators`: creates portable OpenRA and DOSBox-X directories and writes starter config/catalog files.
- `download_portable_emulator`: downloads OpenRA AppImage or DOSBox-X archive into isolated app data.
- `select_game_runner`: routes Windows, Steam, OpenRA, and DOS game types to the matching runner.
- `create_wine_prefix`: creates one Wine prefix per game under `prefixes/`.
- `install_dxvk_vkd3d`: queues DXVK and VKD3D setup markers for a prefix.
- `add_game_installation`: creates a game install directory, selects the runner, creates a Wine prefix when needed, and inserts the game into SQLite.
- `list_games`: reads the SQLite game library for the launcher UI.
- `scan_steam`: scans local Steam roots, library folders, installed game manifests, and Proton directories.
- `sync_steam_library`: imports detected Steam games into SQLite using the preferred Steam Proton path when available.
- `launch_game`: starts the selected game runner and updates the last played timestamp.
- `remove_game`: removes a launcher library record, optionally with local files.
- `game_settings`: returns the selected game record for the settings dialog.
- `update_game_mode`: stores per-game display mode and FPS overlay preferences.
- `save_compatibility_settings`: stores Wine, Windows, DLL override, and launch environment compatibility settings.
- `compatibility_report`: returns troubleshooting file paths and recent compatibility logs.
- `append_compatibility_error`: appends a troubleshooting note to a game's compatibility log.
- `fetch_protondb_summary`: fetches a ProtonDB summary JSON for a Steam AppID when available.
- `list_settings`: reads launcher configuration from SQLite.
- `set_setting`: writes launcher configuration to SQLite.
- `fetch_game_metadata`: searches SteamGridDB and downloads a cover image into app data.
- `set_manual_cover`: stores a manually selected cover path for a game.
- `check_component_updates`: reports installed/update state for Wine, Proton, DXVK, VKD3D, OpenRA, and DOSBox-X.
- `update_component`: downloads and installs a portable component archive from a provided URL.

Game launch emits `game-ended` when the child process exits. The launcher stores `display_mode`, `fps_overlay`, and `last_played_at` in SQLite.
Play sessions are tracked in SQLite and rolled into `total_playtime_seconds` when the child process exits.

Compatibility files are stored under `~/.local/share/ardali-gaming/compatibility/<game-id>/`.
Cover images downloaded from SteamGridDB are stored under `~/.local/share/ardali-gaming/covers/`.
Component update state is stored under `~/.local/share/ardali-gaming/updates/`.

Tauri updater is enabled with placeholder endpoint configuration in `src-tauri/tauri.conf.json`; set a real updater endpoint and signing public key before release builds.

Backend log events are emitted to the UI as `backend-log` and persisted to `logs/backend.log`.

SQLite data is stored at `~/.local/share/ardali-gaming/games.db`.

See `RELEASE.md` for the metadata, update, AppImage, AUR, and pacman repository release checklist.
