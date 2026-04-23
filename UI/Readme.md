# cscs-key UI

This folder contains a small desktop-style UI for the existing `cscs-key` CLI.

It runs on macOS and Linux, opens in its own native window, and provides forms for the main CLI workflows:

- Generate a new SSH key pair
- Sign an existing SSH key
- List certificates
- Revoke certificates

The UI does not reimplement CSCS authentication or the SSH API. It starts a local web server, renders that UI inside a native webview window, and executes the existing `cscs-key` binary underneath. The normal browser-based OIDC flow still applies when `cscs-key` needs authentication.

## Prerequisites

- Rust and Cargo
- A built `cscs-key` binary, or `cscs-key` available on your `PATH`

Linux note: the desktop shell depends on the system webview stack used by `wry`/`tao`, so you need the usual WebKitGTK runtime and development packages installed for your distribution.

macOS note: the packaged `.app` bundle includes the `cscs-key` CLI inside the app resources, so the desktop UI can run as a self-contained release artifact.

The UI looks for the CLI in this order:

1. `--bin /path/to/cscs-key`
2. `CSCS_KEY_BIN=/path/to/cscs-key`
3. `../target/release/cscs-key`
4. `../target/debug/cscs-key`
5. `cscs-key` from `PATH`

## Run

From the repository root:

```bash
cargo run --manifest-path UI/Cargo.toml
```

That starts the local server and opens the UI in a native desktop window.

To use a normal browser tab instead of the desktop shell:

```bash
cargo run --manifest-path UI/Cargo.toml -- --browser
```

To run only the local server without opening a window:

```bash
cargo run --manifest-path UI/Cargo.toml -- --headless
```

To force a specific CLI binary:

```bash
cargo run --manifest-path UI/Cargo.toml -- --bin /absolute/path/to/cscs-key
```

You can also choose a fixed port if needed:

```bash
cargo run --manifest-path UI/Cargo.toml -- --port 8789
```

## Build

```bash
cargo build --release --manifest-path UI/Cargo.toml
```

The generated UI binary will be at `UI/target/release/cscs-key-ui`.

## Package For macOS

Build a release `.app` bundle and zip archive:

```bash
./UI/scripts/package-macos.sh
```

This creates:

- `UI/dist/macos/cscs-key-ui-macos-v<version>.zip`

Unzip the archive to get `CSCS Key.app`.

The macOS packager also:

- Builds the root `cscs-key` CLI and bundles it into the app
- Generates the app icon set and `.icns` bundle icon
- Writes the bundle `Info.plist`
- Applies code signing automatically

### Signing Modes

The packager supports four signing modes through `MACOS_SIGNING_MODE`:

- `auto`: use `Developer ID Application` if available, otherwise fall back to ad-hoc signing
- `developer-id`: require a `Developer ID Application` certificate
- `adhoc`: use ad-hoc signing only
- `none`: skip signing entirely

Examples:

```bash
MACOS_SIGNING_MODE=developer-id ./UI/scripts/package-macos.sh
```

```bash
MACOS_SIGNING_MODE=developer-id \
MACOS_CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
./UI/scripts/package-macos.sh
```

When `MACOS_CODESIGN_IDENTITY` is not set, the script looks for the first installed certificate whose name starts with `Developer ID Application:`.

Developer ID signing uses:

- Hardened runtime via `codesign --options runtime`
- Timestamped signatures via `codesign --timestamp`

### Notarization

If notarization credentials are available, the packager can also notarize and staple the app before producing the final zip archive.

Preferred option: use a saved `notarytool` keychain profile.

```bash
xcrun notarytool store-credentials cscs-key-notary \
	--apple-id "your-apple-id@example.com" \
	--team-id "TEAMID1234" \
	--password "app-specific-password"
```

Then run:

```bash
MACOS_SIGNING_MODE=developer-id \
MACOS_NOTARYTOOL_PROFILE=cscs-key-notary \
./UI/scripts/package-macos.sh
```

Alternative environment variables are also supported:

- `APPLE_ID`
- `APPLE_TEAM_ID`
- `APPLE_APP_SPECIFIC_PASSWORD`

If Developer ID signing is used without notarization credentials, the script still produces a Developer ID signed zip, but it will not submit the archive to Apple notarization.

## Notes

- The UI is intended as a thin browser wrapper around the CLI, not a separate CSCS client.
- The default launch mode is a native desktop shell around the local web UI, similar to a lightweight Electron app.
- If a command needs login, complete the authentication flow in the browser window opened by `cscs-key`.
- The list view shows the CLI output exactly as returned by `cscs-key`.

