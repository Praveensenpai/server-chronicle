#!/usr/bin/env bash
# ==============================================================================
#  ⏱️ Server Chronicle — Install Script
# ==============================================================================
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

echo "==> Building server-chronicle (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "==> Installing binary to $BIN_DIR/server-chronicle..."
install -m 755 "$SCRIPT_DIR/target/release/server-chronicle" "$BIN_DIR/server-chronicle"

echo "==> Enabling background user systemd service..."
"$BIN_DIR/server-chronicle" install-service

echo "✔ server-chronicle successfully installed and running!"
