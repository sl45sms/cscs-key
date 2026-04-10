#!/bin/sh
set -eu

key_path="${CSCS_KEY_PATH:-$HOME/.ssh/cscs-key}"

if [ "${1:-}" = "sign" ]; then
  expect_path_value=0
  for arg in "$@"; do
    if [ "$expect_path_value" -eq 1 ]; then
      key_path="$arg"
      expect_path_value=0
      continue
    fi

    case "$arg" in
      -f|--file)
        expect_path_value=1
        ;;
      --file=*)
        key_path="${arg#--file=}"
        ;;
    esac
  done

  if [ -f "$key_path" ] && [ ! -f "${key_path}.pub" ]; then
    echo "Generating missing public key ${key_path}.pub from existing private key..."
    ssh-keygen -y -f "$key_path" > "${key_path}.pub"
    chmod 644 "${key_path}.pub" || true
  fi
fi

exec /usr/local/bin/cscs-key "$@"
