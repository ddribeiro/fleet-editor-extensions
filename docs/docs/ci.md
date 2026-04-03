---
icon: lucide/git-branch
---

# CI/CD integration

## GitHub Actions

```yaml
name: Lint Fleet GitOps
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install flint
        run: |
          curl -fsSL https://raw.githubusercontent.com/headmin/fleet-editor-extensions/main/scripts/install.sh | sh

      - name: Lint
        run: flint check . --format json
```

## Pre-commit hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/headmin/fleet-editor-extensions
    rev: v0.1.1
    hooks:
      - id: flint-check
```

## GitLab CI

```yaml
lint:
  image: ubuntu:latest
  script:
    - curl -fsSL https://raw.githubusercontent.com/headmin/fleet-editor-extensions/main/scripts/install.sh | sh
    - flint check . --format json
```

## JSON output

Use `--format json` in CI for machine-readable output:

```bash
flint check . --format json
```

```json
{
  "version": "0.1.1",
  "files": [...],
  "summary": {
    "files_linted": 121,
    "errors": 0,
    "warnings": 24,
    "infos": 54
  }
}
```

Exit code `1` when errors are found, `0` otherwise. Warnings and infos do not cause failure.

## Dev containers

A [`.devcontainer`](https://github.com/headmin/fleet-editor-extensions/tree/main/.devcontainer) config is included for GitHub Codespaces. It auto-installs flint, initializes `.fleetlint.toml`, and configures the VS Code extension.
