# Fleet GitOps VS Code Extension

VS Code extension for Fleet GitOps YAML validation, completions, and diagnostics.

## Features

- **Get Started**: Scaffold a new GitOps repo with `Fleet: Get Started` from the command palette
- **Validation**: Real-time diagnostics for Fleet configuration errors
- **Completions**: Context-aware autocompletion for field names and values
- **File Path Completions**: Suggests files when typing `path:` values
- **Hover Documentation**: Shows documentation on hover for fields and osquery tables
- **Go-to-Definition**: Navigate to referenced files
- **Semantic Highlighting**: Syntax highlighting for osquery SQL in YAML
- **Deprecation Warnings**: Strikethrough on deprecated keys with quick-fix renames

## Installation

1. Download `fleet-gitops-<version>.vsix` from [GitHub Releases](https://github.com/headmin/fleet-editor-extensions/releases)

2. Install via command line:
   ```bash
   code --install-extension fleet-gitops-0.1.1.vsix
   ```

   Or in VS Code:
   - Open Extensions (`Cmd+Shift+X`)
   - Click `...` menu → "Install from VSIX..."
   - Select the downloaded `.vsix` file

3. Reload VS Code

The VSIX includes the bundled LSP binary — no additional installation needed.

## File Patterns

The extension activates for YAML files matching Fleet GitOps patterns:
- `default.yml` / `default.yaml`
- `fleets/**/*.yml` / `fleets/**/*.yaml`
- `teams/**/*.yml` / `teams/**/*.yaml` (legacy, still supported)
- `lib/**/*.yml` / `lib/**/*.yaml`

## Configuration

Open VS Code settings (`Cmd+,`) and search for "Fleet":

| Setting | Description | Default |
|---------|-------------|---------|
| `fleetGitops.enable` | Enable/disable the extension | `true` |
| `fleetGitops.serverPath` | Custom path to LSP binary | (bundled) |
| `fleetGitops.fleetVersion` | Fleet version for schema validation | `latest` |
| `fleetGitops.trace.server` | Debug: log raw LSP traffic to output channel | `off` |

## Commands

Open the command palette (`Cmd+Shift+P`) and type "Fleet":

| Command | Description |
|---------|-------------|
| Fleet: Get Started | Scaffold a new GitOps repository |
| Fleet: Open default.yml | Open (or create) the global config |
| Fleet: Restart Language Server | Restart the LSP server |
| Fleet: Show Output Channel | Show the LSP output log |

## Troubleshooting

### Extension not activating

1. Check Output panel: `View > Output` → select "Fleet GitOps"
2. Verify file matches activation patterns
3. Reload: `Cmd+Shift+P` → "Developer: Reload Window"

### Debug logging

Enable verbose logging in settings:
```json
{
    "fleetGitops.trace.server": "verbose"
}
```
