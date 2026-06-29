#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/../src-tauri"
cargo tauri build --bundles appimage
