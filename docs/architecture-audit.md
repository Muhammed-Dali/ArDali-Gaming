# ArDali Gaming Architecture Audit

Status: Step 1 complete baseline

## Objective

Keep the existing Tauri application working while moving game-management behavior into a UI-independent Rust core. Tauri remains the primary UI. A later CLI and small GTK4 proof of concept will verify that the core is portable; they are not immediate rewrites.

## Current shape

ArDali Gaming is currently one Rust package under `src-tauri/` with a static HTML/CSS/JavaScript frontend.

```text
frontend (HTML/CSS/JavaScript)
        |
        | 42 registered Tauri commands / 37 commands currently invoked by JS
        v
src-tauri/src/lib.rs
        |-- database.rs
        |-- runtime.rs
        |-- steam.rs
        |-- updates.rs
        |-- compatibility.rs
        `-- metadata.rs
```

Observed baseline:

- `lib.rs` is about 87 KB and combines UI adapters with application behavior.
- The backend registers 42 Tauri commands.
- Rust sources contain about 50 `AppHandle` references.
- There are no Rust unit or integration tests yet.
- SQLite is bundled through `rusqlite`.
- The frontend is served directly from `frontend/`; there is no frontend build step.
- Runtime data is stored under `~/.local/share/ardali-gaming/`.

## Module responsibilities and coupling

| Module | Current responsibilities | Tauri coupling | Extraction target |
| --- | --- | --- | --- |
| `lib.rs` | Tauri commands, window lifecycle, installers, launching, Gamescope/fullscreen tools, Windows icon extraction, uninstall, Steam sync | High | Thin Tauri adapter plus separate core services |
| `runtime.rs` | Paths, Wine/Proton/emulator download and install, prefix creation, runner selection, progress and logs | High through `AppHandle` and `Emitter` | `ardali-runners` plus an event sink interface |
| `database.rs` | Schema, migrations, games, sessions, settings, metadata, game-specific defaults | Low but imports runtime and Steam models | `ardali-storage` using core-owned models and injected paths |
| `steam.rs` | Local Steam roots, VDF/ACF scanning, Proton discovery | Low; imports runtime only for ID sanitizing | `ardali-steam` depending only on core utilities |
| `updates.rs` | Component inventory, version files, install/remove orchestration | High through runtime and `AppHandle` | Runner update service with event sink and downloader interfaces |
| `compatibility.rs` | Per-game settings/logs, Wine overrides, ProtonDB request | Low; imports database/runtime models and paths | Core compatibility service with HTTP/path adapters |
| `metadata.rs` | SteamGridDB requests, cover download, database update | Medium through database/runtime | Metadata service with repository and HTTP interfaces |
| `main.rs` | Starts the Tauri library | High by design | Remains in the Tauri app |

## Important user flows to preserve

These flows form the regression checklist during extraction:

1. Initialize runtime directories and SQLite.
2. Install, detect, update, remove, and cancel downloads for portable components.
3. Create isolated or shared Wine prefixes.
4. Preview and select EXE/MSI files, run installers, and discover installed executables.
5. Add, list, edit, launch, monitor, uninstall, and remove games/applications.
6. Track active sessions, last-played time, and total playtime.
7. Apply Wine version, DLL override, virtual desktop, DXVK, Gamescope, and display settings.
8. Discover OpenRA, DOSBox-X, CnCNet, Wine, Proton, DXVK, and VKD3D.
9. Scan local Steam libraries and installed Proton versions, then import Steam games.
10. Fetch covers and ProtonDB summaries and preserve manual metadata.
11. Emit backend logs, download progress, library changes, install progress, and game-ended events.
12. Open and close the game settings window and preserve existing window behavior.

## Persistent data contract

Migration must preserve these locations and formats:

- `games.db`: game library, play sessions, application settings, and metadata.
- `wine/`: portable Wine, Proton, and graphics components.
- `prefixes/`: isolated Wine prefixes.
- `games/`: managed game installation directories.
- `emulators/`: OpenRA, DOSBox-X, and CnCNet files.
- `compatibility/<game-id>/`: settings and troubleshooting logs.
- `covers/`: downloaded and manually selected cover references.
- `updates/*.version`: installed component version markers.
- `logs/backend.log`: persistent backend log.

Database changes must remain forward-only and compatible with existing user data. Before changing schema ownership, add a database backup/restore test fixture.

## Current dependency directions that must be corrected

Desired dependency direction:

```text
ardali-tauri  --->  ardali-core  <---  ardali-cli
                         ^
                         |
        +----------------+----------------+
        |                |                |
 ardali-storage   ardali-runners    ardali-steam
```

Current issues:

- Core operations accept `tauri::AppHandle` in `runtime.rs`, `updates.rs`, and much of `lib.rs`.
- Core operations emit Tauri event names directly.
- Database models import runner and Steam types instead of depending on neutral core models.
- Runtime paths are discovered globally from environment variables, which makes tests and alternate frontends harder.
- Network calls shell out to `curl`, preventing controlled HTTP testing and cancellation.
- Launching, UI window management, package-manager scripts, file dialogs, icon parsing, and persistence coexist in `lib.rs`.
- Many errors are unstructured `String` values, making UI-specific translation and recovery difficult.

## Target interfaces

The extraction should introduce small UI-neutral contracts before moving files:

```rust
pub trait EventSink: Send + Sync {
    fn publish(&self, event: ArdaliEvent);
}

pub trait GameRepository {
    fn list_games(&self) -> Result<Vec<Game>, ArdaliError>;
    fn find_game(&self, id: GameId) -> Result<Game, ArdaliError>;
}

pub trait RunnerService {
    fn discover(&self) -> Result<Vec<Runner>, ArdaliError>;
    fn launch(&self, request: LaunchRequest) -> Result<LaunchHandle, ArdaliError>;
}
```

Initial event model:

- `Log`
- `DownloadProgress`
- `LibraryChanged`
- `InstallProgress`
- `GameStarted`
- `GameEnded`

Tauri will implement `EventSink` by converting these events to current frontend event names. CLI will print them. GTK4 can later map them to native signals without changing the core.

## Risk register

| Risk | Impact | Control |
| --- | --- | --- |
| Existing SQLite data becomes unreadable | Critical | Preserve paths/schema, add fixture backup and migration tests first |
| Game launch behavior changes during moves | High | Add command-building unit tests before extraction |
| Progress/events disappear | High | Introduce `EventSink` adapter before moving runtime code |
| Wine prefixes are modified accidentally | High | Use temporary directories in tests; never point tests at user data |
| Long-running process monitoring regresses | High | Isolate launch/session service and test state transitions |
| Tauri frontend command names change | Medium | Keep current commands as a compatibility facade |
| GTK experiment expands into a rewrite | Medium | Limit it to list, scan, launch, and progress proof of concept |
| System package scripts run during tests | High | Put system commands behind an injectable command runner |

## Guardrails for all following steps

1. Keep Tauri as the production UI until the comparison checkpoint.
2. Preserve all existing command names while extraction is in progress.
3. Do not migrate or delete user data as part of structural refactoring.
4. Do not run Wine, installers, package-manager commands, or destructive filesystem operations in automated tests.
5. Add characterization tests before moving behavior.
6. Keep each change buildable and reversible.
7. GTK4 and Slint must consume public core APIs; core crates must never import either toolkit.

## Ordered extraction plan

### Step 2A: Establish a Cargo workspace

- Keep `src-tauri` working while adding `crates/ardali-core`.
- Move only neutral identifiers, request/response models, errors, and events first.
- Add core serialization tests.

### Step 2B: Add ports and adapters

- Add `EventSink`, clock, paths, command runner, HTTP client, and repository interfaces.
- Implement Tauri event and production process adapters without changing frontend behavior.

### Step 3: Extract storage

- Create `ardali-storage`.
- Move SQLite schema and operations behind repository interfaces.
- Add temporary-database migration and CRUD tests.

### Step 4: Extract runner services

- Create `ardali-runners`.
- Move discovery, command construction, prefix, download, update, and launch behavior in small slices.
- Add filesystem and command-construction tests.

### Step 5: Extract Steam

- Create `ardali-steam`.
- Move VDF parsing, local discovery, Proton discovery, and Steam launch requests.
- Add fixture-based parser tests.

### Step 6: Validate alternate entry points

- Add `ardali-cli` for list, runner discovery, Steam scan, and launch.
- Add a deliberately small GTK4 proof of concept after the CLI proves portability.
- Evaluate Slint only at the UI comparison checkpoint; do not couple it to core crates.

## Step 1 exit criteria

- [x] Repository and module inventory recorded.
- [x] Tauri coupling measured and hotspots identified.
- [x] Persistent data and critical user flows documented.
- [x] Risks and refactoring guardrails documented.
- [x] Ordered extraction path defined.
- [x] Initial serialization contract tests exist in `ardali-core`.

Step 2A established the workspace and a minimal `ardali-core` crate containing UI-neutral models, error types, events, and serialization contract tests while leaving existing runtime behavior in place.

Step 2B introduced `TauriEventSink`. Runtime and update services now publish UI-neutral core events without importing Tauri, while the adapter preserves the existing frontend event names and payloads. Runtime path injection remains the next adapter slice before storage extraction.

Step 3A introduced `ardali-storage` and moved SQLite connection setup, base schema creation, column migrations, and default-setting seeds out of the Tauri package. Temporary and in-memory database tests cover new schema creation, legacy migration without record loss, and idempotent initialization. Game repositories and CRUD operations remain the next storage slice.

Step 3B.1 moved play-session persistence and application-setting CRUD into `ardali-storage`. The existing Tauri database functions remain as compatibility adapters, so frontend command names and payloads are unchanged. In-memory repository tests cover session start/finish, detached-session cleanup, and setting upserts. Game records and metadata CRUD remain the next storage slice.

Step 3B.2 moved game insert/list/read/update/delete, Steam game upsert, metadata persistence, mode updates, and CnCNet state updates into `ardali-storage`. The Tauri package no longer depends directly on `rusqlite`; its database module is now a compatibility adapter that adds application-level defaults and filesystem-derived presentation data. In-memory tests cover game CRUD, metadata and settings updates, and idempotent Steam synchronization. Storage extraction is complete; runner-service extraction is the next architectural step.
