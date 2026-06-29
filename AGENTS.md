# AGENTS.md

## Purpose
This file helps AI coding agents understand the current workspace and behave safely when the repository has little or no source code.

## Current workspace state
- Root contains only:
  - `iconc/IMG_20260527_012142.png`
  - `.vscode/settings.json`
- No source files, build scripts, or project metadata were found.
- No existing `README.md`, `package.json`, or other typical project configuration files exist.

## Agent guidance
1. Confirm the repository contents before making changes.
   - Ask the user for the intended language, framework, or project goal if the workspace is empty.
   - Do not assume this is a specific type of project.
2. Avoid creating unrelated code scaffolding unless the user explicitly requests it.
   - If the user asks to initialize a project, request more detail first.
3. Preserve existing files and directories.
   - Do not remove or overwrite `.vscode/settings.json` or `iconc/IMG_20260527_012142.png` unless asked.
4. If new source files are added, inspect the workspace again for build/test conventions.
   - Look for `package.json`, `pyproject.toml`, `go.mod`, `Cargo.toml`, `build.gradle`, etc.

## Notes for future updates
- If the repository gains source code, this file should be updated with project-specific instructions and conventions.
- Prefer `AGENTS.md` in the repo root for workspace-level guidance.
