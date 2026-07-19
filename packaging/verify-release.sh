#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

required_files=(
  Cargo.toml
  src-tauri/tauri.conf.json
  src-tauri/capabilities/default.json
  frontend/index.html
  frontend/app.js
  frontend/styles.css
  packaging/linux/ardali-gaming.desktop
)

for file in "${required_files[@]}"; do
  test -s "$file" || { echo "EKSİK: $file" >&2; exit 1; }
done

command -v cargo >/dev/null || { echo "EKSİK: cargo" >&2; exit 1; }
command -v node >/dev/null || { echo "EKSİK: node" >&2; exit 1; }

cargo fmt --all -- --check
cargo test --workspace
cargo check --workspace
node --check frontend/app.js
node --check frontend/game-settings.js

echo "İsteğe bağlı çalışma zamanı bileşenleri:"
for tool in steam gamemoderun gamemoded gamescope secret-tool; do
  if command -v "$tool" >/dev/null; then
    echo "  HAZIR  $tool -> $(command -v "$tool")"
  else
    echo "  EKSİK  $tool"
  fi
done

if [[ "${1:-}" == "--build" ]]; then
  # Updater artifacts are produced only by the signed release job.
  (cd src-tauri && cargo tauri build --bundles deb \
    --config '{"bundle":{"createUpdaterArtifacts":false}}')
fi

echo "ArDali Gaming release doğrulaması başarılı."
