# Steam account and API security

## Decision for Step 3C

ArDali reads the local Steam installation, `libraryfolders.vdf`, and
`appmanifest_*.acf` files. This workflow does not require a Steam account,
password, session cookie, Steam Web API key, or Steam Guard code.

ArDali must never ask the user to enter a Steam password or Steam Guard code.
The Steam client remains responsible for account authentication and ownership
checks. Launching a Steam game will be delegated to the installed Steam client.

## Optional online integrations

- Steam Web API features are out of scope until a feature actually requires
  them. If introduced, they must be opt-in and use the smallest required data
  scope.
- A future account sign-in must use Steam's browser-based authentication flow.
  ArDali must not embed, intercept, or persist Steam credentials or cookies.
- SteamGridDB is a separate third-party service. Its API key is not a Steam API
  key and must never be sent to Valve or written to application logs.

## Secret storage boundary

The existing SteamGridDB key is stored locally in the application SQLite
database for backward compatibility. Backend settings responses redact its
value, expose only whether it is configured, and accept settings only from a
fixed allowlist. The UI uses a password input and never repopulates the key.

Before distributing builds that depend on online API keys, secret persistence
should move to the Linux Secret Service/keyring with an explicit migration from
the legacy SQLite value. If no keyring is available, the application should
explain that limitation and require explicit user consent before local fallback.

## Logging and diagnostics

- Never include API keys, authorization headers, cookies, or credentials in
  frontend/backend logs and troubleshooting reports.
- Errors may identify the service and HTTP status, but not request headers.
- Account and online metadata features must remain optional; local library
  scanning and launching must continue to work without them.
