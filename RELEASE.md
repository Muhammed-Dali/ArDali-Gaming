# ArDali Gaming Release Checklist

## Metadata

- SteamGridDB integration is implemented through `fetch_game_metadata`.
- Manual cover support is implemented through `set_manual_cover`.
- Store the SteamGridDB API key from the app settings panel or with `set_setting`.
- Downloaded covers are stored under `~/.local/share/ardali-gaming/covers/`.

## Component Updates

- Portable component status is exposed by `check_component_updates`.
- Portable component updates are handled by `update_component`.
- Supported components:
  - Wine
  - Proton
  - DXVK
  - VKD3D
  - OpenRA
  - DOSBox-X
- Component update state is stored under `~/.local/share/ardali-gaming/updates/`.

## App Updates

- Tauri updater plugin is installed and enabled.
- `createUpdaterArtifacts` is enabled in `src-tauri/tauri.conf.json`.
- Before publishing, replace the placeholder updater endpoint.
- Before publishing, generate and configure the Tauri updater signing public key.

## Packaging

- AppImage target is configured in Tauri.
- AUR package scaffold is in `packaging/aur/PKGBUILD`.
- Desktop entry is in `packaging/linux/ardali-gaming.desktop`.
- Pacman repository notes are in `packaging/pacman-repo.md`.

## Verification

Run before release:

```sh
cd src-tauri
cargo fmt --check
cargo check
cargo test
cargo tauri build --bundles appimage
```

If AppImage bundling fails with a read-only filesystem error, retry from a normal writable shell/session. The release binary can still be produced at `src-tauri/target/release/ardali-gaming`.
