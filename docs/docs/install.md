---
icon: lucide/download
---

# Installation

## macOS (recommended: PKG installer)

Download `flint-x.y.z.pkg` from [GitHub Releases](https://github.com/headmin/fleet-editor-extensions/releases). Double-click to install. The binary is code-signed and notarized by Apple.

Installs to `/usr/local/bin/flint`.

## macOS / Linux (script)

```bash
curl -fsSL https://raw.githubusercontent.com/headmin/fleet-editor-extensions/main/scripts/install.sh | sh
```

The script auto-detects your platform (darwin/linux, arm64/x64), downloads the latest release, and installs to `/usr/local/bin`.

### Options

| Variable | Default | Description |
|---|---|---|
| `FLINT_VERSION` | latest | Pin to a specific version |
| `FLINT_INSTALL_DIR` | `/usr/local/bin` | Custom install location |

```bash
# Pin version
FLINT_VERSION=0.1.1 curl -fsSL .../install.sh | sh

# Install to home directory (no sudo)
FLINT_INSTALL_DIR=$HOME/.local/bin curl -fsSL .../install.sh | sh
```

## macOS / Linux (manual)

Download the archive for your platform from [releases](https://github.com/headmin/fleet-editor-extensions/releases):

| Platform | Asset |
|---|---|
| macOS (Apple Silicon) | `flint-x.y.z-darwin-arm64.tar.gz` |
| Linux x64 | `flint-x.y.z-linux-x64.tar.gz` |
| Linux ARM64 | `flint-x.y.z-linux-arm64.tar.gz` |

macOS Intel (x86_64) is not supported.

```bash
tar xzf flint-*.tar.gz
sudo mv flint /usr/local/bin/
```

## Build from source

```bash
git clone https://github.com/headmin/fleet-editor-extensions
cd fleet-editor-extensions
cargo build --release -p flint
sudo cp target/release/flint /usr/local/bin/
```

Requires Rust 1.81+ (`rustup update stable`).

## Dev container

A [`.devcontainer`](https://github.com/headmin/fleet-editor-extensions/tree/main/.devcontainer) config is included for GitHub Codespaces. It auto-installs flint and initializes `.fleetlint.toml`.

## Verify installation

```bash
flint --version
# flint 0.1.1+20260403.0510 (Fleet sync: ...)
```
