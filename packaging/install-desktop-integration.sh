#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data_home="${XDG_DATA_HOME:-${HOME}/.local/share}"
applications_dir="${data_home}/applications"
icons_dir="${data_home}/icons/hicolor"
desktop_id="com.ardali.gaming.desktop"

install -Dm644 \
  "${project_dir}/packaging/linux/ardali-gaming.desktop" \
  "${applications_dir}/${desktop_id}"

install -Dm644 \
  "${project_dir}/src-tauri/icons/32x32.png" \
  "${icons_dir}/32x32/apps/ardali-gaming.png"
install -Dm644 \
  "${project_dir}/src-tauri/icons/64x64.png" \
  "${icons_dir}/64x64/apps/ardali-gaming.png"
install -Dm644 \
  "${project_dir}/src-tauri/icons/128x128.png" \
  "${icons_dir}/128x128/apps/ardali-gaming.png"
install -Dm644 \
  "${project_dir}/src-tauri/icons/128x128@2x.png" \
  "${icons_dir}/256x256/apps/ardali-gaming.png"
install -Dm644 \
  "${project_dir}/src-tauri/icons/icon.png" \
  "${icons_dir}/512x512/apps/ardali-gaming.png"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${applications_dir}" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${icons_dir}" >/dev/null 2>&1 || true
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

echo "ArDali Gaming masaüstü kimliği ve simgeleri hazırlandı (${desktop_id})."
