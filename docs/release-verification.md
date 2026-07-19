# Release and clean-system verification

Run `packaging/verify-release.sh` before producing a package. Pass `--build`
to also create an unsigned local `.deb` smoke package. This path disables
updater artifacts, so a developer machine does not need the release key.

The signed release job must run the configured full bundle build separately
with `TAURI_SIGNING_PRIVATE_KEY` (and its password, when used) available. Never
store the private key in the repository.

## Clean Linux system checklist

1. Install the packaged ArDali build without copying development data.
2. Start it with no Steam, GameMode, Gamescope, or Secret Service installed.
   The app must remain usable and show each missing optional component.
3. Install Steam and rescan. Native, Flatpak, and Snap layouts are supported.
4. Enable automatic Steam synchronization, restart ArDali, and confirm installed
   games are imported while missing games are disabled rather than deleted.
5. Install GameMode and run the in-app `GameMode Testi`. Enable it for one game
   and confirm the card changes from `GameMode hazır` to `GameMode aktif`.
6. Close Steam, select a Proton version in game settings, and verify that a
   timestamped `config.vdf.ardali-backup-*` file was created. Restore by copying
   that backup over `config.vdf` while Steam is closed.
7. Save a SteamGridDB key. With Secret Service available it must not remain in
   SQLite; without Secret Service the UI must explain/use the local fallback.
8. Launch a Steam game, restart ArDali while it runs, then close the game. The
   active session and total play time must recover and finish once.
9. Test keyboard and gamepad navigation: directions move focus, A launches,
   B returns to the library, and Menu opens game settings.

## Package dependencies

- Required: a Linux desktop session and the libraries bundled/declared by Tauri.
- Optional: Steam, `gamemode`, Gamescope, and `libsecret`/`secret-tool`.
- SteamGridDB is optional. Local Steam manifests and covers work without a key.
- Wine/Proton runtime downloads remain managed separately by ArDali.

Database migrations are forward-only and idempotent. Back up the application
SQLite database before downgrading; older binaries cannot understand columns
created by newer releases.
