# CSCS-key

CSCS-key is a command-line tool to manage SSH keys for the Swiss National Supercomputing Centre (CSCS). It allows users to add, remove, and list SSH keys associated with their CSCS account.

## Installation

Download the latest release from the [GitHub repository](0https://github.com/cscs/cscs-key/releases) and unpack the archive. Move the `cscs-key` executable to a directory in your PATH.
```bash
tar -zxf cscs-key-<version>.tar.gz
```

TODO brew, pip, ...

# Build from source

Prerequisites:
- Rust
- Cargo

You can install the prerequisites, e.g., using Homebrew on macOS:
```bash
brew install rust
```

Clone the repository and build the project:
```bash
git clone TODO URL
cd cscs-key
cargo build --release
```

## Docker

Build the Docker image:
```bash
docker build -t cscs-key .
```

If you also want to extract the compiled `target/` artifacts from the builder stage to your host:
```bash
docker build --target builder -t cscs-key-builder .
docker create --name cscs-key-build cscs-key-builder
docker cp cscs-key-build:/app/target ./target
docker rm cscs-key-build
```

Run the tool from Docker instead of `target/release/cscs-key`:
```bash
docker run --rm -it \
  -p 8765:8765 \
  -v "$HOME/.ssh:/home/appuser/.ssh" \
  -v "$HOME/.config/cscs-key:/home/appuser/.config/cscs-key" \
  -v "$HOME/.cache/cscs-key:/home/appuser/.cache/cscs-key" \
  cscs-key sign
```

For CI or service-account usage, pass the API key explicitly:
```bash
docker run --rm \
  -e CSCS_API_KEY="$CSCS_API_KEY" \
  -v "$HOME/.ssh:/home/appuser/.ssh" \
  cscs-key sign
```

## Usage

### Sign ssh key

To sign an SSH key, use the following command:
```bash
cscs-key sign
```
The default key is `~/.ssh/cscs-key`. You can specify a different private key using the `-f, --file` option.

The default duration of the signed key is 1 day. You can specify a different duration using the `-d, --duration` option. Possible values are `1d` or `1min`.

### Generate ssh key on the server (deprecated)

Generating the ssh key on the server is deprecated and will be removed in the future. It is recommended to generate the SSH key locally and then sign it using the `cscs-key sign` command.

To generate a new SSH key on the server, use the following command:
```bash
cscs-key gen
```
The default key is `~/.ssh/cscs-key`. You can specify a different private key using the `-f, --file` option.

The default duration of the signed key is 1 day. You can specify a different duration using the `-d, --duration` option. Possible values are `1d` or `1min`.

### List ssh keys

To list all valid SSH keys associated with your CSCS account, use the following command:
```bash
cscs-key list
```
Or with `-a, --all` to also show expired and revoked keys:
```bash
cscs-key list -a
```

### Revoke ssh key

To revoke one or more SSH key, use the following command:
```bash
cscs-key revoke <key_id> ...
```
Or to revoke all keys, use the `-a, --all` option:
```bash
cscs-key revoke -a
```

### Generate completion script
To generate a completion script for your shell, use the following command:
```bash
cscs-key completion <shell>
```
Replace `<shell>` with your shell. Possible values: `bash`, `zsh`, `fish`, `powershell`, or `elvish`.

To automatically activate the completion when you start a new shell session, you can add the following line to your shell configuration file (e.g., `~/.bashrc` for bash):
```bash
# Enable cscs-key completion
source <(cscs-key completion bash)
```

## Authentication

Users authenticate using the Open ID Connect (OIDC) protocol. The tool opens a web browser where the user authenticates with the CSCS credentials. After successful authentication, an access token is stored locally. This way users only need to authenticate about once per day.

Service accounts used for example in CI/CD pipelines can authenticate using an API key. Export the API key as an environment variable `CSCS_API_KEY`:
```bash
export CSCS_API_KEY=<your_api_key>
```
Pro tip: Use pipeline variables to securely store the API key in your CI/CD setup.
