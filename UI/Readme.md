# cscs-key UI

This folder contains a small local web UI for the existing `cscs-key` CLI.

It runs on macOS and Linux, opens in your browser, and provides forms for the main CLI workflows:

- Generate a new SSH key pair
- Sign an existing SSH key
- List certificates
- Revoke certificates

The UI does not reimplement CSCS authentication or the SSH API. It starts a local web server and executes the existing `cscs-key` binary underneath, so the normal browser-based OIDC flow still applies.

## Prerequisites

- Rust and Cargo
- A built `cscs-key` binary, or `cscs-key` available on your `PATH`

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

To keep the UI from opening the browser automatically:

```bash
cargo run --manifest-path UI/Cargo.toml -- --no-open-browser
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

## Notes

- The UI is intended as a thin browser wrapper around the CLI, not a separate CSCS client.
- If a command needs login, complete the authentication flow in the browser window opened by `cscs-key`.
- The list view shows the CLI output exactly as returned by `cscs-key`.

