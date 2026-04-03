#!/bin/bash
set -e

# Install flint from GitHub releases (or build from source if in dev)
if [ -f "Cargo.toml" ] && grep -q "flint" Cargo.toml 2>/dev/null; then
  echo "Building flint from source..."
  cargo build --release -p flint 2>/dev/null || cargo build --release 2>/dev/null
  sudo cp target/release/flint /usr/local/bin/ 2>/dev/null || \
  sudo cp target/release/fleet-schema-gen /usr/local/bin/flint 2>/dev/null || true
else
  echo "Installing flint from GitHub releases..."
  ARCH=$(uname -m)
  case "$ARCH" in
    x86_64)  TARGET="linux-x64" ;;
    aarch64) TARGET="linux-arm64" ;;
    arm64)   TARGET="linux-arm64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
  esac

  curl -fsSL "https://github.com/fleetdm/fleet-editor-extensions/releases/latest/download/flint-${TARGET}.tar.gz" \
    -o /tmp/flint.tar.gz && \
    tar -xzf /tmp/flint.tar.gz -C /usr/local/bin/ && \
    chmod +x /usr/local/bin/flint && \
    rm /tmp/flint.tar.gz
fi

# Initialize .fleetlint.toml if not present
if [ ! -f ".fleetlint.toml" ]; then
  echo "Initializing .fleetlint.toml..."
  flint init --no-interactive 2>/dev/null || true
fi

echo "Flint setup complete."
flint --version 2>/dev/null || echo "(flint binary not yet available)"
