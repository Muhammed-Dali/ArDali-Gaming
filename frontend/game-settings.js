import { initButtonIcons } from "./button-icons.js";

const settingsTitle = document.querySelector("#settings-title");
const settingsSubtitle = document.querySelector("#settings-subtitle");
const settingsAppIcon = document.querySelector("#settings-app-icon");
const sharedPrefixWarning = document.querySelector("#shared-prefix-warning");
const settingsDetails = document.querySelector("#settings-details");
const gameSettingsForm = document.querySelector("#game-settings-form");
const fullscreenToolWarning = document.querySelector("#fullscreen-tool-warning");
const installKdotoolButton = document.querySelector("#install-kdotool");
const installGamescopeGameButton = document.querySelector("#install-gamescope-game");
const applyLegacyProfileButton = document.querySelector("#apply-legacy-profile");
const fullscreenToolProgress = document.querySelector("#fullscreen-tool-progress");
const closeButton = document.querySelector("#settings-close");
const removeButton = document.querySelector("#game-settings-remove");
const windowTitlebar = document.querySelector(".settings-titlebar");
const windowTitle = document.querySelector(".settings-titlebar .window-title");
const windowControls = document.querySelector(".window-controls");

const tauriCore = window.__TAURI__?.core;
const tauriWindow = window.__TAURI__?.window;
const invoke = tauriCore?.invoke ?? mockInvoke;
const currentWindow = tauriWindow?.getCurrentWindow?.();
const gameId = Number(new URLSearchParams(window.location.search).get("id"));

let activeSettingsGame = null;
let fullscreenToolStatus = null;
let fullscreenToolProgressValue = null;
let fullscreenToolProgressHideTimer = null;
let fullscreenToolBusy = false;

setupWindowControls();
initButtonIcons();

gameSettingsForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveGameSettings();
});

document.querySelector("#game-settings-reset")?.addEventListener("click", () => {
  resetGameSettingsToDefaults();
});

removeButton?.addEventListener("click", async () => {
  await removeGameFromSettings();
});

installKdotoolButton?.addEventListener("click", async () => {
  await installKdotoolFromSettings();
});

installGamescopeGameButton?.addEventListener("click", async () => {
  await installGamescope("Gamescope kurulumu başlatılıyor Kurulumdan sonra bu oyun için Gamescope ölçekleme kullanılabilir");
});

applyLegacyProfileButton?.addEventListener("click", () => {
  applyLegacyGameProfile();
});

closeButton?.addEventListener("click", () => {
  closeSettingsWindow();
});

loadGameSettings();

function setupWindowControls() {
  if (!windowTitlebar || !currentWindow) {
    return;
  }

  windowTitlebar.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || event.target.closest("button")) {
      return;
    }
    currentWindow.startDragging?.();
  });

  windowControls?.addEventListener("click", async (event) => {
    const button = event.target.closest("button[data-window-action]");
    if (!button) {
      return;
    }

    const action = button.dataset.windowAction;
    if (action === "minimize") {
      await currentWindow.minimize?.();
    } else if (action === "maximize") {
      await currentWindow.toggleMaximize?.();
    } else if (action === "close") {
      await currentWindow.close?.();
    }
  });
}

async function loadGameSettings() {
  if (!gameId) {
    return;
  }

  try {
    const game = await invoke("game_settings", { id: gameId });
    showSettings(game);
  } catch (error) {
    console.error(error);
  }
}

function showSettings(game) {
  activeSettingsGame = game;
  gameSettingsForm.elements["game-id"].value = String(game.id);
  const title = `${game.name} Ayarları`;
  settingsTitle.textContent = title;
  settingsSubtitle.textContent = settingsSubtitleText(game);
  document.title = title;
  if (windowTitle) {
    windowTitle.textContent = title;
  }
  renderSettingsIcon(game);
  renderSettingsScopeNote(game);
  updateRemoveButton(game);
  settingsDetails.replaceChildren(
    settingsGroup(
      libraryTypeForGame(game) === "windows-app" ? "Uygulama Çalıştırma" : "Çalıştırma",
      settingsField("Runner", gameKindSelect(game.gameKind)),
      settingsField("Windows sürümü", windowsVersionSelect(game.windowsVersion)),
      settingsField("Wine prefix yolu", readonlyInput(game.prefixPath ?? "Yok"), true),
    ),
    settingsGroup(
      "Görüntü",
      settingsField("Ekran modu", displayModeSelect(game.displayMode)),
      settingsField("Çözünürlük", resolutionSelect(game.resolution)),
      settingsField("Ölçekleme modu", gamescopeScalerSelect(game.gamescopeScaler)),
      settingsField("Sanal Masaüstü", checkboxInput("virtual-desktop", game.virtualDesktop)),
      settingsField("Gamescope ölçekleme", checkboxInput("gamescope-enabled", game.gamescopeEnabled)),
    ),
    settingsGroup(
      "Uyumluluk",
      settingsField("DXVK", checkboxInput("dxvk-enabled", game.dxvkEnabled)),
      settingsField("ddraw override", checkboxInput("ddraw-override", game.ddrawOverride)),
      settingsField("DLL override", textInput("dll-override", game.dllOverride ?? "", "ddraw=n,b;dinput8=n,b"), true),
      settingsField("ProtonDB notu", textInput("protondb-note", game.protondbNote ?? "", "örn gold silver test edildi"), true),
    ),
  );
  syncDirectDrawSettings();
  gameSettingsForm.elements["ddraw-override"]?.addEventListener("change", syncDirectDrawSettings);
  gameSettingsForm.elements["windows-version"]?.addEventListener("change", applyWindowsVersionDefaults);
  gameSettingsForm.elements["display-mode"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["virtual-desktop"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["gamescope-enabled"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["game-kind"]?.addEventListener("change", renderFullscreenToolWarning);
  refreshFullscreenToolWarning(true);
}

async function saveGameSettings() {
  const form = new FormData(gameSettingsForm);
  const id = Number(form.get("game-id"));
  const gameKind = String(form.get("game-kind") ?? "windows-exe");
  const settings = {
    gameKind,
    preferredRunner: runnerForGameKind(gameKind),
    dxvkEnabled: form.get("ddraw-override") === "on" ? false : form.get("dxvk-enabled") === "on",
    dllOverride: String(form.get("dll-override") ?? "").trim() || null,
    displayMode: String(form.get("display-mode") ?? "windowed"),
    virtualDesktop: form.get("virtual-desktop") === "on",
    gamescopeEnabled: form.get("gamescope-enabled") === "on",
    resolution: String(form.get("resolution") ?? "auto"),
    gamescopeScaler: String(form.get("gamescope-scaler") ?? "fit"),
    protondbNote: String(form.get("protondb-note") ?? "").trim() || null,
    ddrawOverride: form.get("ddraw-override") === "on",
    windowsVersion: String(form.get("windows-version") ?? "win10"),
  };

  if (isSharedWindowsPrefix(activeSettingsGame)) {
    const confirmed = window.confirm(
      "Bu uygulama ortak Windows ortamını kullanıyor Wine uyumluluk ayarları aynı ortamı kullanan diğer Windows uygulamalarını da etkileyebilir Kaydetmek istiyor musunuz",
    );
    if (!confirmed) {
      return;
    }
  }

  try {
    await invoke("update_game_settings", { id, settings });
    closeSettingsWindow();
  } catch (error) {
    console.error(error);
  }
}

async function removeGameFromSettings() {
  const id = Number(gameSettingsForm?.elements["game-id"]?.value);
  if (!id) {
    return;
  }

  const wineInstalledApp = isWineInstalledApp(activeSettingsGame);
  const confirmed = window.confirm(
    wineInstalledApp
      ? "Bu uygulama Wine içinden kaldırılacak sonra kütüphane kartı silinecek"
      : "Bu kaydı kütüphaneden kaldırmak istiyor musunuz",
  );
  if (!confirmed) {
    return;
  }

  try {
    if (wineInstalledApp) {
      await invoke("uninstall_windows_app", { id });
    } else {
      await invoke("remove_game", { id, removeFiles: false });
    }
    closeSettingsWindow();
  } catch (error) {
    console.error(error);
  }
}

function updateRemoveButton(game) {
  if (!removeButton) {
    return;
  }
  removeButton.textContent = isWineInstalledApp(game)
    ? "Uygulamayı Wine’dan Kaldır"
    : "Kütüphaneden Sil";
  removeButton.title = removeButton.textContent;
}

function isWineInstalledApp(game) {
  const runner = game?.preferredRunner || game?.preferred_runner || game?.runner;
  return (
    libraryTypeForGame(game) === "windows-app" &&
    runner === "wine" &&
    Boolean(game?.prefixPath || game?.prefix_path)
  );
}

function libraryTypeForGame(game) {
  return game?.libraryType || game?.library_type || "game";
}

function isSharedWindowsPrefix(game) {
  const prefixPath = String(game?.prefixPath || game?.prefix_path || "");
  return libraryTypeForGame(game) === "windows-app" && prefixPath.includes("/prefixes/windows-apps/");
}

function renderSettingsScopeNote(game) {
  if (!sharedPrefixWarning) {
    return;
  }

  if (!isSharedWindowsPrefix(game)) {
    sharedPrefixWarning.hidden = true;
    sharedPrefixWarning.textContent = "";
    return;
  }

  sharedPrefixWarning.hidden = false;
  sharedPrefixWarning.textContent =
    "Bu uygulama ortak Windows ortamını kullanıyor Görüntü ayarları bu karta özeldir Windows sürümü DXVK DLL ve Wine prefix ayarları aynı ortamı kullanan diğer Windows uygulamalarını da etkileyebilir";
}

function renderSettingsIcon(game) {
  if (!settingsAppIcon) {
    return;
  }

  const coverPath = game?.coverPath || game?.cover_path;
  settingsAppIcon.classList.toggle("has-image", Boolean(coverPath));
  settingsAppIcon.replaceChildren();

  if (coverPath) {
    const image = document.createElement("img");
    image.src = coverImageSrc(coverPath);
    image.alt = "";
    image.addEventListener("error", () => {
      settingsAppIcon.classList.remove("has-image");
      settingsAppIcon.textContent = initials(game.name);
    });
    settingsAppIcon.append(image);
  } else {
    settingsAppIcon.textContent = initials(game.name);
  }
}

function coverImageSrc(path) {
  if (!path || /^(https?:|asset:|data:|blob:)/i.test(path)) {
    return path;
  }

  const convertFileSrc = window.__TAURI__?.core?.convertFileSrc;
  return convertFileSrc ? convertFileSrc(path) : path;
}

function initials(name) {
  return String(name ?? "A")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase();
}

function closeSettingsWindow() {
  if (currentWindow) {
    currentWindow.close();
  } else {
    window.close();
  }
}

function syncDirectDrawSettings() {
  const ddraw = gameSettingsForm?.elements["ddraw-override"];
  const dxvk = gameSettingsForm?.elements["dxvk-enabled"];
  if (!ddraw || !dxvk) {
    return;
  }

  dxvk.disabled = ddraw.checked;
  if (ddraw.checked) {
    dxvk.checked = false;
  }
}

function resetGameSettingsToDefaults() {
  const defaults = defaultSettingsForWindowsVersion(
    String(gameSettingsForm.elements["windows-version"]?.value ?? "win10"),
    activeSettingsGame,
  );
  applyDefaults(defaults);
}

function applyWindowsVersionDefaults() {
  const version = String(gameSettingsForm?.elements["windows-version"]?.value ?? "win10");
  applyDefaults(defaultSettingsForWindowsVersion(version, activeSettingsGame));
}

function applyLegacyGameProfile() {
  const defaults = defaultSettingsForWindowsVersion(
    String(gameSettingsForm.elements["windows-version"]?.value ?? "win10"),
    activeSettingsGame,
  );
  applyDefaults(defaults);
  renderFullscreenToolWarning();
}

function applyDefaults(defaults) {
  gameSettingsForm.elements["windows-version"].value = defaults.windowsVersion;
  gameSettingsForm.elements["dxvk-enabled"].checked = defaults.dxvkEnabled;
  gameSettingsForm.elements["dll-override"].value = defaults.dllOverride;
  gameSettingsForm.elements["display-mode"].value = defaults.displayMode;
  gameSettingsForm.elements["virtual-desktop"].checked = defaults.virtualDesktop;
  gameSettingsForm.elements["gamescope-enabled"].checked = defaults.gamescopeEnabled;
  gameSettingsForm.elements.resolution.value = defaults.resolution;
  gameSettingsForm.elements["gamescope-scaler"].value = defaults.gamescopeScaler;
  gameSettingsForm.elements["ddraw-override"].checked = defaults.ddrawOverride;
  syncDirectDrawSettings();
  renderFullscreenToolWarning();
}

async function refreshFullscreenToolWarning(force = false) {
  if (!fullscreenToolWarning) {
    return;
  }

  if (!force && fullscreenToolStatus) {
    renderFullscreenToolWarning();
    return;
  }

  try {
    fullscreenToolStatus = await invoke("fullscreen_tool_status");
  } catch (error) {
    console.error(error);
    fullscreenToolStatus = null;
  }
  renderFullscreenToolWarning();
}

function renderFullscreenToolWarning() {
  if (!fullscreenToolWarning) {
    return;
  }

  if (!needsFullscreenToolHelp()) {
    fullscreenToolWarning.hidden = true;
    return;
  }

  const windowToolInstalled = Boolean(fullscreenToolStatus?.hasRecommendedTool);
  const gamescopeInstalled = Boolean(fullscreenToolStatus?.gamescope);
  const windowToolSupported = fullscreenToolStatus?.recommendedTool !== "unsupported";
  const needsGamescope = needsGamescopeForCurrentGame();
  const needsWindowTool = needsWindowToolForCurrentGame();
  const message = fullscreenToolWarning.querySelector("span");
  const missing = [];
  const ready = [];
  if (needsGamescope) {
    (gamescopeInstalled ? ready : missing).push("Gamescope");
  }
  if (needsWindowTool && windowToolSupported) {
    (windowToolInstalled ? ready : missing).push(fullscreenToolStatus?.recommendedTool ?? "pencere aracı");
  }

  if (missing.length) {
    message.textContent = `${missing.join(" ve ")} kurulu değil Bu eski oyun profilinde tam ekran ölçekleme veya Wine Desktop büyütme düzgün çalışmayabilir`;
  } else if (needsWindowTool && !windowToolSupported && !gamescopeInstalled) {
    message.textContent = fullscreenToolStatus?.warning ?? "Bu masaüstünde pencere büyütme aracı desteklenmiyor Gamescope önerilir";
  } else if (ready.length) {
    message.textContent = `${ready.join(" ve ")} hazır Eski oyun ölçekleme profili kullanılabilir`;
  } else if (needsWindowTool && !windowToolSupported) {
    message.textContent = "Bu masaüstünde pencere büyütme aracı desteklenmiyor Gamescope ile ölçekleme kullanılabilir";
  } else {
    message.textContent = "Bu oyun için ek sistem aracı gerekmiyor";
  }

  fullscreenToolWarning.classList.toggle(
    "is-ready",
    (!needsGamescope || gamescopeInstalled) &&
      (!needsWindowTool || windowToolInstalled || !windowToolSupported),
  );
  fullscreenToolWarning.classList.toggle("is-busy", fullscreenToolBusy);
  installGamescopeGameButton.hidden = !needsGamescope || gamescopeInstalled;
  installGamescopeGameButton.disabled = fullscreenToolBusy;
  applyLegacyProfileButton.hidden = !hasGameProfile(activeSettingsGame);
  applyLegacyProfileButton.disabled = fullscreenToolBusy;
  installKdotoolButton.textContent = fullscreenToolBusy ? "Kuruluyor" : fullscreenToolStatus?.installLabel ?? "Tam Ekran Aracı Kur";
  installKdotoolButton.hidden = !windowToolSupported || !needsWindowTool || windowToolInstalled;
  installKdotoolButton.disabled = fullscreenToolBusy || !windowToolSupported;
  renderFullscreenToolProgress();
  fullscreenToolWarning.hidden = false;
}

function needsFullscreenToolHelp() {
  const gameKind = String(gameSettingsForm.elements["game-kind"]?.value ?? "windows-exe");
  return (
    runnerForGameKind(gameKind) === "wine" &&
    gameSettingsForm.elements["display-mode"]?.value === "fullscreen" &&
    (Boolean(gameSettingsForm.elements["virtual-desktop"]?.checked) ||
      Boolean(gameSettingsForm.elements["gamescope-enabled"]?.checked) ||
      hasGameProfile(activeSettingsGame))
  );
}

function needsGamescopeForCurrentGame() {
  return Boolean(gameSettingsForm?.elements["gamescope-enabled"]?.checked) || isLegacyFullscreenGame(activeSettingsGame);
}

function needsWindowToolForCurrentGame() {
  return Boolean(gameSettingsForm?.elements["virtual-desktop"]?.checked);
}

async function installKdotoolFromSettings() {
  fullscreenToolBusy = true;
  setFullscreenToolProgress(5);
  renderFullscreenToolWarning();

  try {
    fullscreenToolStatus = await invoke("install_kdotool");
    finishFullscreenToolProgress();
  } catch (error) {
    console.error(error);
  } finally {
    fullscreenToolBusy = false;
    await refreshFullscreenToolWarning(true);
  }
}

async function installGamescope(message) {
  fullscreenToolBusy = true;
  setFullscreenToolProgress(5);
  console.info(message);
  renderFullscreenToolWarning();

  try {
    fullscreenToolStatus = await invoke("install_gamescope");
    finishFullscreenToolProgress();
  } catch (error) {
    console.error(error);
  } finally {
    fullscreenToolBusy = false;
    await refreshFullscreenToolWarning(true);
  }
}

function setFullscreenToolProgress(value) {
  window.clearTimeout(fullscreenToolProgressHideTimer);
  fullscreenToolProgressValue = Math.max(0, Math.min(100, Number(value) || 0));
  renderFullscreenToolProgress();
}

function finishFullscreenToolProgress() {
  setFullscreenToolProgress(100);
  fullscreenToolProgressHideTimer = window.setTimeout(() => {
    fullscreenToolProgressValue = null;
    renderFullscreenToolProgress();
  }, 1500);
}

function renderFullscreenToolProgress() {
  if (!fullscreenToolProgress) {
    return;
  }

  if (fullscreenToolProgressValue === null) {
    fullscreenToolProgress.hidden = true;
    return;
  }

  fullscreenToolProgress.hidden = false;
  fullscreenToolProgress.querySelector("strong").textContent = `${fullscreenToolProgressValue}%`;
  fullscreenToolProgress.querySelector(".tool-progress-track div").style.width = `${fullscreenToolProgressValue}%`;
}

function defaultSettingsForWindowsVersion(version, game = null) {
  if (isCncNetLauncher(game)) {
    return defaults("win10", true, "fullscreen", false, false, "auto", "fit", false, "");
  }
  if (isSeriousSamGame(game)) {
    return defaults("win7", true, "fullscreen", true, true, "1024x768", "stretch", false, "");
  }
  if (isPopCapGame(game)) {
    return defaults("win7", true, "fullscreen", true, true, "800x600", "stretch", false, "");
  }
  if (isCncLegacyGame(game)) {
    return defaults("winxp", false, "fullscreen", true, true, "1024x768", "stretch", true, "ddraw=n,b;dinput8=n,b");
  }
  if (version === "winxp") {
    return defaults("winxp", false, "fullscreen", true, true, "auto", "stretch", true, "ddraw=n,b;dinput8=n,b");
  }
  return defaults(version || "win10", true, "windowed", false, false, "auto", "fit", false, "");
}

function defaults(windowsVersion, dxvkEnabled, displayMode, virtualDesktop, gamescopeEnabled, resolution, gamescopeScaler, ddrawOverride, dllOverride) {
  return { windowsVersion, dxvkEnabled, displayMode, virtualDesktop, gamescopeEnabled, resolution, gamescopeScaler, ddrawOverride, dllOverride };
}

function isSeriousSamGame(game) {
  const text = normalizedGameText(game);
  return text.includes("serioussam") || text.includes("serioussamclassic") || text.includes("samse") || text.includes("samfe");
}

function isPopCapGame(game) {
  const text = normalizedGameText(game);
  return ["zuma", "bejeweled", "peggle", "insaniquarium", "feedingfrenzy", "plantsvszombies", "popcap"].some((needle) => text.includes(needle));
}

function isCncLegacyGame(game) {
  const text = normalizedGameText(game);
  if (text.includes("cncnet")) {
    return false;
  }
  return ["ra2", "redalert", "redalert2", "yurisrevenge", "yuri", "tiberiansun"].some((needle) => text.includes(needle));
}

function isCncNetLauncher(game) {
  return normalizedGameText(game).includes("cncnet");
}

function isLegacyFullscreenGame(game) {
  return isSeriousSamGame(game) || isPopCapGame(game) || isCncLegacyGame(game);
}

function hasGameProfile(game) {
  return isLegacyFullscreenGame(game) || isCncNetLauncher(game);
}

function normalizedGameText(game) {
  if (!game) {
    return "";
  }
  return [game.name, game.installDir, game.installerPath, ...(game.arguments ?? [])].join(" ").toLowerCase().replace(/[\s_-]/g, "");
}

function settingsSubtitleText(game) {
  const parts = [
    game.runner || runnerForGameKind(game.gameKind ?? "windows-exe"),
    game.displayMode === "fullscreen" ? "Tam ekran" : "Pencere",
    game.windowsVersion?.replace("win", "Windows "),
  ].filter(Boolean);
  return parts.join(" / ");
}

function settingsGroup(title, ...fields) {
  const section = document.createElement("section");
  section.className = "settings-group";
  const heading = document.createElement("h3");
  heading.textContent = title;
  const grid = document.createElement("div");
  grid.className = "settings-group-grid";
  grid.append(...fields);
  section.append(heading, grid);
  return section;
}

function settingsField(label, control, wide = false) {
  const wrapper = document.createElement("label");
  wrapper.className = "settings-field";
  const isSwitch = control.classList?.contains("switch-input");
  if (isSwitch) {
    wrapper.classList.add("has-switch");
  }
  if (wide) {
    wrapper.classList.add("is-wide");
  }
  const title = document.createElement("span");
  title.textContent = label;
  if (isSwitch) {
    const switchVisual = document.createElement("span");
    switchVisual.className = "switch-control";
    switchVisual.setAttribute("aria-hidden", "true");
    wrapper.append(title, control, switchVisual);
  } else {
    wrapper.append(title, control);
  }
  return wrapper;
}

function gameKindSelect(value) {
  const select = document.createElement("select");
  select.name = "game-kind";
  [
    ["windows-exe", "Windows EXE"],
    ["windows-msi", "Windows MSI"],
    ["open-ra-red-alert", "OpenRA Red Alert"],
    ["open-ra-tiberian-dawn", "OpenRA Tiberian Dawn"],
    ["open-ra-dune2000", "OpenRA Dune 2000"],
    ["cncnet", "CnCNet"],
    ["dos", "DOSBox-X"],
    ["steam", "Steam"],
  ].forEach(([optionValue, label]) => select.append(option(optionValue, label, value || "windows-exe")));
  return select;
}

function runnerForGameKind(value) {
  if (value === "steam") return "steam-proton";
  if (value.startsWith("open-ra")) return "openra";
  if (value === "dos") return "dosbox-x";
  if (value === "cncnet") return "cncnet";
  return "wine";
}

function windowsVersionSelect(value) {
  const select = document.createElement("select");
  select.name = "windows-version";
  [
    ["winxp", "Windows XP"],
    ["win7", "Windows 7"],
    ["win10", "Windows 10"],
    ["win11", "Windows 11"],
  ].forEach(([optionValue, label]) => select.append(option(optionValue, label, value || "win10")));
  return select;
}

function displayModeSelect(value) {
  const select = document.createElement("select");
  select.name = "display-mode";
  select.append(option("windowed", "Pencere", value), option("fullscreen", "Tam ekran", value));
  return select;
}

function resolutionSelect(value) {
  const select = document.createElement("select");
  select.name = "resolution";
  [
    ["auto", "Otomatik kaynak"],
    ["1920x1080", "1920x1080"],
    ["1600x1200", "1600x1200"],
    ["1280x1024", "1280x1024"],
    ["1280x960", "1280x960"],
    ["1280x720", "1280x720"],
    ["1024x768", "1024x768"],
    ["800x600", "800x600"],
    ["640x480", "640x480"],
  ].forEach(([optionValue, label]) => select.append(option(optionValue, label, value)));
  return select;
}

function gamescopeScalerSelect(value) {
  const select = document.createElement("select");
  select.name = "gamescope-scaler";
  [
    ["fit", "Oranı koru"],
    ["stretch", "Ekrana yay"],
    ["fill", "Kırparak doldur"],
    ["integer", "Keskin piksel"],
    ["auto", "Otomatik"],
  ].forEach(([optionValue, label]) => select.append(option(optionValue, label, value || "fit")));
  return select;
}

function option(value, label, selectedValue) {
  const item = document.createElement("option");
  item.value = value;
  item.textContent = label;
  item.selected = value === selectedValue;
  return item;
}

function checkboxInput(name, checked) {
  const input = document.createElement("input");
  input.name = name;
  input.type = "checkbox";
  input.checked = Boolean(checked);
  input.className = "switch-input";
  return input;
}

function textInput(name, value, placeholder) {
  const input = document.createElement("input");
  input.name = name;
  input.value = value;
  input.placeholder = placeholder;
  input.autocomplete = "off";
  return input;
}

function readonlyInput(value) {
  const input = document.createElement("input");
  input.value = value;
  input.readOnly = true;
  return input;
}

function mockInvoke(command) {
  return Promise.reject(new Error(`${command} için Tauri backend yok`));
}
