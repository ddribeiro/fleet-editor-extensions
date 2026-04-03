//! Generate machine-readable CLI reference for AI agents.
//!
//! Progressive discovery modes:
//! - **Index** (default): Agent guide + command index
//! - **Command**: Full detail for a single command by dotted path
//! - **Full**: Complete CLI reference (all commands, all flags)
//! - **SOP**: Step-by-step standard operating procedures
//! - **JSON**: Full CLI schema as JSON

use std::fmt::Write as _;
use std::io::Write;

use anyhow::{bail, Result};

/// Built-in subcommands to skip in output.
const SKIP_SUBCOMMANDS: &[&str] = &["help"];

// ── Index mode (default) ─────────────────────────────────────────────

/// Generate the agent guide and command index.
pub fn generate_index(cmd: &clap::Command, writer: &mut impl Write) -> Result<()> {
    let mut buf = String::with_capacity(4 * 1024);
    let name = cmd.get_name();

    writeln!(
        buf,
        "# {name} — Fleet GitOps YAML linter and language server"
    )?;
    writeln!(buf)?;
    writeln!(buf, "## Agent guide")?;
    writeln!(buf)?;
    writeln!(
        buf,
        "{name} is a CLI tool for linting, validating, and migrating Fleet GitOps YAML configurations."
    )?;
    writeln!(buf)?;
    writeln!(buf, "**Discovery workflow:**")?;
    writeln!(
        buf,
        "1. Read the command index below to find relevant commands"
    )?;
    writeln!(
        buf,
        "2. Run `{name} help-ai --command <name>` for full flags and usage of a specific command"
    )?;
    writeln!(
        buf,
        "3. Run `{name} help-ai --sop <tool>` for step-by-step workflows (lint, migrate, lsp)"
    )?;
    writeln!(
        buf,
        "4. Run `{name} help-ai --full` for the complete reference (large output)"
    )?;
    writeln!(buf)?;
    writeln!(buf, "**JSON schema (for structured parsing):**")?;
    writeln!(buf, "- `{name} help-json` — full CLI schema as JSON")?;
    writeln!(
        buf,
        "- `{name} help-json <name>` — scoped subtree, globals stripped"
    )?;
    writeln!(buf)?;
    writeln!(buf, "**Common patterns:**")?;
    writeln!(
        buf,
        "- `--format json` on check and list-rules for structured output"
    )?;
    writeln!(
        buf,
        "- `--fix` auto-applies safe fixes, `--unsafe-fixes` for risky ones"
    )?;
    writeln!(
        buf,
        "- `{name} migrate` outputs JSON report — does NOT apply changes"
    )?;
    writeln!(
        buf,
        "- `.fleetlint.toml` configures rules, thresholds, and Fleet connection"
    )?;
    writeln!(buf)?;
    writeln!(
        buf,
        "**When to use which SOP (match user intent to the right SOP):**"
    )?;
    writeln!(
        buf,
        "- lint, validate, check, fix YAML files → `--sop lint`"
    )?;
    writeln!(
        buf,
        "- migrate, upgrade, rename teams/queries/team_settings → `--sop migrate`"
    )?;
    writeln!(
        buf,
        "- editor setup, VS Code, Neovim, Zed, Sublime, JetBrains → `--sop lsp`"
    )?;
    writeln!(buf)?;
    writeln!(
        buf,
        "**SOPs:** Run `{name} help-ai --sop <tool>` for step-by-step workflows:"
    )?;
    writeln!(
        buf,
        "- `--sop lint` — linting workflow (init → check → fix → json output)"
    )?;
    writeln!(
        buf,
        "- `--sop migrate` — version migration (report → rename → verify)"
    )?;
    writeln!(
        buf,
        "- `--sop lsp` — editor setup guide for all supported editors"
    )?;
    writeln!(buf)?;

    // Command index
    writeln!(buf, "## Command index")?;
    writeln!(buf)?;

    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || SKIP_SUBCOMMANDS.contains(&sub.get_name()) {
            continue;
        }
        let about = sub.get_about().map(|a| a.to_string()).unwrap_or_default();
        writeln!(buf, "### {name} {}", sub.get_name())?;
        writeln!(buf, "{about}")?;

        // List args briefly
        for arg in sub.get_arguments() {
            if arg.is_hide_set() || arg.get_id() == "help" || arg.get_id() == "version" {
                continue;
            }
            let flag = if let Some(long) = arg.get_long() {
                format!("--{long}")
            } else if arg.is_positional() {
                format!("<{}>", arg.get_id())
            } else {
                continue;
            };
            let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
            let req = if arg.is_required_set() {
                " (required)"
            } else {
                ""
            };
            writeln!(buf, "  {flag}{req} — {help}")?;
        }
        writeln!(buf)?;
    }

    writer.write_all(buf.as_bytes())?;
    Ok(())
}

// ── Command detail mode ──────────────────────────────────────────────

/// Generate full detail for a single command by dotted path.
pub fn generate_command(
    cmd: &clap::Command,
    dotted_path: &str,
    writer: &mut impl Write,
) -> Result<()> {
    let mut buf = String::with_capacity(2 * 1024);
    let parts: Vec<&str> = dotted_path.split('.').collect();

    let mut current = cmd;
    let mut path_parts = vec![cmd.get_name().to_string()];

    for part in &parts {
        current = current
            .get_subcommands()
            .find(|s| s.get_name() == *part)
            .ok_or_else(|| {
                let available: Vec<_> = current
                    .get_subcommands()
                    .filter(|s| !s.is_hide_set() && s.get_name() != "help")
                    .map(|s| s.get_name().to_string())
                    .collect();
                anyhow::anyhow!(
                    "Unknown command '{part}'. Available: {}",
                    available.join(", ")
                )
            })?;
        path_parts.push(current.get_name().to_string());
    }

    let full_path = path_parts.join(" ");
    let about = current
        .get_about()
        .map(|a| a.to_string())
        .unwrap_or_default();

    writeln!(buf, "# {full_path}")?;
    writeln!(buf)?;
    writeln!(buf, "{about}")?;
    writeln!(buf)?;

    if let Some(long_about) = current.get_long_about() {
        writeln!(buf, "{long_about}")?;
        writeln!(buf)?;
    }

    // Arguments
    let args: Vec<_> = current
        .get_arguments()
        .filter(|a| {
            !a.is_hide_set()
                && a.get_id() != "help"
                && a.get_id() != "version"
                && !a.is_global_set()
        })
        .collect();

    if !args.is_empty() {
        writeln!(buf, "## Arguments")?;
        writeln!(buf)?;
        for arg in args {
            write_arg_detail(&mut buf, arg)?;
        }
    }

    // Subcommands
    let subs: Vec<_> = current
        .get_subcommands()
        .filter(|s| !s.is_hide_set() && s.get_name() != "help")
        .collect();

    if !subs.is_empty() {
        writeln!(buf, "## Subcommands")?;
        writeln!(buf)?;
        for sub in subs {
            let sub_about = sub.get_about().map(|a| a.to_string()).unwrap_or_default();
            writeln!(buf, "- `{} {}` — {sub_about}", full_path, sub.get_name())?;
        }
        writeln!(buf)?;
    }

    writer.write_all(buf.as_bytes())?;
    Ok(())
}

fn write_arg_detail(buf: &mut String, arg: &clap::Arg) -> Result<()> {
    let name = arg.get_id().as_str();
    let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();

    if arg.is_positional() {
        write!(buf, "- `<{name}>`")?;
    } else if let Some(long) = arg.get_long() {
        write!(buf, "- `--{long}`")?;
        if let Some(short) = arg.get_short() {
            write!(buf, " / `-{short}`")?;
        }
    } else if let Some(short) = arg.get_short() {
        write!(buf, "- `-{short}`")?;
    } else {
        return Ok(());
    }

    if arg.is_required_set() {
        write!(buf, " **(required)**")?;
    }

    writeln!(buf, " — {help}")?;

    let defaults = arg.get_default_values();
    if !defaults.is_empty() {
        let vals: Vec<&str> = defaults.iter().filter_map(|v| v.to_str()).collect();
        writeln!(buf, "  Default: `{}`", vals.join(", "))?;
    }

    if arg.get_action().takes_values() {
        let possible: Vec<_> = arg
            .get_possible_values()
            .iter()
            .map(|v| v.get_name().to_string())
            .collect();
        if !possible.is_empty() {
            writeln!(buf, "  Values: {}", possible.join(", "))?;
        }
    }

    Ok(())
}

// ── Full mode ────────────────────────────────────────────────────────

/// Generate the complete CLI reference.
pub fn generate_full(cmd: &clap::Command, writer: &mut impl Write) -> Result<()> {
    let mut buf = String::with_capacity(8 * 1024);
    let name = cmd.get_name();

    writeln!(buf, "# {name} — complete CLI reference")?;
    writeln!(buf)?;

    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || SKIP_SUBCOMMANDS.contains(&sub.get_name()) {
            continue;
        }
        write_command_full(&mut buf, sub, name)?;
    }

    writer.write_all(buf.as_bytes())?;
    Ok(())
}

fn write_command_full(buf: &mut String, cmd: &clap::Command, parent: &str) -> Result<()> {
    let full_name = format!("{parent} {}", cmd.get_name());
    let about = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();

    writeln!(buf, "## {full_name}")?;
    writeln!(buf, "{about}")?;
    writeln!(buf)?;

    let args: Vec<_> = cmd
        .get_arguments()
        .filter(|a| {
            !a.is_hide_set()
                && a.get_id() != "help"
                && a.get_id() != "version"
                && !a.is_global_set()
        })
        .collect();

    if !args.is_empty() {
        for arg in args {
            write_arg_detail(buf, arg)?;
        }
        writeln!(buf)?;
    }

    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || sub.get_name() == "help" {
            continue;
        }
        write_command_full(buf, sub, &full_name)?;
    }

    Ok(())
}

// ── SOP mode ─────────────────────────────────────────────────────────

/// Generate standard operating procedures for a specific tool.
pub fn generate_sop(tool: &str, writer: &mut impl Write) -> Result<()> {
    let sop = match tool.to_lowercase().as_str() {
        "lint" | "check" => SOP_LINT,
        "migrate" | "migration" => SOP_MIGRATE,
        "lsp" | "editor" | "editors" => SOP_LSP,
        _ => bail!("Unknown SOP: '{tool}'. Available: lint, migrate, lsp"),
    };
    writer.write_all(sop.as_bytes())?;
    Ok(())
}

const SOP_LINT: &str = r#"# SOP: Linting Fleet GitOps YAML

## Setup
```
1. flint init                                  # create .fleetlint.toml (auto-detects repo structure)
2. flint init --no-interactive                 # non-interactive mode with defaults
```

## Lint files
```
1. flint check <path>                          # lint single file or directory
2. flint check <path> --format json            # structured JSON output
```

## Auto-fix
```
1. flint check <path> --fix                    # apply safe fixes only
2. flint check <path> --fix --unsafe-fixes     # also apply risky fixes
```

## Inspect rules
```
1. flint list-rules                            # table of all rules
2. flint list-rules --format json              # rules as JSON with metadata
```

## Configuration (.fleetlint.toml)
```toml
[rules]
disabled = ["secret-hygiene"]        # disable specific rules
warn = ["interval-validation"]       # downgrade errors to warnings

[thresholds]
min_interval = 60                    # minimum query interval (seconds)
max_interval = 86400                 # maximum query interval

[files]
include = ["**/*.yml", "**/*.yaml"]
exclude = ["node_modules", "target"]

[deprecations]
fleet_version = "4.85.0"            # target version for deprecation checks
future_names = true                  # opt-in to new naming (reports, settings, fleets)
```

## Inline suppressions
```yaml
queries:  # flint: ignore [deprecated-keys]
```

## Key flags
- `--format json` — structured output for programmatic consumption
- `--fix` — auto-apply safe fixes (key renames, typo corrections)
- `--unsafe-fixes` — also apply fixes that may change semantics (requires --fix)
"#;

const SOP_MIGRATE: &str = r#"# SOP: Fleet GitOps Migration

Migrate a Fleet GitOps repo to a target version using `flint migrate`.

## Step 1: Generate migration report
```
flint migrate <path> --target-version <version>
```
Output is JSON with:
- `summary` — counts: files_scanned, directory_renames, file_renames, key_renames, safe_fixes
- `directory_renames[]` — `{ old, new, files_affected }`
- `file_renames[]` — `{ old, new }`
- `file_changes[]` — `{ path, move_to?, key_renames[] }` where each key_rename has `{ line, old_key, new_key, safety }`

## Step 2: Apply directory renames (first)
```
mv <path>/teams/ <path>/fleets/
```

## Step 3: Apply file renames
- Root level: `mv <path>/no-team.yml <path>/unassigned.yml`
- Inside moved dirs: `mv <path>/fleets/no-team.yml <path>/fleets/unassigned.yml`

## Step 4: Apply key renames
For each file in `file_changes`:
- Use `move_to` path if set (directory was moved in Step 2)
- Apply renames bottom-up (highest line number first) to preserve offsets
- Replace `old_key:` with `new_key:` at the specified line

## Step 5: Update cross-file path references
```
grep -rn "teams/" <path>/**/*.yml      # find stale path: references
grep -rn "no-team.yml" <path>/**/*.yml
```
Replace: teams/ -> fleets/, no-team.yml -> unassigned.yml

## Step 6: Verify
```
flint check <path>                     # confirm zero deprecation warnings
```

## Current renames (warnings since v4.80.1)
- Directory: `teams/` -> `fleets/`
- File: `no-team.yml` -> `unassigned.yml`
- Key: `team_settings` -> `settings`
- Key: `queries` -> `reports`
"#;

const SOP_LSP: &str = r#"# SOP: Editor Setup (LSP)

All editors use `flint lsp` as a language server subprocess.

## VS Code
Install the `fleetdm.flint` extension from the marketplace.

## Neovim
```lua
require('flint').setup()
```
Or add to lua/flint.lua (see editors/neovim/).

## Zed
Install the flint extension from the Zed extension gallery.

## Sublime Text
1. Install Package Control
2. Install LSP package
3. Install Flint LSP package (see editors/sublime/)

## JetBrains (IntelliJ, etc.)
Install the Flint plugin (see editors/jetbrains/).

## What the LSP provides
- Real-time diagnostics (linting on every keystroke)
- Hover documentation (field descriptions, platform info)
- Autocompletion (keys, platforms, osquery tables, SQL keywords)
- Code actions (quick-fixes for deprecated keys, typos)
- Go-to-definition (path references)
- Document symbols and folding
- Semantic syntax highlighting

## Configuration
The LSP reads `.fleetlint.toml` for:
- Rule configuration (disabled rules, warning overrides)
- Fleet server connection (URL, token for live validation)
- Deprecation settings (target version, future_names opt-in)
"#;

// ── JSON mode ────────────────────────────────────────────────────────

/// Generate JSON schema of the CLI.
/// If `path` is provided, scopes to that subtree with global flags stripped.
pub fn generate_json(
    cmd: &clap::Command,
    path: Option<&str>,
    writer: &mut impl Write,
) -> Result<()> {
    let json = if let Some(path) = path {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = cmd;
        for part in &parts {
            current = current
                .get_subcommands()
                .find(|s| s.get_name() == *part)
                .ok_or_else(|| {
                    let available: Vec<_> = current
                        .get_subcommands()
                        .filter(|s| !s.is_hide_set() && s.get_name() != "help")
                        .map(|s| s.get_name().to_string())
                        .collect();
                    anyhow::anyhow!(
                        "Unknown command '{part}'. Available: {}",
                        available.join(", ")
                    )
                })?;
        }
        command_to_json_no_globals(current)
    } else {
        command_to_json(cmd)
    };
    let output = serde_json::to_string_pretty(&json)?;
    writer.write_all(output.as_bytes())?;
    writeln!(writer)?;
    Ok(())
}

fn command_to_json(cmd: &clap::Command) -> serde_json::Value {
    let args: Vec<serde_json::Value> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set() && a.get_id() != "help" && a.get_id() != "version")
        .map(arg_to_json)
        .collect();

    let subcommands: Vec<serde_json::Value> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set() && s.get_name() != "help")
        .map(command_to_json)
        .collect();

    let mut obj = serde_json::json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|a| a.to_string()),
    });

    if let Some(version) = cmd.get_version() {
        obj["version"] = serde_json::json!(version);
    }

    if !args.is_empty() {
        obj["args"] = serde_json::json!(args);
    }

    if !subcommands.is_empty() {
        obj["subcommands"] = serde_json::json!(subcommands);
    }

    obj
}

fn command_to_json_no_globals(cmd: &clap::Command) -> serde_json::Value {
    let args: Vec<serde_json::Value> = cmd
        .get_arguments()
        .filter(|a| {
            !a.is_hide_set()
                && a.get_id() != "help"
                && a.get_id() != "version"
                && !a.is_global_set()
        })
        .map(arg_to_json)
        .collect();

    let subcommands: Vec<serde_json::Value> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set() && s.get_name() != "help")
        .map(command_to_json_no_globals)
        .collect();

    let mut obj = serde_json::json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|a| a.to_string()),
    });

    if !args.is_empty() {
        obj["args"] = serde_json::json!(args);
    }

    if !subcommands.is_empty() {
        obj["subcommands"] = serde_json::json!(subcommands);
    }

    obj
}

fn arg_to_json(arg: &clap::Arg) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "name": arg.get_id().as_str(),
        "required": arg.is_required_set(),
        "positional": arg.is_positional(),
    });

    if let Some(long) = arg.get_long() {
        obj["long"] = serde_json::json!(format!("--{long}"));
    }

    if let Some(short) = arg.get_short() {
        obj["short"] = serde_json::json!(format!("-{short}"));
    }

    if let Some(help) = arg.get_help() {
        obj["help"] = serde_json::json!(help.to_string());
    }

    let defaults = arg.get_default_values();
    if !defaults.is_empty() {
        let vals: Vec<&str> = defaults.iter().filter_map(|v| v.to_str()).collect();
        obj["default"] = serde_json::json!(vals.join(", "));
    }

    if arg.get_action().takes_values() {
        let possible: Vec<_> = arg
            .get_possible_values()
            .iter()
            .map(|v| v.get_name().to_string())
            .collect();
        if !possible.is_empty() {
            obj["possible_values"] = serde_json::json!(possible);
        }
    }

    if arg.is_global_set() {
        obj["global"] = serde_json::json!(true);
    }

    obj
}

// ── Skill file installation ──────────────────────────────────────────

const SKILL_FLINT: &str = include_str!("../skills/flint.md");
const SKILL_FLEET_MIGRATE: &str = include_str!("../skills/fleet-migrate.md");

/// Install Claude Code skill files for flint.
///
/// Creates `.claude/skills/flint.md` and `.claude/skills/fleet-migrate.md`,
/// then ensures `CLAUDE.md` has a bootstrap hint.
pub fn install_skill(version: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    // 1. Install skill files
    let skills_dir = Path::new(".claude/skills");
    fs::create_dir_all(skills_dir)?;

    for (filename, template) in &[
        ("flint.md", SKILL_FLINT),
        ("fleet-migrate.md", SKILL_FLEET_MIGRATE),
    ] {
        let skill_path = skills_dir.join(filename);
        let content = template.replace("{{VERSION}}", version);
        fs::write(&skill_path, &content)?;
        eprintln!("\u{2713} Installed Claude Code skill: .claude/skills/{filename}");
    }

    // 2. Ensure CLAUDE.md has a flint bootstrap hint
    let bootstrap_line = "## flint\n\n\
        `flint` (Fleet GitOps linter) is available. \
        Run `flint setup-agent` to install the AI agent skill, \
        or `flint help-ai` for the command reference.\n";

    let claude_md = Path::new("CLAUDE.md");
    if claude_md.exists() {
        let existing = fs::read_to_string(claude_md)?;
        if !existing.contains("flint setup-agent") && !existing.contains("flint help-ai") {
            let mut updated = existing;
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push('\n');
            updated.push_str(bootstrap_line);
            fs::write(claude_md, updated)?;
            eprintln!("\u{2713} Added flint bootstrap hint to CLAUDE.md");
        } else {
            eprintln!("  CLAUDE.md already references flint");
        }
    } else {
        let content = format!("# Project Instructions\n\n{bootstrap_line}");
        fs::write(claude_md, content)?;
        eprintln!("\u{2713} Created CLAUDE.md with flint bootstrap hint");
    }

    eprintln!("  Agents will now discover flint automatically.");
    eprintln!("  Regenerate anytime with: flint help-ai --install-skill");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};

    fn sample_cmd() -> Command {
        Command::new("flint")
            .version("0.1.0")
            .about("Test tool")
            .subcommand(
                Command::new("check")
                    .about("Lint YAML files")
                    .arg(Arg::new("path").required(true))
                    .arg(
                        Arg::new("fix")
                            .long("fix")
                            .action(clap::ArgAction::SetTrue)
                            .help("Auto-fix"),
                    ),
            )
            .subcommand(
                Command::new("migrate")
                    .about("Generate migration report")
                    .arg(Arg::new("path").required(true)),
            )
    }

    #[test]
    fn test_generate_index() {
        let cmd = sample_cmd();
        let mut out = Vec::new();
        generate_index(&cmd, &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("# flint"));
        assert!(output.contains("## Command index"));
        assert!(output.contains("check"));
        assert!(output.contains("migrate"));
    }

    #[test]
    fn test_generate_command() {
        let cmd = sample_cmd();
        let mut out = Vec::new();
        generate_command(&cmd, "check", &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("# flint check"));
        assert!(output.contains("--fix"));
    }

    #[test]
    fn test_generate_command_not_found() {
        let cmd = sample_cmd();
        let mut out = Vec::new();
        let result = generate_command(&cmd, "nonexistent", &mut out);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_sop_lint() {
        let mut out = Vec::new();
        generate_sop("lint", &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("SOP: Linting"));
    }

    #[test]
    fn test_generate_sop_unknown() {
        let mut out = Vec::new();
        let result = generate_sop("nonexistent", &mut out);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_json() {
        let cmd = sample_cmd();
        let mut out = Vec::new();
        generate_json(&cmd, None, &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["name"], "flint");
        assert!(json["subcommands"].is_array());
    }

    #[test]
    fn test_generate_json_scoped() {
        let cmd = sample_cmd();
        let mut out = Vec::new();
        generate_json(&cmd, Some("check"), &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["name"], "check");
    }
}
