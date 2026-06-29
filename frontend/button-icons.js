const svgNamespace = "http://www.w3.org/2000/svg";

const iconPaths = {
  add: ["M12 5v14", "M5 12h14"],
  archive: ["M3 7h18", "M5 7l1 12h12l1-12", "M9 11h6"],
  check: ["M20 6 9 17l-5-5"],
  close: ["M18 6 6 18", "M6 6l12 12"],
  database: ["M4 6c0-2 4-3 8-3s8 1 8 3-4 3-8 3-8-1-8-3Z", "M4 6v6c0 2 4 3 8 3s8-1 8-3V6", "M4 12v6c0 2 4 3 8 3s8-1 8-3v-6"],
  download: ["M12 3v12", "M7 10l5 5 5-5", "M5 21h14"],
  file: ["M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z", "M14 2v6h6"],
  folder: ["M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"],
  gamepad: ["M6 11h4", "M8 9v4", "M15 12h.01", "M18 10h.01", "M17 5H7a5 5 0 0 0-4 8l1.5 2.5a3 3 0 0 0 5-1L10 14h4l.5.5a3 3 0 0 0 5 1L21 13a5 5 0 0 0-4-8Z"],
  image: ["M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Z", "M8 11a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z", "M21 15l-5-5L5 21"],
  layout: ["M4 5h16", "M4 12h16", "M4 19h16"],
  library: ["M4 19.5A2.5 2.5 0 0 1 6.5 17H20", "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z"],
  loader: ["M21 12a9 9 0 0 1-9 9"],
  play: ["M8 5v14l11-7Z"],
  refresh: ["M21 12a9 9 0 0 1-15.4 6.4L3 16", "M3 21v-5h5", "M3 12a9 9 0 0 1 15.4-6.4L21 8", "M21 3v5h-5"],
  reset: ["M3 12a9 9 0 1 0 3-6.7", "M3 3v6h6"],
  save: ["M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z", "M17 21v-8H7v8", "M7 3v5h8"],
  search: ["M11 19a8 8 0 1 1 0-16 8 8 0 0 1 0 16Z", "M21 21l-4.3-4.3"],
  settings: ["M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z", "M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3h.1a1.7 1.7 0 0 0 .9-1.6V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 .9 1.6h.1a1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9v.1a1.7 1.7 0 0 0 1.6.9h.1a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.6.9Z"],
  shield: ["M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z", "M9 12l2 2 4-4"],
  sync: ["M17 2l4 4-4 4", "M3 11V9a4 4 0 0 1 4-4h14", "M7 22l-4-4 4-4", "M21 13v2a4 4 0 0 1-4 4H3"],
  trash: ["M3 6h18", "M8 6V4h8v2", "M19 6l-1 15H6L5 6", "M10 11v6", "M14 11v6"],
  wrench: ["M14.7 6.3a4 4 0 0 0-5 5L3 18l3 3 6.7-6.7a4 4 0 0 0 5-5L15 12l-3-3 2.7-2.7Z"],
};

const buttonIconObserver = new MutationObserver((mutations) => {
  const buttons = new Set();
  for (const mutation of mutations) {
    if (mutation.target instanceof HTMLButtonElement) {
      buttons.add(mutation.target);
    }
    mutation.addedNodes.forEach((node) => {
      if (node instanceof HTMLButtonElement) {
        buttons.add(node);
      } else if (node instanceof Element) {
        node.querySelectorAll("button").forEach((button) => buttons.add(button));
      }
    });
  }
  buttons.forEach(iconifyButton);
});

export function initButtonIcons(root = document) {
  root.querySelectorAll("button").forEach(iconifyButton);
  buttonIconObserver.observe(document.body, {
    childList: true,
    subtree: true,
    characterData: true,
  });
}

function iconifyButton(button) {
  if (button.dataset.iconRendering === "true" || button.closest(".window-controls")) {
    return;
  }

  const label = currentLabel(button);
  const iconName = iconForButton(button, label);
  if (!iconName) {
    button.classList.remove("has-icon", "icon-only");
    delete button.dataset.iconName;
    delete button.dataset.iconLabel;
    return;
  }

  const iconOnly = isIconOnlyButton(button, label);
  const visibleLabel = label === "⚙" ? "Ayarlar" : displayLabelForButton(button, label);
  if (
    button.dataset.iconName === iconName &&
    button.dataset.iconLabel === visibleLabel &&
    button.querySelector(".button-icon")
  ) {
    return;
  }

  button.dataset.iconRendering = "true";
  button.replaceChildren(iconElement(iconName));
  if (!iconOnly) {
    const labelSpan = document.createElement("span");
    labelSpan.className = "button-label";
    labelSpan.textContent = visibleLabel;
    button.append(labelSpan);
  }
  button.classList.add("has-icon");
  button.classList.toggle("icon-only", iconOnly);
  button.classList.toggle("reveal-label", shouldRevealLabelOnHover(button));
  button.classList.toggle("spinning-icon", shouldSpinIcon(button));
  if (shouldSuppressNativeTooltip(button)) {
    button.removeAttribute("title");
  } else {
    button.title = iconOnly ? visibleLabel : button.title || visibleLabel;
  }
  button.dataset.iconName = iconName;
  button.dataset.iconLabel = visibleLabel;
  if (iconOnly && !button.getAttribute("aria-label")) {
    button.setAttribute("aria-label", visibleLabel);
  }
  button.dataset.iconRendering = "false";
}

function currentLabel(button) {
  return (
    button.querySelector(".button-label")?.textContent?.trim() ||
    button.textContent.trim() ||
    button.dataset.iconLabel ||
    ""
  );
}

function iconElement(iconName) {
  const svg = document.createElementNS(svgNamespace, "svg");
  svg.setAttribute("class", "button-icon");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  for (const d of iconPaths[iconName]) {
    const path = document.createElementNS(svgNamespace, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function iconForButton(button, label) {
  if (button.classList.contains("game-settings-button") || button.dataset.action === "settings") {
    return "settings";
  }
  if (button.dataset.componentRemove) {
    return "trash";
  }
  if (button.dataset.componentInstall) {
    return label.toLocaleLowerCase("tr").includes("güncelle") ? "refresh" : "download";
  }
  if (button.dataset.action === "launch") {
    return "play";
  }
  if (button.dataset.action === "install-cncnet") {
    return "download";
  }

  const target = button.dataset.pageTarget;
  if (target === "library") return "library";
  if (target === "steam") return "gamepad";
  if (target === "compatibility") return "shield";
  if (target === "settings") return "settings";

  const idIcons = {
    "show-add-game": "add",
    "download-cancel": "close",
    "runtime-init": "wrench",
    "wine-install": "download",
    "graphics-install": "download",
    "emulator-init": "wrench",
    "pick-installer": "file",
    "pick-install-dir": "folder",
    "run-installer": "play",
    "steam-scan": "search",
    "steam-sync": "sync",
    "compat-refresh": "file",
    "settings-load": "refresh",
    "system-tools-refresh": "refresh",
    "metadata-fetch": "download",
    "updates-check": "refresh",
    "settings-close": "close",
    "game-settings-remove": "trash",
    "game-settings-reset": "reset",
    "game-settings-save": "save",
    "install-gamescope-game": "download",
    "apply-legacy-profile": "settings",
    "install-kdotool": "download",
    "install-gamescope-settings": "download",
    "remove-gamescope-settings": "trash",
    "install-fullscreen-tool-settings": "download",
    "remove-fullscreen-tool-settings": "trash",
  };
  if (idIcons[button.id]) {
    return idIcons[button.id];
  }

  const normalized = label.toLocaleLowerCase("tr");
  if (normalized.includes("kaydet")) return "save";
  if (normalized.includes("yenile")) return "refresh";
  if (normalized.includes("kaldır") || normalized.includes("sil")) return "trash";
  if (normalized.includes("kuruluyor") || normalized.includes("hazırlanıyor")) return "loader";
  if (normalized.includes("kur") || normalized.includes("indir") || normalized.includes("güncelle")) return "download";
  if (normalized.includes("hazırla")) return "wrench";
  if (normalized.includes("ekle")) return "add";
  if (normalized.includes("seç")) return normalized.includes("klasör") ? "folder" : "file";
  if (normalized.includes("çalıştır") || normalized.includes("başlat") || normalized.includes("çalışıyor")) return "play";
  if (normalized.includes("tara") || normalized.includes("ara")) return "search";
  if (normalized.includes("senkronize")) return "sync";
  if (normalized.includes("rapor")) return "file";
  if (normalized.includes("kontrol")) return "check";
  if (normalized.includes("protondb")) return "database";
  if (normalized.includes("kapak")) return "image";
  if (normalized.includes("kapat") || normalized.includes("iptal")) return "close";
  if (normalized.includes("sıfırla")) return "reset";
  if (normalized === "⚙") return "settings";
  return null;
}

function shouldRevealLabelOnHover(button) {
  return false;
}

function shouldSpinIcon(button) {
  return button.id === "settings-load" || button.id === "system-tools-refresh" || button.id === "updates-check";
}

function isIconOnlyButton(button, label) {
  return (
    button.classList.contains("game-settings-button") ||
    label === "⚙" ||
    button.id === "settings-load" ||
    button.id === "system-tools-refresh" ||
    button.id === "updates-check" ||
    Boolean(button.dataset.componentInstall) ||
    Boolean(button.dataset.componentRemove) ||
    button.id === "remove-gamescope-settings" ||
    button.id === "remove-fullscreen-tool-settings"
  );
}

function shouldSuppressNativeTooltip(button) {
  return button.id === "settings-load" || button.id === "system-tools-refresh" || button.id === "updates-check";
}

function displayLabelForButton(button, label) {
  if (button.dataset.componentInstall) {
    return label.toLocaleLowerCase("tr").includes("güncelle") ? "Güncelle" : "Kur";
  }
  if (button.dataset.componentRemove) {
    return "Kaldır";
  }
  if (button.id === "remove-gamescope-settings" || button.id === "remove-fullscreen-tool-settings") {
    return "Kaldır";
  }
  return label;
}
