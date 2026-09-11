#!/usr/bin/env bash
# ==============================================================================
#  ⏱️ Server Chronicle — Remote One-Liner Bootstrapper
# ==============================================================================
set -euo pipefail
IFS=$'\n\t'

REPO="Praveensenpai/server-chronicle"
BINARY="server-chronicle"
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "🌸 ========================================= 🌸"
echo "        ⏱️ Server Chronicle Bootstrapper       "
echo "🌸 ========================================= 🌸"

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
    aarch64|arm64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "❌ Architecture $ARCH is not supported."
        exit 1
        ;;
esac

TAG=$(curl -4 -sSL -H "Cache-Control: no-cache" -H "Pragma: no-cache" "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)

if [ -n "$TAG" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/server-chronicle-${TARGET}.tar.gz"
    echo "==> Downloading pre-compiled server-chronicle (${TARGET} - $TAG)..."
    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    if curl -4 -fsSL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR" 2>/dev/null; then
        install -m 755 "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
        echo "✔ Installed server-chronicle binary to $INSTALL_DIR/$BINARY"
    else
        echo "⚠️  Failed to download release binary. Falling back to local build..."
        TAG=""
    fi
fi

if [ -z "${TAG:-}" ]; then
    TARGET_DIR="$HOME/server-chronicle"
    if [ -d "$TARGET_DIR/.git" ]; then
        git -C "$TARGET_DIR" pull --ff-only origin main || true
    else
        git clone "https://github.com/$REPO.git" "$TARGET_DIR"
    fi
    cd "$TARGET_DIR"
    cargo build --release
    install -m 755 target/release/$BINARY "$INSTALL_DIR/$BINARY"
fi

"$INSTALL_DIR/$BINARY" install-service
echo "✨ Server Chronicle setup complete! Launch anytime with: server-chronicle status"
