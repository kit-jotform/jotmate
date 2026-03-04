#!/bin/sh
# jotmate installer
# Usage: curl -fsSL https://raw.githubusercontent.com/kit-jotform/Jotmate/main/install.sh | sh
# Or: ./install.sh [--prefix /usr/local]

set -e

REPO="kit-jotform/Jotmate"
BINARY="jotmate"
INSTALL_DIR="${HOME}/.local/bin"

# Parse args
while [ $# -gt 0 ]; do
    case "$1" in
        --prefix)
            INSTALL_DIR="$2/bin"
            shift 2
            ;;
        --prefix=*)
            INSTALL_DIR="${1#--prefix=}/bin"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--prefix /usr/local]"
            exit 1
            ;;
    esac
done

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64)  TARGET="aarch64-apple-darwin" ;;
            x86_64) TARGET="x86_64-apple-darwin" ;;
            *)
                echo "Unsupported architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64)         TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64|arm64)  TARGET="aarch64-unknown-linux-gnu" ;;
            *)
                echo "Unsupported architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Get latest release version
echo "Fetching latest release..."
if command -v curl >/dev/null 2>&1; then
    LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
elif command -v wget >/dev/null 2>&1; then
    LATEST=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
else
    echo "Error: curl or wget is required"
    exit 1
fi

if [ -z "$LATEST" ]; then
    echo "Error: Could not determine latest release version"
    exit 1
fi

ASSET="${BINARY}-${LATEST}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}"

echo "Downloading ${BINARY} ${LATEST} for ${TARGET}..."

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMPDIR/${ASSET}"
else
    wget -qO "$TMPDIR/${ASSET}" "$URL"
fi

echo "Extracting..."
tar -xzf "$TMPDIR/${ASSET}" -C "$TMPDIR"

# Create install dir if needed
mkdir -p "$INSTALL_DIR"

# Install binary
mv "$TMPDIR/${BINARY}" "$INSTALL_DIR/${BINARY}"
chmod +x "$INSTALL_DIR/${BINARY}"

echo ""
echo "✓ jotmate ${LATEST} installed to ${INSTALL_DIR}/${BINARY}"

# Check if install dir is on PATH
case ":$PATH:" in
    *":${INSTALL_DIR}:"*)
        echo "✓ ${INSTALL_DIR} is already on your PATH"
        ;;
    *)
        echo ""
        echo "⚠  ${INSTALL_DIR} is not on your PATH."
        echo "   Add the following to your shell profile (~/.zshrc, ~/.bashrc, etc.):"
        echo ""
        echo "     export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        ;;
esac
