import { initButtonIcons } from "./button-icons.js";

const addGameButton = document.querySelector(".toolbar button");
const navButtons = document.querySelectorAll("[data-page-target]");
const pageSections = document.querySelectorAll("[data-page]");
const toolbarTitle = document.querySelector(".toolbar h1");
const toolbarSubtitle = document.querySelector(".toolbar p");
const backendBanner = document.querySelector("#backend-banner");
const initButton = document.querySelector("#runtime-init");
const wineInstallButton = document.querySelector("#wine-install");
const legacyWineInstallButton = document.querySelector("#wine-legacy-install");
const graphicsInstallButton = document.querySelector("#graphics-install");
const emulatorInitButton = document.querySelector("#emulator-init");
const prefixForm = document.querySelector("#prefix-form");
const installForm = document.querySelector("#install-form");
const installPanel = document.querySelector("#install-panel");
const pickInstallerButton = document.querySelector("#pick-installer");
const pickInstallDirButton = document.querySelector("#pick-install-dir");
const runInstallerButton = document.querySelector("#run-installer");
const installModeMessage = document.querySelector("#windows-file-mode");
const installSubmitButton = document.querySelector("#install-form button[type='submit']");
const windowsFilePreview = document.querySelector("#windows-file-preview");
const gameSearch = document.querySelector("#game-search");
const libraryFilterButtons = document.querySelectorAll("[data-library-filter]");
const playStatus = document.querySelector("#play-status");
const appShell = document.querySelector(".app-shell");
const settingsDialog = document.querySelector("#settings-dialog");
const settingsDetails = document.querySelector("#settings-details");
const gameSettingsForm = document.querySelector("#game-settings-form");
const fullscreenToolWarning = document.querySelector("#fullscreen-tool-warning");
const installKdotoolButton = document.querySelector("#install-kdotool");
const installGamescopeGameButton = document.querySelector("#install-gamescope-game");
const applyLegacyProfileButton = document.querySelector("#apply-legacy-profile");
const fullscreenToolProgress = document.querySelector("#fullscreen-tool-progress");
const systemToolsRefreshButton = document.querySelector("#system-tools-refresh");
const installGamescopeSettingsButton = document.querySelector("#install-gamescope-settings");
const removeGamescopeSettingsButton = document.querySelector("#remove-gamescope-settings");
const installFullscreenToolSettingsButton = document.querySelector("#install-fullscreen-tool-settings");
const removeFullscreenToolSettingsButton = document.querySelector("#remove-fullscreen-tool-settings");
const steamScanButton = document.querySelector("#steam-scan");
const steamSyncButton = document.querySelector("#steam-sync");
const compatForm = document.querySelector("#compat-form");
const compatRefreshButton = document.querySelector("#compat-refresh");
const compatGame = document.querySelector("#compat-game");
const protondbForm = document.querySelector("#protondb-form");
const compatSettingsPath = document.querySelector("#compat-settings-path");
const compatLogPath = document.querySelector("#compat-log-path");
const protondbSummary = document.querySelector("#protondb-summary");
const compatLastError = document.querySelector("#compat-last-error");
const settingsLoadButton = document.querySelector("#settings-load");
const appSettingsForm = document.querySelector("#app-settings-form");
const metadataForm = document.querySelector("#metadata-form");
const metadataFetchButton = document.querySelector("#metadata-fetch");
const metadataGame = document.querySelector("#metadata-game");
const updatesCheckButton = document.querySelector("#updates-check");
const componentUpdateForm = document.querySelector("#component-update-form");
const updatesList = document.querySelector("#updates-list");
const librarySummary = document.querySelector("#library-summary");
const gameList = document.querySelector("#game-list");
const dataDir = document.querySelector("#data-dir");
const wineStatus = document.querySelector("#wine-status");
const protonStatus = document.querySelector("#proton-status");
const graphicsStatus = document.querySelector("#graphics-status");
const emulatorsDir = document.querySelector("#emulators-dir");
const openraStatus = document.querySelector("#openra-status");
const dosboxStatus = document.querySelector("#dosbox-status");
const steamRoot = document.querySelector("#steam-root");
const steamLibraries = document.querySelector("#steam-libraries");
const steamProton = document.querySelector("#steam-proton");
const steamGames = document.querySelector("#steam-games");
const gamescopeStatus = document.querySelector("#gamescope-status");
const fullscreenWindowToolStatus = document.querySelector("#fullscreen-window-tool-status");
const logList = document.querySelector("#runtime-logs");
const downloadProgress = document.querySelector("#download-progress");
const downloadProgressTitle = document.querySelector("#download-progress-title");
const downloadProgressPercent = document.querySelector("#download-progress-percent");
const downloadProgressBar = document.querySelector("#download-progress-bar");
const downloadProgressMeta = document.querySelector("#download-progress-meta");
const downloadCancelButton = document.querySelector("#download-cancel");
const windowTitlebar = document.querySelector(".window-titlebar");
const windowControls = document.querySelector(".window-controls");

const tauriCore = window.__TAURI__?.core;
const tauriEvent = window.__TAURI__?.event;
const tauriWindow = window.__TAURI__?.window;
const tauriAvailable = Boolean(tauriCore?.invoke);
const convertFileSrc = tauriCore?.convertFileSrc;
const currentWindow = tauriWindow?.getCurrentWindow?.();
let libraryGames = [];
const gameLaunchStates = new Map();
let progressHideTimer = null;
let progressTimer = null;
let progressStartedAt = 0;
let lastProgress = null;
const cncnetProgress = new Map();
let activeSettingsGame = null;
let fullscreenToolStatus = null;
let fullscreenToolProgressValue = null;
let fullscreenToolProgressHideTimer = null;
let fullscreenToolBusy = false;
let windowsPreviewToken = 0;
let activeLibraryFilter = "all";
const requestedWindowsIconIds = new Set();

const portableWineBundle = {
  component: "wine",
  version: "11.9-staging-tkg-amd64-wow64",
  url: "https://github.com/Kron4ek/Wine-Builds/releases/download/11.9/wine-11.9-staging-tkg-amd64-wow64.tar.xz",
};

const legacyWineBundle = {
  component: "wine",
  version: "10.0-amd64-wow64",
  url: "https://github.com/Kron4ek/Wine-Builds/releases/download/10.0/wine-10.0-amd64-wow64.tar.xz",
};

const recommendedComponentSources = {
  wine: {
    repo: "Kron4ek/Wine-Builds",
    asset: (name) => /^wine-[\d.]+-staging-tkg-amd64-wow64\.tar\.xz$/.test(name),
    version: (_release, asset) => asset.name.replace(/^wine-/, "").replace(/\.tar\.xz$/, ""),
  },
  proton: {
    repo: "GloriousEggroll/proton-ge-custom",
    asset: (name) => name.endsWith(".tar.gz") && !name.endsWith(".sha512sum"),
  },
  dxvk: {
    repo: "doitsujin/dxvk",
    asset: (name) => /^dxvk-[\d.]+\.tar\.gz$/.test(name),
  },
  vkd3d: {
    repo: "HansKristian-Work/vkd3d-proton",
    asset: (name) => /^vkd3d-proton-[\d.]+\.tar\.zst$/.test(name),
  },
  openra: {
    repo: "OpenRA/OpenRA",
    asset: (name) => name === "OpenRA-Red-Alert-x86_64.AppImage",
    versionSuffix: "red-alert",
  },
  "dosbox-x": {
    repo: "joncampbell123/dosbox-x",
    asset: (name) => name.endsWith(".AppImage") && /linux|x86_64|x64/i.test(name),
  },
  cncnet: {
    repo: "CnCNet/cncnet-client",
    asset: (name) =>
      name.endsWith(".AppImage") ||
      (/\.(tar\.gz|tgz|tar\.xz|tar\.zst|zip)$/i.test(name) &&
        /linux|x86_64|x64|appimage/i.test(name)),
  },
};

setupWindowControls();
setupSettingsDialogDrag();
initButtonIcons();

const pageLabels = {
  library: {
    title: "Kütüphane",
    subtitle: "Wine Proton ve emülatör yönetimi için başlangıç alanı",
  },
  steam: {
    title: "Steam",
    subtitle: "Yerel Steam kütüphanesini tara ve Proton kayıtlarıyla eşitle",
  },
  compatibility: {
    title: "Uyumluluk",
    subtitle: "Wine ayarları DLL override ProtonDB notu ve sorun giderme",
  },
  settings: {
    title: "Ayarlar",
    subtitle: "Genel ayarlar kapak metadata ve güncelleme yönetimi",
  },
};

const addButtonLabels = {
  all: "Ekle",
  game: "Oyun Ekle",
  "windows-app": "Uygulama Ekle",
  tool: "Araç Ekle",
  installer: "Kurulum Ekle",
};

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

function setupSettingsDialogDrag() {
  const header = settingsDialog?.querySelector("[data-dialog-drag]");
  if (!settingsDialog || !header) {
    return;
  }

  let dragState = null;

  header.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || event.target.closest("button")) {
      return;
    }

    const rect = settingsDialog.getBoundingClientRect();
    dragState = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      left: rect.left,
      top: rect.top,
    };
    header.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  });

  header.addEventListener("pointermove", (event) => {
    if (!dragState || event.pointerId !== dragState.pointerId) {
      return;
    }

    moveSettingsDialog(
      dragState.left + event.clientX - dragState.startX,
      dragState.top + event.clientY - dragState.startY,
    );
  });

  const stopDragging = (event) => {
    if (dragState && event.pointerId === dragState.pointerId) {
      dragState = null;
    }
  };
  header.addEventListener("pointerup", stopDragging);
  header.addEventListener("pointercancel", stopDragging);
  window.addEventListener("resize", () => {
    if (settingsDialog.open) {
      const rect = settingsDialog.getBoundingClientRect();
      moveSettingsDialog(rect.left, rect.top);
    }
  });
}

function centerSettingsDialog() {
  if (!settingsDialog) {
    return;
  }

  const rect = settingsDialog.getBoundingClientRect();
  moveSettingsDialog((window.innerWidth - rect.width) / 2, (window.innerHeight - rect.height) / 2);
}

function moveSettingsDialog(left, top) {
  if (!settingsDialog) {
    return;
  }

  const rect = settingsDialog.getBoundingClientRect();
  const padding = 14;
  const titlebarOffset = 40;
  const maxLeft = Math.max(padding, window.innerWidth - rect.width - padding);
  const maxTop = Math.max(titlebarOffset, window.innerHeight - rect.height - padding);
  const nextLeft = Math.min(Math.max(padding, left), maxLeft);
  const nextTop = Math.min(Math.max(titlebarOffset, top), maxTop);

  settingsDialog.style.position = "fixed";
  settingsDialog.style.margin = "0";
  settingsDialog.style.left = `${nextLeft}px`;
  settingsDialog.style.top = `${nextTop}px`;
}

navButtons.forEach((button) => {
  button.addEventListener("click", () => {
    showPage(button.dataset.pageTarget);
  });
});

document.addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button || button.disabled) {
    return;
  }

  button.classList.remove("clicked");
  void button.offsetWidth;
  button.classList.add("clicked");
  window.setTimeout(() => button.classList.remove("clicked"), 160);
});

addGameButton?.addEventListener("click", () => {
  showPage("library");
  installPanel?.classList.toggle("visible");
  installPanel?.scrollIntoView({ behavior: "smooth", block: "nearest" });
});

initButton?.addEventListener("click", async () => {
  await initializeRuntime();
});

wineInstallButton?.addEventListener("click", async () => {
  await installPortableWine();
});

legacyWineInstallButton?.addEventListener("click", async () => {
  await installLegacyWine();
});

graphicsInstallButton?.addEventListener("click", async () => {
  await installGraphicsStack();
});

emulatorInitButton?.addEventListener("click", async () => {
  await initializeEmulators();
});

prefixForm?.addEventListener("submit", async (event) => {
  event.preventDefault();

  const form = new FormData(prefixForm);
  const gameId = String(form.get("game-id") ?? "");
  const name = String(form.get("game-name") ?? "");

  try {
    const prefix = await invoke("create_wine_prefix", { gameId, name });
    appendLog("info", `Prefix ready: ${prefix.wineprefix}`);
    await invoke("install_dxvk_vkd3d", { prefixId: prefix.id });
  } catch (error) {
    appendLog("error", String(error));
  }
});

installForm?.addEventListener("submit", async (event) => {
  event.preventDefault();

  const request = installRequestFromForm();
  const validationError = validateInstallRequest(request, false);
  if (validationError) {
    appendLog("error", validationError);
    return;
  }
  if (request.libraryType === "installer") {
    appendLog("warn", "Bu dosya kurulum paketi Kur butonunu kullan");
    updateWindowsInstallMode();
    return;
  }

  try {
    const game = await invoke("add_game_installation", { request });
    appendLog("info", `SQLite kaydı eklendi: ${game.name}`);
    installForm.reset();
    clearWindowsFilePreview();
    await loadGames();
  } catch (error) {
    appendLog("error", String(error));
  }
});

pickInstallerButton?.addEventListener("click", async () => {
  try {
    const path = await invoke("pick_file");
    if (!path || !installForm) {
      return;
    }
    await selectInstallerPath(path);
  } catch (error) {
    appendLog("error", String(error));
  }
});

installForm?.elements["installer-path"]?.addEventListener("change", async (event) => {
  const path = event.target.value.trim();
  applyWindowsFileDefaults(path);
  updateWindowsInstallMode();
  await renderWindowsFilePreview(path);
});

async function selectInstallerPath(path) {
  if (!path || !installForm) {
    return;
  }
  installForm.elements["installer-path"].value = path;
  const nameInput = installForm.elements.name;
  if (!nameInput.value.trim()) {
    nameInput.value = guessGameName(path);
  }
  applyWindowsFileDefaults(path);
  updateWindowsInstallMode();
  await renderWindowsFilePreview(path);
  appendLog("info", `Windows dosyası seçildi: ${path}`);
}

async function renderWindowsFilePreview(path) {
  const token = ++windowsPreviewToken;
  if (!windowsFilePreview) {
    return;
  }
  if (!path) {
    clearWindowsFilePreview();
    return;
  }

  windowsFilePreview.hidden = false;
  windowsFilePreview.replaceChildren(previewIcon("..."), previewText("Dosya inceleniyor", path));
  try {
    const preview = await invoke("windows_file_preview", { path });
    if (token !== windowsPreviewToken) {
      return;
    }
    windowsFilePreview.replaceChildren(
      previewIcon(preview.kind, preview.iconPath),
      previewText(preview.name, preview.path, preview.kind),
    );
  } catch (error) {
    if (token !== windowsPreviewToken) {
      return;
    }
    windowsFilePreview.replaceChildren(previewIcon("EXE"), previewText("Önizleme alınamadı", path, String(error)));
  }
}

function clearWindowsFilePreview() {
  windowsPreviewToken += 1;
  if (!windowsFilePreview) {
    return;
  }
  windowsFilePreview.hidden = true;
  windowsFilePreview.replaceChildren();
}

function previewIcon(label, iconPath = null) {
  const icon = document.createElement("div");
  icon.className = "windows-file-preview-icon";
  if (iconPath) {
    const image = document.createElement("img");
    image.src = coverImageSrc(iconPath);
    image.alt = "";
    image.addEventListener("error", () => {
      icon.textContent = label;
    });
    icon.append(image);
  } else {
    icon.textContent = label;
  }
  return icon;
}

function previewText(name, path, meta = "") {
  const wrapper = document.createElement("div");
  wrapper.className = "windows-file-preview-text";
  const title = document.createElement("strong");
  title.textContent = name;
  const details = document.createElement("span");
  details.textContent = [meta, path].filter(Boolean).join(" · ");
  wrapper.append(title, details);
  return wrapper;
}

pickInstallDirButton?.addEventListener("click", async () => {
  try {
    const path = await invoke("pick_folder");
    if (!path || !installForm) {
      return;
    }
    installForm.elements["install-dir"].value = path;
    appendLog("info", `Kurulum klasörü seçildi: ${path}`);
  } catch (error) {
    appendLog("error", String(error));
  }
});

runInstallerButton?.addEventListener("click", async () => {
  if (!installForm || !runInstallerButton) {
    return;
  }

  const request = installRequestFromForm();
  const validationError = validateInstallRequest(request, false);
  if (validationError) {
    appendLog("error", validationError);
    return;
  }

  runInstallerButton.disabled = true;
  try {
    await invoke("run_game_installer", { request });
    appendLog("info", "Windows kurulumu başlatıldı Kurulum kapanınca otomatik kütüphaneye eklenecek");
  } catch (error) {
    appendLog("error", String(error));
  } finally {
    runInstallerButton.disabled = false;
    updateWindowsInstallMode();
  }
});

steamScanButton?.addEventListener("click", async () => {
  await scanSteam();
});

steamSyncButton?.addEventListener("click", async () => {
  try {
    const records = await invoke("sync_steam_library");
    appendLog("info", `Steam senkronizasyonu tamamlandı: ${records.length ?? 0} oyun`);
    await scanSteam();
    await loadGames();
  } catch (error) {
    appendLog("error", String(error));
  }
});

compatForm?.addEventListener("submit", async (event) => {
  event.preventDefault();

  const id = Number(compatGame?.value);
  const form = new FormData(compatForm);
  const dllName = String(form.get("dll-name") ?? "").trim();
    const settings = {
      wineVersion: String(form.get("wine-version") ?? "") || null,
      windowsVersion: null,
      dllOverrides: dllName
      ? [{ name: dllName, mode: String(form.get("dll-mode") ?? "native,builtin") }]
      : [],
    launchEnv: [],
  };

  try {
    const report = await invoke("save_compatibility_settings", { id, settings });
    renderCompatibilityReport(report);
    appendLog("info", "Uyumluluk ayarları kaydedildi");
  } catch (error) {
    appendLog("error", String(error));
  }
});

compatRefreshButton?.addEventListener("click", async () => {
  await refreshCompatibilityReport();
});

protondbForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const form = new FormData(protondbForm);
  const appId = String(form.get("app-id") ?? "");

  try {
    const summary = await invoke("fetch_protondb_summary", { appId });
    protondbSummary.textContent = summary.tier
      ? `${summary.tier} · ${summary.confidence ?? "confidence yok"}`
      : "Not bulunamadı";
  } catch (error) {
    appendLog("error", String(error));
  }
});

settingsLoadButton?.addEventListener("click", async () => {
  await withButtonLoading(settingsLoadButton, async () => {
    await loadSettings();
    await refreshSystemTools(true);
  });
});

systemToolsRefreshButton?.addEventListener("click", async () => {
  await withButtonLoading(systemToolsRefreshButton, () => refreshSystemTools(true));
});

installGamescopeSettingsButton?.addEventListener("click", async () => {
  await installGamescopeFromSettings();
});

removeGamescopeSettingsButton?.addEventListener("click", async () => {
  await removeGamescopeFromSettings();
});

installFullscreenToolSettingsButton?.addEventListener("click", async () => {
  await installKdotoolFromSettings();
});

removeFullscreenToolSettingsButton?.addEventListener("click", async () => {
  await removeFullscreenToolFromSettings();
});

appSettingsForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const form = new FormData(appSettingsForm);

  try {
    await invoke("set_setting", {
      key: "default_display_mode",
      value: String(form.get("default-display-mode") ?? "windowed"),
    });
    await invoke("set_setting", {
      key: "fps_overlay",
      value: String(form.get("fps-overlay") ?? "false"),
    });
    await loadSettings();
    appendLog("info", "Genel ayarlar kaydedildi");
  } catch (error) {
    appendLog("error", String(error));
  }
});

metadataForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const form = new FormData(metadataForm);
  const id = Number(metadataGame?.value);
  const keyValue = String(form.get("steamgriddb-key") ?? "");
  const coverPath = String(form.get("cover-path") ?? "");

  try {
    if (keyValue) {
      await invoke("set_setting", { key: "steamgriddb_api_key", value: keyValue });
    }
    if (coverPath) {
      await invoke("set_manual_cover", { id, coverPath });
      appendLog("info", "Manuel kapak kaydedildi");
      await loadGames();
    }
  } catch (error) {
    appendLog("error", String(error));
  }
});

metadataFetchButton?.addEventListener("click", async () => {
  const id = Number(metadataGame?.value);
  if (!id) {
    return;
  }

  try {
    const result = await invoke("fetch_game_metadata", { id });
    appendLog("info", `Metadata alındı: ${result.name}`);
    await loadGames();
  } catch (error) {
    appendLog("error", String(error));
  }
});

metadataGame?.addEventListener("change", updateFormStates);
metadataForm?.addEventListener("input", updateFormStates);
compatGame?.addEventListener("change", updateFormStates);
componentUpdateForm?.addEventListener("input", updateFormStates);
installForm?.addEventListener("input", updateWindowsInstallMode);
installForm?.addEventListener("change", updateWindowsInstallMode);

updatesCheckButton?.addEventListener("click", async () => {
  await withButtonLoading(updatesCheckButton, () => loadComponentUpdates());
});

downloadCancelButton?.addEventListener("click", async () => {
  downloadCancelButton.disabled = true;
  setDownloadProgress("Bileşen", Number(downloadProgressBar?.value ?? 1), "İptal ediliyor", "");
  try {
    await invoke("cancel_component_download");
  } catch (error) {
    appendLog("error", String(error));
  }
});

componentUpdateForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const form = new FormData(componentUpdateForm);
  const request = {
    component: String(form.get("component") ?? ""),
    version: String(form.get("version") ?? "") || null,
    url: String(form.get("url") ?? ""),
  };

  try {
    beginDownloadProgress(request.component);
    await invoke("update_component", { request });
    finishDownloadProgress(request.component, "Kurulum tamamlandı");
    appendLog("info", `${request.component} güncellemesi tamamlandı`);
    await loadComponentUpdates();
  } catch (error) {
    failDownloadProgress(request.component, String(error));
    appendLog("error", String(error));
  }
});

updatesList?.addEventListener("click", async (event) => {
  const button = event.target.closest("button[data-component-install], button[data-component-remove]");
  if (!button) {
    return;
  }

  button.disabled = true;
  const previousText = button.textContent;
  button.textContent = "Hazırlanıyor";
  try {
    if (button.dataset.componentInstall) {
      await installRecommendedComponent(button.dataset.componentInstall);
    } else if (button.dataset.componentRemove) {
      await removePortableComponent(button.dataset.componentRemove);
    }
  } finally {
    button.textContent = previousText;
    await loadComponentUpdates();
  }
});

async function removePortableComponent(component) {
  const label = normalizeComponentName(component);
  const confirmed = window.confirm(`${label} portable bileşeni kaldırılsın mı`);
  if (!confirmed) {
    return;
  }

  try {
    await invoke("remove_component", { component });
    appendLog("info", `${label} kaldırıldı`);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function installPortableWine() {
  if (!wineInstallButton) {
    return;
  }

  wineInstallButton.disabled = true;
  wineInstallButton.textContent = "Kuruluyor";

  try {
    const currentStatus = await initializeRuntime();
    if (currentStatus?.portable_wine_ready) {
      appendLog("info", "Portable Wine zaten kurulu");
      finishDownloadProgress(portableWineBundle.component, "Zaten kurulu");
      return;
    }

    beginDownloadProgress(portableWineBundle.component);
    await invoke("update_component", { request: portableWineBundle });
    finishDownloadProgress(portableWineBundle.component, "Kurulum tamamlandı");
    appendLog("info", "Portable Wine kuruldu Sistem Wine paketine dokunulmadı");
    await initializeRuntime();
    await loadComponentUpdates();
  } catch (error) {
    failDownloadProgress(portableWineBundle.component, String(error));
    appendLog("error", String(error));
  } finally {
    await initializeRuntime();
  }
}

async function installLegacyWine() {
  if (!legacyWineInstallButton) {
    return;
  }

  legacyWineInstallButton.disabled = true;
  const previousText = legacyWineInstallButton.textContent;
  legacyWineInstallButton.textContent = "Kuruluyor";

  try {
    beginDownloadProgress(legacyWineBundle.component);
    await invoke("update_component", { request: legacyWineBundle });
    finishDownloadProgress(legacyWineBundle.component, "Legacy Wine kuruldu");
    appendLog("info", "Legacy Wine kuruldu Eski InstallShield kurulumları için tekrar Kur deneyin");
    await initializeRuntime();
    await loadComponentUpdates();
  } catch (error) {
    failDownloadProgress(legacyWineBundle.component, String(error));
    appendLog("error", String(error));
  } finally {
    legacyWineInstallButton.textContent = previousText;
    legacyWineInstallButton.disabled = false;
    await initializeRuntime();
  }
}

async function installGraphicsStack() {
  if (!graphicsInstallButton) {
    return;
  }

  graphicsInstallButton.disabled = true;
  const previousText = graphicsInstallButton.textContent;
  graphicsInstallButton.textContent = "İndiriliyor";
  try {
    await installRecommendedComponent("dxvk");
    await installRecommendedComponent("vkd3d");
    await initializeRuntime();
    await loadComponentUpdates();
    appendLog("info", "DXVK/VKD3D portable bileşenleri hazırlandı");
  } finally {
    graphicsInstallButton.textContent = previousText;
    graphicsInstallButton.disabled = false;
  }
}

async function installRecommendedComponent(component) {
  let request = { component };
  beginDownloadProgress(component, "Güncel sürüm aranıyor", "GitHub release bilgisi alınıyor");
  try {
    request = await resolveRecommendedComponent(component);
    setDownloadProgress(
      normalizeComponentName(request.component),
      1,
      "İndirme hazırlanıyor",
      `${request.version} bulundu`,
    );
    await invoke("update_component", { request });
    finishDownloadProgress(request.component, "Kurulum tamamlandı");
    appendLog("info", `${normalizeComponentName(request.component)} güncel kurulum tamamlandı`);
    if (request.component === "wine") {
      await initializeRuntime();
    }
  } catch (error) {
    failDownloadProgress(request.component, String(error));
    appendLog("error", String(error));
  }
}

async function resolveRecommendedComponent(component) {
  const source = recommendedComponentSources[component];
  if (!source) {
    throw new Error(`${component} için hazır kurulum kaynağı yok`);
  }

  const response = await fetch(`https://api.github.com/repos/${source.repo}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!response.ok) {
    throw new Error(`${component} güncel sürüm bilgisi alınamadı: ${response.status}`);
  }

  const release = await response.json();
  const asset = release.assets?.find((item) => source.asset(item.name));
  if (!asset?.browser_download_url) {
    throw new Error(`${component} için uygun portable dosya bulunamadı`);
  }

  return {
    component,
    version: source.version
      ? source.version(release, asset)
      : [release.tag_name, source.versionSuffix].filter(Boolean).join("-"),
    url: asset.browser_download_url,
  };
}

gameSearch?.addEventListener("input", () => {
  renderGames(libraryGames);
});

libraryFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    activeLibraryFilter = button.dataset.libraryFilter ?? "all";
    libraryFilterButtons.forEach((item) => {
      item.classList.toggle("active", item === button);
    });
    updateAddButtonLabel();
    renderGames(libraryGames);
  });
});

gameSettingsForm?.addEventListener("submit", async (event) => {
  if (event.submitter?.value !== "save") {
    return;
  }
  event.preventDefault();
  await saveGameSettings();
});

document.querySelector("#game-settings-reset")?.addEventListener("click", () => {
  resetGameSettingsToDefaults();
});

document.querySelector("#game-settings-remove")?.addEventListener("click", async () => {
  await removeGameFromSettings();
});

installKdotoolButton?.addEventListener("click", async () => {
  await installKdotoolFromSettings();
});

installGamescopeGameButton?.addEventListener("click", async () => {
  await installGamescopeFromGame();
});

applyLegacyProfileButton?.addEventListener("click", () => {
  applyLegacyGameProfile();
});

gameList?.addEventListener("click", async (event) => {
  const button = event.target.closest("button[data-action]");
  if (!button) {
    return;
  }

  const id = Number(button.dataset.id);
  const action = button.dataset.action;

  try {
    if (action === "launch") {
      const options = currentGameModeOptions();
      gameLaunchStates.set(id, "launching");
      renderGames(libraryGames);
      const game = await invoke("launch_game", { id, options });
      gameLaunchStates.set(id, "running");
      appShell?.classList.add("playing");
      playStatus.textContent = `${game.name} çalışıyor ArDali Gaming arka planda`;
      appendLog("info", `Başlatıldı: ${game.name} (${options.displayMode})`);
      await loadGames();
    }

    if (action === "settings") {
      await invoke("open_game_settings_window", { id });
    }

    if (action === "install-cncnet") {
      setCncNetProgress(id, 1, "Hazırlanıyor");
      const game = await invoke("install_cncnet_for_game", { id });
      setCncNetProgress(id, 100, "Kuruldu");
      appendLog("info", `${game.name} için CnCNet kuruldu`);
      await loadGames();
    }

  } catch (error) {
    if (action === "launch") {
      gameLaunchStates.delete(id);
      renderGames(libraryGames);
    }
    appendLog("error", String(error));
  }
});

if (tauriEvent?.listen) {
  tauriEvent.listen("backend-log", (event) => {
    appendLog(event.payload.level, event.payload.message);
  });
  tauriEvent.listen("download-progress", (event) => {
    renderDownloadProgress(event.payload);
  });
  tauriEvent.listen("cncnet-install-progress", (event) => {
    const payload = event.payload;
    setCncNetProgress(Number(payload.id), Number(payload.percent ?? 1), payload.status ?? "");
  });
  tauriEvent.listen("game-ended", async (event) => {
    const payload = event.payload;
    gameLaunchStates.delete(Number(payload.id));
    appShell?.classList.remove("playing");
    playStatus.textContent = `${payload.name} kapandı ArDali Gaming geri döndü`;
    appendLog("info", `${payload.name} kapandı: ${payload.status}`);
    await loadGames();
  });
  tauriEvent.listen("library-changed", async () => {
    await loadGames();
    await refreshSystemTools(true);
  });
}

if (applyBackendAvailability()) {
  initializeRuntime();
  initializeEmulators();
  loadGames();
  scanSteam();
  loadSettings();
  refreshSystemTools();
  loadComponentUpdates();
}
updateFormStates();
updateWindowsInstallMode();
showPage("library");

function showPage(page) {
  const nextPage = pageLabels[page] ? page : "library";
  navButtons.forEach((button) => {
    button.classList.toggle("active", button.dataset.pageTarget === nextPage);
  });
  pageSections.forEach((section) => {
    section.hidden = section.dataset.page !== nextPage;
  });

  toolbarTitle.textContent = pageLabels[nextPage].title;
  toolbarSubtitle.textContent = pageLabels[nextPage].subtitle;
  addGameButton.hidden = nextPage !== "library";
  updateAddButtonLabel();
}

function updateAddButtonLabel() {
  if (!addGameButton) {
    return;
  }
  const label = addButtonLabels[activeLibraryFilter] ?? addButtonLabels.all;
  addGameButton.textContent = label;
  addGameButton.title = label;
  addGameButton.setAttribute("aria-label", label);
}

async function initializeRuntime() {
  try {
    const status = await invoke("initialize_runtime");
    renderRuntime(status);
    return status;
  } catch (error) {
    appendLog("error", String(error));
    return null;
  }
}

async function loadGames() {
  try {
    const games = await invoke("list_games");
    libraryGames = Array.isArray(games) ? games : [];
    await clearDetachedGameSessions();
    renderGames(libraryGames);
    renderCompatibilityGames(libraryGames);
    renderMetadataGames(libraryGames);
    await refreshMissingWindowsIcons();
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function refreshMissingWindowsIcons() {
  if (!tauriAvailable) {
    return;
  }
  const targets = libraryGames.filter(
    (game) =>
      ["windows-app", "tool"].includes(libraryTypeForGame(game)) &&
      !game.coverPath &&
      !requestedWindowsIconIds.has(game.id),
  );
  if (!targets.length) {
    return;
  }
  let refreshed = false;
  for (const game of targets) {
    requestedWindowsIconIds.add(game.id);
    try {
      const updated = await invoke("refresh_windows_app_icon", { id: game.id });
      Object.assign(game, updated);
      refreshed = refreshed || Boolean(updated.coverPath);
    } catch (error) {
      appendLog("warn", `${game.name} simgesi çıkarılamadı: ${error}`);
    }
  }
  if (refreshed) {
    renderGames(libraryGames);
  }
}

async function clearDetachedGameSessions() {
  if (!tauriAvailable) {
    return;
  }

  const staleGames = libraryGames.filter(
    (game) => game.activeSessionId && !gameLaunchStates.has(game.id),
  );
  if (!staleGames.length) {
    return;
  }

  for (const game of staleGames) {
    try {
      const cleared = await invoke("clear_game_session", { id: game.id });
      Object.assign(game, cleared);
      appendLog("info", `${game.name} önceki çalışma durumu otomatik temizlendi`);
    } catch (error) {
      appendLog("warn", `${game.name} çalışma durumu temizlenemedi: ${error}`);
    }
  }
}

async function refreshCompatibilityReport() {
  const id = Number(compatGame?.value);
  if (!id) {
    return;
  }

  try {
    const report = await invoke("compatibility_report", { id });
    renderCompatibilityReport(report);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function loadSettings() {
  try {
    const settings = await invoke("list_settings");
    renderSettings(Array.isArray(settings) ? settings : []);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function refreshSystemTools(force = false) {
  await refreshFullscreenToolWarning(force);
  renderSystemTools();
}

async function loadComponentUpdates() {
  try {
    const updates = await invoke("check_component_updates");
    const componentUpdates = Array.isArray(updates) ? updates : [];
    renderComponentUpdates(componentUpdates);
    await refreshRecommendedComponentUpdates(componentUpdates);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function withButtonLoading(button, task) {
  if (!button) {
    return task();
  }
  if (button.dataset.loading === "true") {
    return undefined;
  }

  button.dataset.loading = "true";
  button.classList.add("is-loading");
  button.setAttribute("aria-busy", "true");
  const startedAt = performance.now();
  try {
    return await task();
  } finally {
    const elapsed = performance.now() - startedAt;
    if (elapsed < 650) {
      await new Promise((resolve) => window.setTimeout(resolve, 650 - elapsed));
    }
    button.classList.remove("is-loading");
    button.removeAttribute("aria-busy");
    delete button.dataset.loading;
    updateFormStates();
  }
}

async function refreshRecommendedComponentUpdates(updates) {
  if (!updates.length || !window.fetch) {
    return;
  }

  const failures = [];
  const refreshed = await Promise.all(
    updates.map(async (item) => {
      try {
        const recommendation = await resolveRecommendedComponent(item.component);
        const availableVersion = recommendation.version ?? item.availableVersion;
        return {
          ...item,
          availableVersion,
          url: recommendation.url ?? item.url,
          updateAvailable:
            Boolean(item.installed) &&
            Boolean(availableVersion) &&
            availableVersion !== item.currentVersion,
        };
      } catch (error) {
        failures.push(item.component);
        return {
          ...item,
          updateCheckFailed: true,
          updateCheckError: String(error),
        };
      }
    }),
  );

  renderComponentUpdates(refreshed);
  if (failures.length) {
    appendLog("warn", `Güncelleme kontrol edilemedi: ${failures.join(" / ")}`);
  }
}

async function scanSteam() {
  try {
    const scan = await invoke("scan_steam");
    renderSteam(scan);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function initializeEmulators() {
  try {
    const status = await invoke("initialize_emulators");
    renderEmulators(status);
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function invoke(command, args = {}) {
  if (!tauriAvailable) {
    return {};
  }

  return tauriCore.invoke(command, args);
}

function applyBackendAvailability() {
  if (tauriAvailable) {
    backendBanner.hidden = true;
    return true;
  }

  backendBanner.hidden = false;
  playStatus.textContent = "Wine kurulumu için uygulamayı Tauri penceresinde açmalısın";
  dataDir.textContent = "Tauri penceresi gerekli";
  wineStatus.textContent = "Kurulmadı: tarayıcı önizlemesi";
  protonStatus.textContent = "Tauri gerekli";
  graphicsStatus.textContent = "Tauri gerekli";
  emulatorsDir.textContent = "Tauri penceresi gerekli";
  openraStatus.textContent = "Tauri gerekli";
  dosboxStatus.textContent = "Tauri gerekli";

  document
    .querySelectorAll(
      [
        "#runtime-init",
        "#wine-install",
        "#graphics-install",
        "#emulator-init",
        "#steam-scan",
        "#steam-sync",
        "#compat-refresh",
        "#settings-load",
        "#metadata-fetch",
        "#updates-check",
        "form button",
        "button[data-action]",
        "button[data-component-install]",
        "#pick-installer",
        "#pick-install-dir",
        "#run-installer",
      ].join(", "),
    )
    .forEach((button) => {
      button.disabled = true;
    });

  if (wineInstallButton) {
    wineInstallButton.textContent = "Tauri ile aç";
  }
  appendLog("warn", "Tarayıcı önizlemesinde backend yok Wine kurulumu başlamadı");
  return false;
}

function renderRuntime(status) {
  if (!status?.paths) {
    return;
  }

  dataDir.textContent = status.paths.data_dir;
  wineStatus.textContent = runtimeWineLabel(status);
  protonStatus.textContent = runtimeProtonLabel(status);
  graphicsStatus.textContent =
    status.dxvk_ready && status.vkd3d_ready ? "Hazır" : "DXVK/VKD3D bekleniyor";

  if (wineInstallButton) {
    wineInstallButton.disabled = Boolean(status.portable_wine_ready);
    wineInstallButton.textContent = status.portable_wine_ready ? "Wine Hazır" : "Wine Kur";
  }
  if (graphicsInstallButton) {
    const ready = Boolean(status.dxvk_ready && status.vkd3d_ready);
    graphicsInstallButton.textContent = ready ? "DXVK/VKD3D Hazır" : "DXVK/VKD3D İndir";
  }
}

function runtimeWineLabel(status) {
  if (status.portable_wine_ready) {
    return "Portable hazır";
  }
  if (status.system_wine_version) {
    return status.system_wine_compatible
      ? `Sistem Wine: ${status.system_wine_version}`
      : `Uyumsuz olabilir: ${status.system_wine_version}`;
  }
  return "Wine bekleniyor";
}

function runtimeProtonLabel(status) {
  if (status.portable_proton_ready) {
    return "Portable hazır";
  }
  if (status.system_proton_version) {
    return `Sistem Proton: ${status.system_proton_version}`;
  }
  return "Proton bekleniyor";
}

function beginDownloadProgress(component, status = "Bağlanıyor", meta = "1% ile başladı indirme boyutu ölçülüyor") {
  if (!downloadProgress) {
    return;
  }

  window.clearTimeout(progressHideTimer);
  window.clearInterval(progressTimer);
  progressStartedAt = Date.now();
  downloadProgress.hidden = false;
  if (downloadCancelButton) {
    downloadCancelButton.disabled = false;
  }
  setDownloadProgress(normalizeComponentName(component), 1, status, meta);
  progressTimer = window.setInterval(refreshProgressTimer, 1000);
}

function finishDownloadProgress(component, status) {
  setDownloadProgress(normalizeComponentName(component), 100, status, "100% tamamlandı");
  if (downloadCancelButton) {
    downloadCancelButton.disabled = true;
  }
  window.clearInterval(progressTimer);
  progressHideTimer = window.setTimeout(() => {
    downloadProgress.hidden = true;
  }, 3500);
}

function failDownloadProgress(component, message) {
  const percent = Number(downloadProgressBar?.value ?? 1);
  setDownloadProgress(normalizeComponentName(component), percent, "Durdu", message);
  if (downloadCancelButton) {
    downloadCancelButton.disabled = true;
  }
  window.clearInterval(progressTimer);
}

function renderDownloadProgress(payload) {
  if (!payload) {
    return;
  }

  const percent = Number(payload.percent ?? 1);
  const downloaded = Number(payload.downloadedBytes ?? 0);
  const total = Number(payload.totalBytes ?? 0);
  const meta =
    total > 0
      ? `${formatBytes(downloaded)} / ${formatBytes(total)}`
      : downloaded > 0
        ? `${formatBytes(downloaded)} indirildi`
        : "İndirme başlatıldı";

  setDownloadProgress(
    normalizeComponentName(payload.kind),
    percent,
    payload.status ?? "İndiriliyor",
    meta,
  );
}

function setDownloadProgress(componentName, percent, status, meta) {
  if (!downloadProgress || !downloadProgressBar) {
    return;
  }

  const value = Math.max(1, Math.min(100, Math.round(Number(percent) || 1)));
  downloadProgress.hidden = false;
  lastProgress = { componentName, value, status, meta };
  downloadProgressTitle.textContent = `${componentName}: ${status}`;
  downloadProgressPercent.textContent = `${value}%`;
  downloadProgressBar.value = value;
  downloadProgressMeta.textContent = progressMetaWithElapsed(meta);
  if (downloadCancelButton && value >= 100) {
    downloadCancelButton.disabled = true;
  }
}

function refreshProgressTimer() {
  if (!lastProgress || downloadProgress?.hidden) {
    return;
  }
  downloadProgressMeta.textContent = progressMetaWithElapsed(lastProgress.meta);
}

function progressMetaWithElapsed(meta) {
  const elapsed = progressStartedAt ? `Süre: ${formatElapsed(Date.now() - progressStartedAt)}` : "";
  return [meta, elapsed].filter(Boolean).join(" · ");
}

function formatElapsed(milliseconds) {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function normalizeComponentName(value) {
  const names = {
    wine: "Wine",
    proton: "Proton",
    dxvk: "DXVK",
    vkd3d: "VKD3D",
    openra: "OpenRA",
    "dosbox-x": "DOSBox-X",
    cncnet: "CnCNet",
  };
  return names[value] ?? value ?? "Bileşen";
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

function installRequestFromForm() {
  const form = new FormData(installForm);
  return {
    name: String(form.get("name") ?? "").trim(),
    gameKind: String(form.get("game-kind") ?? ""),
    libraryType: String(form.get("library-type") ?? "game"),
    preferredRunner: String(form.get("preferred-runner") ?? "auto"),
    prefixMode: String(form.get("prefix-mode") ?? "isolated"),
    installerPath: String(form.get("installer-path") ?? "").trim(),
    installDir: String(form.get("install-dir") ?? "").trim() || null,
  };
}

function validateInstallRequest(request, requireInstallDir) {
  if (!request.name) {
    return "Kütüphaneye eklemek için ad girmelisin";
  }
  if (!request.installerPath) {
    return "Windows dosyası veya hedef dosya seçmelisin";
  }
  if (requireInstallDir && !request.installDir) {
    return "Installer çalıştırmak için kurulum klasörü seçmelisin";
  }
  if (request.gameKind === "windows-exe" && !/\.exe$/i.test(request.installerPath)) {
    return "Windows EXE türü için .exe dosyası seçmelisin";
  }
  if (request.gameKind === "windows-msi" && !/\.msi$/i.test(request.installerPath)) {
    return "Windows MSI türü için .msi dosyası seçmelisin";
  }
  return "";
}

function applyWindowsFileDefaults(path) {
  if (!installForm) {
    return;
  }

  const gameKindSelect = installForm.elements["game-kind"];
  const libraryTypeSelect = installForm.elements["library-type"];
  const fileName = path.split("/").pop() ?? path;
  const isMsi = /\.msi$/i.test(fileName);
  const isWindowsFile = /\.(exe|msi)$/i.test(fileName);
  const mode = windowsFileMode(path);
  if (gameKindSelect && isWindowsFile) {
    gameKindSelect.value = isMsi ? "windows-msi" : "windows-exe";
  }
  if (libraryTypeSelect && isWindowsFile) {
    libraryTypeSelect.value = mode === "installer" ? "installer" : "windows-app";
  }
  const prefixModeSelect = installForm.elements["prefix-mode"];
  if (prefixModeSelect && isWindowsFile) {
    prefixModeSelect.value = mode === "installer" ? "isolated" : "shared-windows-apps";
  }
}

function updateWindowsInstallMode() {
  if (!installForm) {
    return;
  }

  const request = installRequestFromForm();
  const mode = windowsFileMode(request.installerPath);
  const isWindowsFile = /\.(exe|msi)$/i.test(request.installerPath);

  if (!isWindowsFile) {
    if (installModeMessage) {
      installModeMessage.textContent = "Windows dosyası seçilince kurulum modu otomatik belirlenecek";
    }
    if (runInstallerButton) {
      runInstallerButton.textContent = "Kur";
      runInstallerButton.disabled = false;
    }
    if (installSubmitButton) {
      installSubmitButton.textContent = "Kütüphaneye Ekle";
      installSubmitButton.disabled = false;
    }
    return;
  }

  if (mode === "installer") {
    const libraryTypeSelect = installForm.elements["library-type"];
    if (libraryTypeSelect) {
      libraryTypeSelect.value = "installer";
    }
    const prefixModeSelect = installForm.elements["prefix-mode"];
    if (prefixModeSelect) {
      prefixModeSelect.value = "isolated";
    }
    if (installModeMessage) {
      installModeMessage.textContent =
        "Kurulum gerekiyor ayrı Windows ortamında kurulur bitince Windows uygulaması olarak eklenir";
    }
    if (runInstallerButton) {
      runInstallerButton.textContent = "Kur";
      runInstallerButton.disabled = false;
    }
    if (installSubmitButton) {
      installSubmitButton.textContent = "Doğrudan Ekle";
      installSubmitButton.disabled = true;
      installSubmitButton.title = "Kurulum paketi için Kur butonunu kullan";
    }
    return;
  }

  if (mode === "direct") {
    const libraryTypeSelect = installForm.elements["library-type"];
    if (libraryTypeSelect) {
      libraryTypeSelect.value = "windows-app";
    }
    const prefixModeSelect = installForm.elements["prefix-mode"];
    if (prefixModeSelect) {
      prefixModeSelect.value = "shared-windows-apps";
    }
    if (installModeMessage) {
      installModeMessage.textContent = "Bu dosya doğrudan çalışabilir Kütüphaneye Ekle ile kaydedilir";
    }
    if (runInstallerButton) {
      runInstallerButton.textContent = "Kur";
      runInstallerButton.disabled = true;
    }
    if (installSubmitButton) {
      installSubmitButton.textContent = "Kütüphaneye Ekle";
      installSubmitButton.disabled = false;
      installSubmitButton.removeAttribute("title");
    }
    return;
  }

  if (installModeMessage) {
    installModeMessage.textContent =
      "Dosya tipi net değil kurulum gerekiyorsa Kur doğrudan çalışıyorsa Kütüphaneye Ekle";
  }
  if (runInstallerButton) {
    runInstallerButton.textContent = "Kur";
    runInstallerButton.disabled = false;
  }
  if (installSubmitButton) {
    installSubmitButton.textContent = "Kütüphaneye Ekle";
    installSubmitButton.disabled = false;
    installSubmitButton.removeAttribute("title");
  }
}

function windowsFileMode(path) {
  const fileName = String(path || "")
    .split("/")
    .pop()
    ?.toLowerCase() ?? "";
  if (!fileName) {
    return "unknown";
  }
  if (/\.msi$/i.test(fileName)) {
    return "installer";
  }
  if (!/\.exe$/i.test(fileName)) {
    return "unknown";
  }
  if (/\b(anydesk|rufus)\b/i.test(fileName)) {
    return "direct";
  }
  if (
    /(setup|install|installer|kurulum|chrome|brave|steam|opera|jetaudio|jet-audio|jet_audio)/i.test(
      fileName,
    ) ||
    /^7z[\w.-]*\.exe$/i.test(fileName) ||
    /free-offline|websetup|bootstrap|launcher/i.test(fileName)
  ) {
    return "installer";
  }
  return "unknown";
}

function guessGameName(path) {
  const file = path.split("/").pop() ?? path;
  return file
    .replace(/\.(exe|msi)$/i, "")
    .replace(/[_-]+/g, " ")
    .replace(/\b(installer|setup)\b/gi, "")
    .replace(/\s+/g, " ")
    .trim();
}

function renderEmulators(status) {
  if (!status) {
    return;
  }

  emulatorsDir.textContent = status.emulators_dir ?? "Bekleniyor";
  openraStatus.textContent = [
    status.openra_red_alert_ready ? "RA" : null,
    status.openra_tiberian_dawn_ready ? "TD" : null,
    status.openra_dune_2000_ready ? "D2K" : null,
  ]
    .filter(Boolean)
    .join(", ") || "Portable AppImage bekleniyor";
  dosboxStatus.textContent = status.dosbox_x_ready ? "Hazır" : "Portable DOSBox-X bekleniyor";
}

function renderGames(games) {
  if (!gameList || !librarySummary) {
    return;
  }

  const query = gameSearch?.value?.trim().toLowerCase() ?? "";
  const filteredGames = games.filter((game) => {
    const matchesSearch = !query || game.name.toLowerCase().includes(query);
    const matchesType =
      activeLibraryFilter === "all" || libraryTypeForGame(game) === activeLibraryFilter;
    return matchesSearch && matchesType;
  });

  librarySummary.textContent = librarySummaryText(games, filteredGames);
  if (!filteredGames.length) {
    const empty = document.createElement("div");
    empty.className = "empty-library-state";
    empty.textContent = games.length
      ? "Bu filtrede kayıt yok"
      : "Henüz kütüphane kaydı yok";
    gameList.replaceChildren(empty);
    return;
  }

  gameList.replaceChildren(
    ...filteredGames.map((game) => {
      const libraryType = libraryTypeForGame(game);
      const usesAppIcon = libraryType === "windows-app" || libraryType === "tool";
      const row = document.createElement("article");
      row.className = "game-card";
      row.classList.toggle("game-card-app", usesAppIcon);

      const cover = document.createElement("div");
      cover.className = "cover";
      cover.classList.toggle("cover-app-icon", usesAppIcon);
      if (game.coverPath) {
        const image = document.createElement("img");
        image.src = coverImageSrc(game.coverPath);
        image.alt = "";
        image.addEventListener("error", () => {
          cover.replaceChildren(initials(game.name));
        });
        cover.append(image);
      } else {
        cover.textContent = initials(game.name);
      }

      const body = document.createElement("div");
      body.className = "game-card-body";

      const title = document.createElement("strong");
      const titleHeading = document.createElement("h3");
      title.textContent = game.name;
      titleHeading.append(title);

      const meta = document.createElement("div");
      meta.className = "game-card-meta";
      meta.append(
        badge(libraryTypeLabel(libraryType), "type"),
        badge(runnerLabel(game), "runner"),
      );

      const settings = actionButton("⚙", "settings", game.id);
      settings.className = "game-settings-button";
      settings.setAttribute("aria-label", `${game.name} ayarları`);

      const actions = document.createElement("div");
      actions.className = "game-actions";
      actions.append(playButton(game));
      if (shouldShowCncNetInstall(game)) {
        const install = actionButton("CnCNet Kur", "install-cncnet", game.id);
        install.className = "cncnet-install-button";
        actions.append(install);
      }
      const progress = cncnetProgress.get(game.id);
      if (progress && progress.percent < 100) {
        actions.append(cncNetProgressElement(progress));
      }

      body.append(titleHeading, meta);
      row.append(settings, cover, body, actions);
      return row;
    }),
  );
}

function badge(text, variant) {
  const element = document.createElement("span");
  element.className = `library-badge ${variant ? `library-badge-${variant}` : ""}`;
  element.textContent = text;
  return element;
}

function librarySummaryText(games, filteredGames) {
  if (!games.length) {
    return "Kayıt yok";
  }

  if (filteredGames.length === games.length && activeLibraryFilter === "all") {
    return `${games.length} kayıt`;
  }

  const typeLabel = libraryTypeShortLabel(activeLibraryFilter);
  return `${filteredGames.length} ${typeLabel} / ${games.length} toplam`;
}

function libraryTypeForGame(game) {
  return game?.libraryType || game?.library_type || "game";
}

function libraryTypeLabel(value) {
  const labels = {
    game: "Oyun",
    "windows-app": "Windows Uygulaması",
    tool: "Araç",
    installer: "Kurulum",
  };
  return labels[value] ?? "Kayıt";
}

function libraryTypeShortLabel(value) {
  const labels = {
    game: "oyun",
    "windows-app": "Windows",
    tool: "araç",
    installer: "kurulum",
  };
  return labels[value] ?? "kayıt";
}

function runnerLabel(game) {
  const runner = game.preferredRunner || game.runner || runnerForGameKind(game.gameKind ?? "");
  const labels = {
    wine: "Wine",
    proton: "Proton",
    "steam-proton": "Steam Proton",
    openra: "OpenRA",
    "dosbox-x": "DOSBox-X",
    cncnet: "CnCNet",
  };
  return labels[runner] ?? runner ?? "Runner";
}

function gameDetailText(game) {
  const parts = [gameKindLabel(game.gameKind)];
  if (game.windowsVersion && runnerLabel(game).includes("Wine")) {
    parts.push(String(game.windowsVersion).replace(/^win/i, "Windows "));
  }
  if (game.installDir) {
    parts.push(shortPath(game.installDir));
  }
  return parts.filter(Boolean).join(" · ");
}

function gameKindLabel(value) {
  const labels = {
    "windows-exe": "EXE",
    "windows-msi": "MSI",
    steam: "Steam",
    "open-ra-red-alert": "OpenRA RA",
    "open-ra-tiberian-dawn": "OpenRA TD",
    "open-ra-dune2000": "OpenRA D2K",
    dos: "DOS",
    cncnet: "CnCNet",
  };
  return labels[value] ?? value;
}

function shortPath(path) {
  const parts = String(path).split("/").filter(Boolean);
  if (parts.length <= 2) {
    return path;
  }
  return `…/${parts.slice(-2).join("/")}`;
}

function shouldShowCncNetInstall(game) {
  return game.gameKind === "windows-exe" && isCncNetGame(game) && !game.cncnetInstalled;
}

function isCncNetGame(game) {
  const combined = [game.name, game.installDir, game.installerPath]
    .filter(Boolean)
    .join(" ")
    .toLowerCase()
    .replace(/[\s_-]/g, "");
  return [
    "ra2",
    "redalert",
    "redalert2",
    "yurisrevenge",
    "yuri",
    "tiberiansun",
    "commandandconquer",
  ].some((needle) => combined.includes(needle));
}

function playButton(game) {
  const state = gameLaunchStates.get(game.id) ?? (game.activeSessionId ? "running" : null);
  const label = state === "launching" ? "Başlatılıyor" : state === "running" ? "Çalışıyor" : "Başlat";
  const button = actionButton(label, "launch", game.id);
  button.classList.add("play-button");
  if (state) {
    button.disabled = true;
    button.classList.add(`is-${state}`);
  }
  return button;
}

function cncNetProgressElement(progress) {
  const wrapper = document.createElement("div");
  wrapper.className = "cncnet-progress";
  const label = document.createElement("span");
  label.textContent = progress.status || "CnCNet kuruluyor";
  const bar = document.createElement("progress");
  bar.max = 100;
  bar.value = Math.max(1, Math.min(100, progress.percent || 1));
  wrapper.append(label, bar);
  return wrapper;
}

function setCncNetProgress(id, percent, status) {
  if (!id) {
    return;
  }
  cncnetProgress.set(id, { percent, status });
  renderGames(libraryGames);
  if (percent >= 100) {
    window.setTimeout(() => {
      cncnetProgress.delete(id);
      renderGames(libraryGames);
    }, 1800);
  }
}

function coverImageSrc(path) {
  if (!path || /^(https?:|asset:|data:|blob:)/i.test(path)) {
    return path;
  }

  return convertFileSrc ? convertFileSrc(path) : path;
}

function renderCompatibilityGames(games) {
  if (!compatGame) {
    return;
  }

  const current = compatGame.value;
  compatGame.replaceChildren(
    placeholderOption("Oyun seç"),
    ...games.map((game) => {
      const option = document.createElement("option");
      option.value = String(game.id);
      option.textContent = game.name;
      return option;
    }),
  );
  if (current) {
    compatGame.value = current;
  }
  updateFormStates();
}

function renderMetadataGames(games) {
  if (!metadataGame) {
    return;
  }

  const current = metadataGame.value;
  metadataGame.replaceChildren(
    placeholderOption("Oyun seç"),
    ...games.map((game) => {
      const option = document.createElement("option");
      option.value = String(game.id);
      option.textContent = game.name;
      return option;
    }),
  );
  if (current) {
    metadataGame.value = current;
  }
  updateFormStates();
}

function placeholderOption(label) {
  const option = document.createElement("option");
  option.value = "";
  option.textContent = label;
  return option;
}

function renderCompatibilityReport(report) {
  compatSettingsPath.textContent = report.settingsPath ?? "Yok";
  compatLogPath.textContent = report.logPath ?? "Yok";
  compatLastError.textContent = report.recentLogs?.[0] ?? "Yok";
}

function renderSettings(settings) {
  const map = Object.fromEntries(settings.map((setting) => [setting.key, setting.value]));
  if (map.default_display_mode) {
    appSettingsForm.elements["default-display-mode"].value = map.default_display_mode;
  }
  if (map.fps_overlay) {
    appSettingsForm.elements["fps-overlay"].value = map.fps_overlay;
  }
  if (metadataForm?.elements["steamgriddb-key"] && map.steamgriddb_api_key) {
    metadataForm.elements["steamgriddb-key"].value = map.steamgriddb_api_key;
  }
}

function renderSystemTools() {
  const gamescopeReady = Boolean(fullscreenToolStatus?.gamescope);
  const windowToolReady = Boolean(fullscreenToolStatus?.hasRecommendedTool);
  const windowToolSupported = fullscreenToolStatus?.recommendedTool !== "unsupported";

  if (gamescopeStatus) {
    gamescopeStatus.textContent = gamescopeReady
      ? "Kurulu - eski oyun ölçekleme hazır"
      : "Kurulu değil - pacman dnf zypper veya apt-get ile kurulabilir";
  }
  if (fullscreenWindowToolStatus) {
    fullscreenWindowToolStatus.textContent = windowToolReady
      ? `${fullscreenToolStatus?.recommendedTool ?? "Araç"} hazır`
      : fullscreenToolStatus?.warning ?? "Pencere büyütme aracı kurulu değil";
  }
  if (installGamescopeSettingsButton) {
    installGamescopeSettingsButton.hidden = gamescopeReady;
    installGamescopeSettingsButton.disabled = fullscreenToolBusy;
  }
  if (removeGamescopeSettingsButton) {
    removeGamescopeSettingsButton.hidden = !gamescopeReady;
    removeGamescopeSettingsButton.disabled = fullscreenToolBusy;
  }
  if (installFullscreenToolSettingsButton) {
    installFullscreenToolSettingsButton.hidden = !windowToolSupported || windowToolReady;
    installFullscreenToolSettingsButton.disabled = fullscreenToolBusy || !windowToolSupported;
    installFullscreenToolSettingsButton.textContent =
      fullscreenToolStatus?.installLabel ?? "Araç Kur";
  }
  if (removeFullscreenToolSettingsButton) {
    removeFullscreenToolSettingsButton.hidden = !windowToolSupported || !windowToolReady;
    removeFullscreenToolSettingsButton.disabled = fullscreenToolBusy || !windowToolSupported;
    removeFullscreenToolSettingsButton.textContent =
      `${fullscreenToolStatus?.recommendedTool ?? "Araç"} Kaldır`;
  }
}

function updateFormStates() {
  const hasCompatGame = Boolean(compatGame?.value);
  const compatSubmit = compatForm?.querySelector('button[type="submit"]');
  if (compatSubmit) {
    compatSubmit.disabled = !hasCompatGame;
  }
  if (compatRefreshButton) {
    compatRefreshButton.disabled = !hasCompatGame;
  }

  const hasMetadataGame = Boolean(metadataGame?.value);
  const coverPath = metadataForm?.elements["cover-path"]?.value?.trim();
  const metadataSubmit = metadataForm?.querySelector('button[type="submit"]');
  if (metadataSubmit) {
    metadataSubmit.disabled = !hasMetadataGame || !coverPath;
  }
  if (metadataFetchButton) {
    metadataFetchButton.disabled = !hasMetadataGame;
  }

  const updateUrl = componentUpdateForm?.elements.url?.value?.trim();
  const updateSubmit = componentUpdateForm?.querySelector('button[type="submit"]');
  if (updateSubmit) {
    updateSubmit.disabled = !updateUrl;
  }
}

function renderComponentUpdates(updates) {
  if (!updatesList) {
    return;
  }

  updatesList.replaceChildren(
    ...updates.map((item) => {
      const row = document.createElement("article");
      row.className = "game-row update-row";

      const title = document.createElement("strong");
      title.textContent = item.component;

      const state = document.createElement("span");
      state.textContent = item.updateCheckFailed
        ? "Kontrol edilemedi"
        : item.installed
          ? item.updateAvailable
            ? "Güncelleme var"
            : item.source === "system"
              ? "Sistemde kurulu"
              : "Kurulu"
          : "Bekliyor";
      if (item.updateCheckError) {
        state.title = item.updateCheckError;
      }

      const version = document.createElement("span");
      version.textContent = item.updateAvailable
        ? `${item.currentVersion ?? "Sürüm yok"} → ${item.availableVersion}`
        : item.currentVersion ?? "Sürüm yok";

      const actions = document.createElement("div");
      actions.className = "update-actions";

      const installAction = document.createElement("button");
      installAction.type = "button";
      installAction.dataset.componentInstall = item.component;
      installAction.textContent = item.updateAvailable ? "Güncelle" : "Kur";
      installAction.title = item.updateAvailable ? "Güncelle" : "Kur";
      if (!item.installed || item.updateAvailable) {
        actions.append(installAction);
      }

      if (item.removable) {
        const removeAction = document.createElement("button");
        removeAction.type = "button";
        removeAction.dataset.componentRemove = item.component;
        removeAction.textContent = "Kaldır";
        removeAction.title = "Kaldır";
        actions.append(removeAction);
      }

      row.append(title, state, version, actions);
      return row;
    }),
  );
}

function actionButton(label, action, id) {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.action = action;
  button.dataset.id = String(id);
  button.textContent = label;
  button.title = label;
  return button;
}

function showSettings(game) {
  if (!settingsDialog || !settingsDetails) {
    return;
  }

  activeSettingsGame = game;
  gameSettingsForm.elements["game-id"].value = String(game.id);
  document.querySelector("#settings-title").textContent = `${game.name} Ayarları`;
  const removeButton = document.querySelector("#game-settings-remove");
  if (removeButton) {
    removeButton.textContent = removeLabelForGame(game);
    removeButton.title = removeButton.textContent;
  }
  settingsDetails.replaceChildren(
    settingsField("Runner", gameKindSelect(game.gameKind)),
    settingsField("Windows sürümü", windowsVersionSelect(game.windowsVersion)),
    settingsField("Wine prefix yolu", readonlyInput(game.prefixPath ?? "Yok")),
    settingsField("DXVK", checkboxInput("dxvk-enabled", game.dxvkEnabled)),
    settingsField(
      "DLL override",
      textInput("dll-override", game.dllOverride ?? "", "ddraw=n,b;dinput8=n,b"),
    ),
    settingsField("Ekran modu", displayModeSelect(game.displayMode)),
    settingsField("Sanal Masaüstü", checkboxInput("virtual-desktop", game.virtualDesktop)),
    settingsField("Gamescope ölçekleme", checkboxInput("gamescope-enabled", game.gamescopeEnabled)),
    settingsField("Çözünürlük", resolutionSelect(game.resolution)),
    settingsField("Ölçekleme modu", gamescopeScalerSelect(game.gamescopeScaler)),
    settingsField("ddraw override", checkboxInput("ddraw-override", game.ddrawOverride)),
    settingsField(
      "ProtonDB notu",
      textInput("protondb-note", game.protondbNote ?? "", "örn gold silver test edildi"),
    ),
  );
  syncDirectDrawSettings();
  gameSettingsForm.elements["ddraw-override"]?.addEventListener("change", syncDirectDrawSettings);
  gameSettingsForm.elements["windows-version"]?.addEventListener("change", applyWindowsVersionDefaults);
  gameSettingsForm.elements["display-mode"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["virtual-desktop"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["gamescope-enabled"]?.addEventListener("change", renderFullscreenToolWarning);
  gameSettingsForm.elements["game-kind"]?.addEventListener("change", renderFullscreenToolWarning);
  settingsDialog.showModal();
  requestAnimationFrame(centerSettingsDialog);
  refreshFullscreenToolWarning();
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

  try {
    const game = await invoke("update_game_settings", { id, settings });
    appendLog("info", `${game.name} ayarları kaydedildi`);
    settingsDialog.close();
    await loadGames();
  } catch (error) {
    appendLog("error", String(error));
  }
}

async function removeGameFromSettings() {
  const id = Number(gameSettingsForm?.elements["game-id"]?.value);
  if (!id) {
    return;
  }

  const game = libraryGames.find((item) => item.id === id);
  const wineInstalledApp = game && isWineInstalledApp(game);
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
    settingsDialog.close();
    appendLog(
      "info",
      wineInstalledApp
        ? `${game?.name ?? "Uygulama"} Wine içinden kaldırıldı`
        : `${game?.name ?? "Kayıt"} kütüphaneden kaldırıldı`,
    );
    await loadGames();
  } catch (error) {
    appendLog("error", String(error));
  }
}

function isWineInstalledApp(game) {
  const runner = game?.preferredRunner || game?.preferred_runner || game?.runner;
  return (
    libraryTypeForGame(game) === "windows-app" &&
    runner === "wine" &&
    Boolean(game.prefixPath || game.prefix_path)
  );
}

function removeLabelForGame(game) {
  return isWineInstalledApp(game) ? "Uygulamayı Wine’dan Kaldır" : "Kütüphaneden Sil";
}

function syncDirectDrawSettings() {
  const ddraw = gameSettingsForm?.elements["ddraw-override"];
  const dxvk = gameSettingsForm?.elements["dxvk-enabled"];
  if (!ddraw || !dxvk) {
    return;
  }

  if (ddraw.checked) {
    dxvk.checked = false;
    dxvk.disabled = true;
  } else {
    dxvk.disabled = false;
  }
}

function resetGameSettingsToDefaults() {
  if (!gameSettingsForm) {
    return;
  }

  const defaults = defaultSettingsForWindowsVersion(
    String(gameSettingsForm.elements["windows-version"]?.value ?? "win10"),
    activeSettingsGame,
  );
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

function applyWindowsVersionDefaults() {
  const version = String(gameSettingsForm?.elements["windows-version"]?.value ?? "win10");
  const defaults = defaultSettingsForWindowsVersion(version, activeSettingsGame);
  gameSettingsForm.elements["dxvk-enabled"].checked = defaults.dxvkEnabled;
  gameSettingsForm.elements["dll-override"].value = defaults.dllOverride;
  gameSettingsForm.elements["virtual-desktop"].checked = defaults.virtualDesktop;
  gameSettingsForm.elements["gamescope-enabled"].checked = defaults.gamescopeEnabled;
  gameSettingsForm.elements["ddraw-override"].checked = defaults.ddrawOverride;
  gameSettingsForm.elements["gamescope-scaler"].value = defaults.gamescopeScaler;
  syncDirectDrawSettings();
  renderFullscreenToolWarning();
}

function applyLegacyGameProfile() {
  if (!gameSettingsForm) {
    return;
  }

  const defaults = defaultSettingsForWindowsVersion(
    String(gameSettingsForm.elements["windows-version"]?.value ?? "win10"),
    activeSettingsGame,
  );
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
  appendLog("info", "Eski oyun profili ayarlara uygulandı Kalıcı olması için Kaydet");
}

async function refreshFullscreenToolWarning(force = false) {
  if (!fullscreenToolWarning) {
    return;
  }

  if (!tauriAvailable) {
    fullscreenToolWarning.hidden = true;
    return;
  }

  if (!force && fullscreenToolStatus) {
    renderFullscreenToolWarning();
    renderSystemTools();
    return;
  }

  try {
    fullscreenToolStatus = await invoke("fullscreen_tool_status");
  } catch (error) {
    appendLog("warn", `Tam ekran aracı denetlenemedi: ${error}`);
    fullscreenToolStatus = null;
  }
  renderFullscreenToolWarning();
  renderSystemTools();
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
  if (message) {
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
      message.textContent =
        fullscreenToolStatus?.warning ??
        "Bu masaüstünde pencere büyütme aracı desteklenmiyor Gamescope önerilir";
    } else if (ready.length) {
      message.textContent = `${ready.join(" ve ")} hazır Eski oyun ölçekleme profili kullanılabilir`;
    } else if (needsWindowTool && !windowToolSupported) {
      message.textContent = "Bu masaüstünde pencere büyütme aracı desteklenmiyor Gamescope ile ölçekleme kullanılabilir";
    } else {
      message.textContent = "Bu oyun için ek sistem aracı gerekmiyor";
    }
  }
  fullscreenToolWarning.classList.toggle(
    "is-ready",
    (!needsGamescope || gamescopeInstalled) &&
      (!needsWindowTool || windowToolInstalled || !windowToolSupported),
  );
  fullscreenToolWarning.classList.toggle("is-busy", fullscreenToolBusy);
  if (installGamescopeGameButton) {
    installGamescopeGameButton.hidden = !needsGamescope || gamescopeInstalled;
    installGamescopeGameButton.disabled = fullscreenToolBusy;
  }
  if (applyLegacyProfileButton) {
    applyLegacyProfileButton.hidden = !hasGameProfile(activeSettingsGame);
    applyLegacyProfileButton.disabled = fullscreenToolBusy;
  }
  if (installKdotoolButton) {
    installKdotoolButton.textContent = fullscreenToolBusy
      ? "Kuruluyor"
      : fullscreenToolStatus?.installLabel ?? "Tam Ekran Aracı Kur";
    installKdotoolButton.title = fullscreenToolStatus?.recommendedTool
      ? `${fullscreenToolStatus.recommendedTool} kur`
      : "Tam ekran aracı kur";
    installKdotoolButton.hidden = !windowToolSupported || !needsWindowTool || windowToolInstalled;
    installKdotoolButton.disabled = fullscreenToolBusy || !windowToolSupported;
  }
  renderFullscreenToolProgress();
  fullscreenToolWarning.hidden = false;
}

function needsFullscreenToolHelp() {
  if (!gameSettingsForm) {
    return false;
  }

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
  return (
    Boolean(gameSettingsForm?.elements["gamescope-enabled"]?.checked) ||
    isLegacyFullscreenGame(activeSettingsGame)
  );
}

function needsWindowToolForCurrentGame() {
  return Boolean(gameSettingsForm?.elements["virtual-desktop"]?.checked);
}

async function installKdotoolFromSettings() {
  if (!installKdotoolButton && !installFullscreenToolSettingsButton) {
    return;
  }

  fullscreenToolBusy = true;
  setFullscreenToolProgress(5);
  appendLog("info", "Tam ekran aracı kurulumu sistem terminalinde başlatılıyor Parola istenebilir");
  renderFullscreenToolWarning();
  renderSystemTools();

  try {
    fullscreenToolStatus = await invoke("install_kdotool");
    finishFullscreenToolProgress();
    appendLog(
      "info",
      fullscreenToolStatus?.hasRecommendedTool
        ? `${fullscreenToolStatus.recommendedTool} hazır`
        : "Kurulum tamamlandı ama önerilen tam ekran aracı bulunamadı",
    );
  } catch (error) {
    appendLog("error", String(error));
  } finally {
    fullscreenToolBusy = false;
    await refreshFullscreenToolWarning(true);
  }
}

async function installGamescopeFromSettings() {
  await installGamescope("Gamescope kurulumu sistem terminalinde başlatılıyor Parola istenebilir");
}

async function installGamescopeFromGame() {
  await installGamescope(
    "Gamescope kurulumu başlatılıyor Kurulumdan sonra bu oyun için Gamescope ölçekleme kullanılabilir",
  );
}

async function installGamescope(message) {
  fullscreenToolBusy = true;
  setFullscreenToolProgress(5);
  appendLog("info", message);
  renderFullscreenToolWarning();
  renderSystemTools();

  try {
    fullscreenToolStatus = await invoke("install_gamescope");
    finishFullscreenToolProgress();
    appendLog(
      "info",
      fullscreenToolStatus?.gamescope
        ? "Gamescope hazır"
        : "Kurulum tamamlandı ama Gamescope bulunamadı",
    );
  } catch (error) {
    appendLog("error", String(error));
  } finally {
    fullscreenToolBusy = false;
    await refreshFullscreenToolWarning(true);
  }
}

async function removeGamescopeFromSettings() {
  const confirmed = window.confirm("Gamescope sistemden kaldırılsın mı");
  if (!confirmed) {
    return;
  }

  fullscreenToolBusy = true;
  setFullscreenToolProgress(10);
  appendLog("info", "Gamescope kaldırma işlemi sistem terminalinde başlatılıyor");
  renderFullscreenToolWarning();
  renderSystemTools();

  try {
    fullscreenToolStatus = await invoke("remove_gamescope");
    finishFullscreenToolProgress();
    appendLog(
      "info",
      fullscreenToolStatus?.gamescope ? "Gamescope kaldırılamadı" : "Gamescope kaldırıldı",
    );
  } catch (error) {
    appendLog("error", String(error));
  } finally {
    fullscreenToolBusy = false;
    await refreshFullscreenToolWarning(true);
  }
}

async function removeFullscreenToolFromSettings() {
  if (!removeFullscreenToolSettingsButton) {
    return;
  }

  const tool = fullscreenToolStatus?.recommendedTool ?? "tam ekran aracı";
  const confirmed = window.confirm(
    `${tool} sistemden temiz kaldırılsın mı Kurulum kalıntıları da temizlenir`,
  );
  if (!confirmed) {
    return;
  }

  fullscreenToolBusy = true;
  setFullscreenToolProgress(10);
  appendLog("info", `${tool} temiz kaldırma işlemi sistem terminalinde başlatılıyor`);
  renderSystemTools();

  try {
    fullscreenToolStatus = await invoke("remove_fullscreen_tool");
    finishFullscreenToolProgress();
    appendLog(
      "info",
      fullscreenToolStatus?.hasRecommendedTool
        ? `${tool} kaldırılamadı araç hâlâ algılanıyor`
        : `${tool} sistemden temiz kaldırıldı`,
    );
  } catch (error) {
    appendLog("error", String(error));
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

  const value = fullscreenToolProgressValue;
  fullscreenToolProgress.hidden = false;
  fullscreenToolProgress.querySelector("strong").textContent = `${value}%`;
  fullscreenToolProgress.querySelector(".tool-progress-track div").style.width = `${value}%`;
}

function defaultSettingsForWindowsVersion(version, game = null) {
  if (isCncNetLauncher(game)) {
    return {
      windowsVersion: "win10",
      dxvkEnabled: true,
      displayMode: "fullscreen",
      virtualDesktop: false,
      gamescopeEnabled: false,
      resolution: "auto",
      gamescopeScaler: "fit",
      ddrawOverride: false,
      dllOverride: "",
    };
  }

  if (isSeriousSamGame(game)) {
    return {
      windowsVersion: "win7",
      dxvkEnabled: true,
      displayMode: "fullscreen",
      virtualDesktop: true,
      gamescopeEnabled: true,
      resolution: "1024x768",
      gamescopeScaler: "stretch",
      ddrawOverride: false,
      dllOverride: "",
    };
  }

  if (isPopCapGame(game)) {
    return {
      windowsVersion: "win7",
      dxvkEnabled: true,
      displayMode: "fullscreen",
      virtualDesktop: true,
      gamescopeEnabled: true,
      resolution: "800x600",
      gamescopeScaler: "stretch",
      ddrawOverride: false,
      dllOverride: "",
    };
  }

  if (isCncLegacyGame(game)) {
    return {
      windowsVersion: "winxp",
      dxvkEnabled: false,
      displayMode: "fullscreen",
      virtualDesktop: true,
      gamescopeEnabled: true,
      resolution: "1024x768",
      gamescopeScaler: "stretch",
      ddrawOverride: true,
      dllOverride: "ddraw=n,b;dinput8=n,b",
    };
  }

  if (version === "winxp") {
    return {
      windowsVersion: "winxp",
      dxvkEnabled: false,
      displayMode: "fullscreen",
      virtualDesktop: true,
      gamescopeEnabled: true,
      resolution: "auto",
      gamescopeScaler: "stretch",
      ddrawOverride: true,
      dllOverride: "ddraw=n,b;dinput8=n,b",
    };
  }

  return {
    windowsVersion: version || "win10",
    dxvkEnabled: true,
    displayMode: "windowed",
    virtualDesktop: false,
    gamescopeEnabled: false,
    resolution: "auto",
    gamescopeScaler: "fit",
    ddrawOverride: false,
    dllOverride: "",
  };
}

function isSeriousSamGame(game) {
  const text = normalizedGameText(game);
  return (
    text.includes("serioussam") ||
    text.includes("serioussamclassic") ||
    text.includes("samse") ||
    text.includes("samfe")
  );
}

function isPopCapGame(game) {
  const text = normalizedGameText(game);
  return ["zuma", "bejeweled", "peggle", "insaniquarium", "feedingfrenzy", "plantsvszombies", "popcap"].some(
    (needle) => text.includes(needle),
  );
}

function isCncLegacyGame(game) {
  const text = normalizedGameText(game);
  if (text.includes("cncnet")) {
    return false;
  }
  return ["ra2", "redalert", "redalert2", "yurisrevenge", "yuri", "tiberiansun"].some(
    (needle) => text.includes(needle),
  );
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

  return [game.name, game.installDir, game.installerPath, ...(game.arguments ?? [])]
    .join(" ")
    .toLowerCase()
    .replace(/[\s_-]/g, "");
}

function settingsField(label, control) {
  const wrapper = document.createElement("label");
  wrapper.className = "settings-field";
  const title = document.createElement("span");
  title.textContent = label;
  wrapper.append(title, control);
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
  if (value === "steam") {
    return "steam-proton";
  }
  if (value.startsWith("open-ra")) {
    return "openra";
  }
  if (value === "dos") {
    return "dosbox-x";
  }
  if (value === "cncnet") {
    return "cncnet";
  }
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

function currentGameModeOptions() {
  return {
    displayMode: "windowed",
    fpsOverlay: false,
  };
}

function initials(name) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
}

function lastPlayedLabel(timestamp) {
  if (!timestamp) {
    return "Henüz oynanmadı";
  }

  return new Date(timestamp * 1000).toLocaleDateString();
}

function playtimeLabel(seconds) {
  const total = Number(seconds ?? 0);
  if (total <= 0) {
    return "0 dk";
  }

  const hours = Math.floor(total / 3600);
  const minutes = Math.max(1, Math.floor((total % 3600) / 60));
  return hours > 0 ? `${hours} sa ${minutes} dk` : `${minutes} dk`;
}

function renderSteam(scan) {
  if (!scan) {
    return;
  }

  steamRoot.textContent = summarizeList(scan.steamRoots);
  steamLibraries.textContent = `${scan.libraryDirs?.length ?? 0} kütüphane`;
  steamProton.textContent = summarizeList(scan.protonVersions);
  steamGames.textContent = `${scan.games?.length ?? 0} oyun bulundu`;
}

function summarizeList(items) {
  if (!Array.isArray(items) || items.length === 0) {
    return "Bulunamadı";
  }

  return items.length === 1 ? items[0] : `${items.length} kayıt`;
}

function appendLog(level, message) {
  if (!logList) {
    return;
  }

  const item = document.createElement("li");
  item.textContent = `${new Date().toLocaleTimeString()} [${level}] ${message}`;
  logList.prepend(item);
}
