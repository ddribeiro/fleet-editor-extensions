---
icon: lucide/rocket
---

# Getting started

## 1. Initialize configuration

Run `flint init` in your Fleet GitOps repo root:

```bash
cd /path/to/your/fleet-gitops-repo
flint init
```

This auto-detects your directory structure and creates `.fleetlint.toml` with sensible defaults.

## 2. Lint your repo

```bash
flint check .
```

Flint scans all YAML files and reports errors, warnings, and info:

```
🔍 Linting directory .

File: fleets/engineering.yml
warning: 'update_new_hosts' expects a boolean value, got null
  --> fleets/engineering.yml:21:22
  help: Use 'true' or 'false'

Summary: Linted 121 file(s)
  0 error(s)
  24 warning(s)
  54 info
```

## 3. Auto-fix

```bash
# Fix safe issues (key renames, typo corrections)
flint check . --fix

# Also apply risky fixes
flint check . --fix --unsafe-fixes
```

## 4. JSON output (for CI)

```bash
flint check . --format json
```

Returns structured JSON with diagnostics per file — exit code 1 on errors, 0 on success.

## 5. Set up your editor

Install the flint extension for your editor to get real-time diagnostics, completions, and hover docs. See [Editors](editors.md).

## 6. Agent integration

```bash
flint setup-agent       # Install Claude Code skills
flint help-ai           # Command reference for agents
flint help-ai --sop lint     # Step-by-step linting SOP
flint help-ai --sop migrate  # Migration SOP
```
