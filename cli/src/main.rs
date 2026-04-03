//! Flint CLI — Fleet GitOps YAML linter and language server.
//!
//! Thin command dispatcher that delegates to `flint-lint` (linting engine)
//! and `flint-lsp` (language server). See `flint help-ai` for agent discovery.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use flint_lint as linter;
use flint_lsp as lsp;

#[derive(Parser)]
#[command(name = "flint")]
#[command(version = concat!(
    env!("CARGO_PKG_VERSION"), "+", env!("BUILD_TIMESTAMP"),
    " (Fleet sync: ", env!("FLEET_SYNC_COMMIT"), ", ", env!("FLEET_SYNC_DATE"), ")"
))]
#[command(about = "Flint — Fleet GitOps YAML linter and language server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check (lint) YAML file(s) with Fleet-specific validation
    #[command(alias = "lint")]
    Check {
        /// File or directory to lint
        path: PathBuf,

        /// Automatically apply safe fixes
        #[arg(long)]
        fix: bool,

        /// Also apply fixes that may change semantics (requires --fix)
        #[arg(long)]
        unsafe_fixes: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Start language server (called by editor extensions, not directly)
    #[command(hide = true)]
    Lsp {
        /// Enable debug logging to stderr
        #[arg(long)]
        debug: bool,

        /// Use stdio transport (default, accepted for compatibility)
        #[arg(long)]
        stdio: bool,
    },

    /// Initialize Fleet linter configuration
    ///
    /// Creates a .fleetlint.toml configuration file in the current directory.
    /// Auto-detects your Fleet GitOps structure and generates sensible defaults.
    Init {
        /// Output path for config file (default: .fleetlint.toml)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Skip interactive prompts, use detected/default values
        #[arg(long)]
        no_interactive: bool,

        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// List all available lint rules
    ListRules {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Generate a migration report for upgrading Fleet GitOps YAML
    Migrate {
        /// Root directory of the GitOps repo
        path: PathBuf,

        /// Target Fleet version (e.g., "4.85.0" or "latest")
        #[arg(long, default_value = "latest")]
        target_version: String,
    },

    /// Output CLI reference for AI agents (default: command index)
    #[command(name = "help-agents", alias = "help-ai")]
    HelpAgents {
        /// Show full detail for a specific command (dot notation)
        #[arg(long)]
        command: Option<String>,

        /// Show standard operating procedures for a tool (lint, migrate, lsp)
        #[arg(long)]
        sop: Option<String>,

        /// Output the complete reference (all commands, all flags)
        #[arg(long)]
        full: bool,

        /// Install Claude Code skill files for flint
        #[arg(long)]
        install_skill: bool,
    },

    /// Install AI agent skill files (.claude/skills/)
    #[command(name = "setup-agent")]
    SetupAgent,

    /// Output CLI schema as JSON for tooling integration
    #[command(name = "help-json", hide = true)]
    HelpJson {
        /// Command path to scope output (dot notation, e.g. check)
        command: Option<String>,
    },

    /// Show the directory tree of a Fleet GitOps repo
    Tree {
        /// Root directory (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            path,
            fix,
            unsafe_fixes,
            format,
        } => {
            use linter::Linter;

            use colored::Colorize;

            let linter = Linter::new();
            let json_mode = format == "json";

            if path.is_file() {
                let source = std::fs::read_to_string(&path)?;
                let report = linter.lint_file(&path)?;

                // Apply fixes if requested
                if fix {
                    let fixed = apply_fixes(&path, &report, unsafe_fixes)?;
                    if fixed > 0 && !json_mode {
                        println!(
                            "{} Fixed {} issue(s) in {}",
                            "✓".green(),
                            fixed,
                            path.display()
                        );
                    }
                }

                if json_mode {
                    let output = lint_report_to_json(&path.display().to_string(), &report);
                    let wrapper = serde_json::json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "files": [output],
                        "summary": {
                            "files_linted": 1,
                            "errors": report.errors.len(),
                            "warnings": report.warnings.len(),
                            "infos": report.infos.len(),
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&wrapper)?);
                } else {
                    println!("{} Linting {}...\n", "🔍".blue(), path.display());
                    report.print(Some(&source));
                }

                if report.has_errors() {
                    std::process::exit(1);
                }
            } else if path.is_dir() {
                let results = linter.lint_directory(&path, None)?;

                // Apply fixes if requested
                if fix {
                    let mut total_fixed = 0;
                    for (file_path, report) in &results {
                        if let Ok(n) = apply_fixes(file_path, report, unsafe_fixes) {
                            total_fixed += n;
                        }
                    }
                    if total_fixed > 0 && !json_mode {
                        println!("{} Fixed {} issue(s)\n", "✓".green(), total_fixed);
                    }
                }

                let mut total_errors = 0;
                let mut total_warnings = 0;
                let mut total_infos = 0;

                if json_mode {
                    let mut file_outputs = Vec::new();

                    for (file_path, report) in &results {
                        total_errors += report.errors.len();
                        total_warnings += report.warnings.len();
                        total_infos += report.infos.len();
                        file_outputs.push(lint_report_to_json(
                            &file_path.display().to_string(),
                            report,
                        ));
                    }

                    let wrapper = serde_json::json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "files": file_outputs,
                        "summary": {
                            "files_linted": results.len(),
                            "errors": total_errors,
                            "warnings": total_warnings,
                            "infos": total_infos,
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&wrapper)?);
                } else {
                    println!("{} Linting directory {}...\n", "🔍".blue(), path.display());

                    for (file_path, report) in &results {
                        if report.total_issues() > 0 {
                            println!("\n{} {}", "File:".bold(), file_path.display());

                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                report.print(Some(&source));
                            } else {
                                report.print(None);
                            }

                            total_errors += report.errors.len();
                            total_warnings += report.warnings.len();
                            total_infos += report.infos.len();
                        }
                    }

                    println!("\n{}", "=".repeat(60));
                    println!("{} Linted {} file(s)", "Summary:".bold(), results.len());
                    println!("  {} error(s)", total_errors.to_string().red());
                    println!("  {} warning(s)", total_warnings.to_string().yellow());
                    println!("  {} info", total_infos.to_string().blue());
                }

                if total_errors > 0 {
                    std::process::exit(1);
                }
            } else {
                anyhow::bail!("Path does not exist: {}", path.display());
            }
        }

        Commands::Lsp { debug, stdio: _ } => {
            // Set up logging if debug mode is enabled
            if debug {
                eprintln!("Fleet LSP server starting in debug mode...");
                // TODO: Set up tracing/logging to stderr
            }

            // Start the LSP server - this blocks until the client disconnects
            // Note: stdio transport is always used, the --stdio flag is accepted for compatibility
            lsp::start_server().await?;
        }

        Commands::Init {
            output,
            no_interactive,
            force,
        } => {
            let current_dir = std::env::current_dir()?;
            linter::init_config(&current_dir, output, !no_interactive, force)?;
        }

        Commands::ListRules { format } => {
            let ruleset = linter::RuleSet::default_rules();

            if format == "json" {
                let rules: Vec<serde_json::Value> = ruleset
                    .rules()
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "name": r.name(),
                            "description": r.description(),
                            "category": r.category(),
                            "fixable": r.is_fixable(),
                            "preview": r.is_preview(),
                            "severity": format!("{:?}", r.default_severity()),
                            "docs_url": r.docs_url(),
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "rules": rules,
                        "total": rules.len(),
                    }))?
                );
            } else {
                use colored::Colorize;
                println!("{}", "Fleet GitOps Lint Rules".bold());
                println!("{}", "=".repeat(90));
                println!(
                    "{:<28} {:<14} {:<8} {}",
                    "Rule".bold(),
                    "Category".bold(),
                    "Fixable".bold(),
                    "Description".bold()
                );
                println!("{}", "-".repeat(90));
                for rule in ruleset.rules() {
                    let fixable = if rule.is_fixable() {
                        "yes".green()
                    } else {
                        "no".dimmed()
                    };
                    println!(
                        "{:<28} {:<14} {:<8} {}",
                        rule.name(),
                        rule.category(),
                        fixable,
                        rule.description()
                    );
                }
                println!("{}", "-".repeat(90));
                println!("{} rule(s) total", ruleset.rules().len());
            }
        }

        Commands::Migrate {
            path,
            target_version,
        } => {
            use linter::{
                DeprecationKind, FixSafety, Linter, RuleSet, VersionContext, DEPRECATION_REGISTRY,
            };

            if !path.is_dir() {
                anyhow::bail!("Not a directory: {}", path.display());
            }

            // Build version context with future_names enabled so all active deprecations fire
            let mut version_ctx = VersionContext::from_config(&target_version);
            version_ctx.future_names = true;

            let target_ver = version_ctx.version.clone();
            let linter = Linter::with_rules(RuleSet::default_rules_with_version(version_ctx));
            let results = linter.lint_directory(&path, None)?;

            // Collect deprecation diagnostics from lint results
            let mut file_changes: Vec<serde_json::Value> = Vec::new();
            let mut total_key_renames = 0usize;
            let mut total_safe = 0usize;
            let mut total_unsafe = 0usize;

            for (file_path_buf, report) in &results {
                let all_errors: Vec<&linter::LintError> = report
                    .errors
                    .iter()
                    .chain(report.warnings.iter())
                    .chain(report.infos.iter())
                    .collect();

                let key_renames: Vec<serde_json::Value> = all_errors
                    .iter()
                    .filter(|e| e.rule_code.as_deref() == Some("deprecated-keys"))
                    .filter(|e| e.context.is_some() && e.suggestion.is_some())
                    .map(|e| {
                        let safety = match e.fix_safety.as_ref() {
                            Some(FixSafety::Safe) => "safe",
                            _ => "unsafe",
                        };
                        if safety == "safe" {
                            total_safe += 1;
                        } else {
                            total_unsafe += 1;
                        }
                        serde_json::json!({
                            "line": e.line.unwrap_or(0),
                            "old_key": e.context.as_deref().unwrap_or(""),
                            "new_key": e.suggestion.as_deref().unwrap_or(""),
                            "safety": safety,
                        })
                    })
                    .collect();

                if key_renames.is_empty() {
                    continue;
                }

                total_key_renames += key_renames.len();

                // Compute relative path and potential move_to
                let file_path_str = file_path_buf.display().to_string();
                let rel_path = file_path_str
                    .strip_prefix(&format!("{}/", path.display()))
                    .or_else(|| file_path_str.strip_prefix(&path.display().to_string()))
                    .unwrap_or(&file_path_str);

                let mut entry = serde_json::json!({
                    "path": rel_path,
                    "key_renames": key_renames,
                });

                // Check if this file is inside a directory that needs renaming
                for dep in DEPRECATION_REGISTRY.active_directory_renames(&target_ver) {
                    if let DeprecationKind::DirectoryRename { old_dir, new_dir } = &dep.kind {
                        let prefix = format!("{}/", old_dir);
                        if rel_path.starts_with(&prefix) {
                            let new_path = format!("{}/{}", new_dir, &rel_path[prefix.len()..]);
                            entry
                                .as_object_mut()
                                .unwrap()
                                .insert("move_to".into(), serde_json::json!(new_path));
                        }
                    }
                }

                file_changes.push(entry);
            }

            // Scan for directory renames
            let mut dir_renames: Vec<serde_json::Value> = Vec::new();
            for dep in DEPRECATION_REGISTRY.active_directory_renames(&target_ver) {
                if let DeprecationKind::DirectoryRename { old_dir, new_dir } = &dep.kind {
                    let old_path = path.join(old_dir);
                    if old_path.is_dir() {
                        let file_count = walkdir_count(&old_path);
                        dir_renames.push(serde_json::json!({
                            "old": old_dir,
                            "new": new_dir,
                            "files_affected": file_count,
                        }));
                    }
                }
            }

            // Scan for file renames from registry
            let mut file_renames: Vec<serde_json::Value> = Vec::new();
            for dep in DEPRECATION_REGISTRY.active_file_renames(&target_ver) {
                if let DeprecationKind::FileRename { old_name, new_name } = &dep.kind {
                    if path.join(old_name).exists() {
                        file_renames.push(serde_json::json!({
                            "old": old_name,
                            "new": new_name,
                        }));
                    }
                }
            }

            let report = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "target_version": target_ver.to_string(),
                "summary": {
                    "files_scanned": results.len(),
                    "deprecations_found": total_key_renames + dir_renames.len() + file_renames.len(),
                    "directory_renames": dir_renames.len(),
                    "file_renames": file_renames.len(),
                    "key_renames": total_key_renames,
                    "safe_fixes": total_safe,
                    "unsafe_fixes": total_unsafe,
                },
                "directory_renames": dir_renames,
                "file_renames": file_renames,
                "file_changes": file_changes,
            });

            println!("{}", serde_json::to_string_pretty(&report)?);
        }

        Commands::SetupAgent => {
            linter::help_agents::install_skill(env!("CARGO_PKG_VERSION"))?;
        }

        Commands::HelpAgents {
            command,
            sop,
            full,
            install_skill,
        } => {
            if install_skill {
                linter::help_agents::install_skill(env!("CARGO_PKG_VERSION"))?;
                return Ok(());
            }
            use clap::CommandFactory;
            let cmd = Cli::command();
            let mut out = std::io::stdout();
            if let Some(tool) = sop {
                linter::help_agents::generate_sop(&tool, &mut out)?;
            } else if let Some(path) = command {
                linter::help_agents::generate_command(&cmd, &path, &mut out)?;
            } else if full {
                linter::help_agents::generate_full(&cmd, &mut out)?;
            } else {
                linter::help_agents::generate_index(&cmd, &mut out)?;
            }
        }

        Commands::HelpJson { command } => {
            use clap::CommandFactory;
            let cmd = Cli::command();
            let mut out = std::io::stdout();
            linter::help_agents::generate_json(&cmd, command.as_deref(), &mut out)?;
        }

        Commands::Tree { path } => {
            if !path.is_dir() {
                anyhow::bail!("Not a directory: {}", path.display());
            }
            println!("{}", path.display());
            print_tree(&path, "")?;
        }
    }

    Ok(())
}

/// Count YAML files in a directory recursively.
fn walkdir_count(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += walkdir_count(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "yml" || ext == "yaml" {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Print a directory tree, excluding .git, .DS_Store, node_modules, target, dist.
fn print_tree(dir: &std::path::Path, prefix: &str) -> anyhow::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            !matches!(
                s.as_ref(),
                ".git" | ".DS_Store" | "node_modules" | "target" | "dist" | ".gitkeep"
            )
        })
        .collect();

    entries.sort_by_key(|e| {
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        (!is_dir, e.file_name()) // dirs first, then alphabetical
    });

    let total = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };
        let name = entry.file_name();

        println!("{}{}{}", prefix, connector, name.to_string_lossy());

        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            print_tree(&entry.path(), &format!("{}{}", prefix, child_prefix))?;
        }
    }

    Ok(())
}

/// Convert a LintReport to a JSON value for structured output.
fn lint_report_to_json(file_path: &str, report: &linter::error::LintReport) -> serde_json::Value {
    let to_json = |errors: &[linter::LintError]| -> Vec<serde_json::Value> {
        errors
            .iter()
            .map(|e| {
                let mut obj = serde_json::json!({
                    "message": e.message,
                    "severity": match e.severity {
                        linter::Severity::Error => "error",
                        linter::Severity::Warning => "warning",
                        linter::Severity::Info => "info",
                    },
                });
                let m = obj.as_object_mut().unwrap();
                if let Some(line) = e.line {
                    m.insert("line".into(), serde_json::json!(line));
                }
                if let Some(col) = e.column {
                    m.insert("column".into(), serde_json::json!(col));
                }
                if let Some(ref code) = e.rule_code {
                    m.insert("rule".into(), serde_json::json!(code));
                }
                if let Some(ref help) = e.help {
                    m.insert("help".into(), serde_json::json!(help));
                }
                if let Some(ref suggestion) = e.suggestion {
                    m.insert("suggestion".into(), serde_json::json!(suggestion));
                }
                if let Some(ref ctx) = e.context {
                    m.insert("context".into(), serde_json::json!(ctx));
                }
                obj
            })
            .collect()
    };

    let mut diagnostics = to_json(&report.errors);
    diagnostics.extend(to_json(&report.warnings));
    diagnostics.extend(to_json(&report.infos));

    serde_json::json!({
        "path": file_path,
        "diagnostics": diagnostics,
        "counts": {
            "errors": report.errors.len(),
            "warnings": report.warnings.len(),
            "infos": report.infos.len(),
        }
    })
}

/// Apply auto-fixable suggestions to a file.
///
/// Collects all fixable errors (Safe, or Unsafe if `include_unsafe` is true),
/// applies them bottom-up to preserve line numbers, and writes the file back.
/// Returns the number of fixes applied.
fn apply_fixes(
    file_path: &std::path::Path,
    report: &linter::error::LintReport,
    include_unsafe: bool,
) -> anyhow::Result<usize> {
    use linter::error::FixSafety;

    // Collect fixable errors from all severity levels
    let all_errors: Vec<&linter::LintError> = report
        .errors
        .iter()
        .chain(report.warnings.iter())
        .chain(report.infos.iter())
        .collect();

    // Filter to fixable errors with line/context info
    let mut fixes: Vec<(&linter::LintError, &str)> = all_errors
        .iter()
        .filter_map(|e| {
            let suggestion = e.suggestion.as_deref()?;
            let safety = e.fix_safety.as_ref()?;

            match safety {
                FixSafety::Safe => Some((*e, suggestion)),
                FixSafety::Unsafe if include_unsafe => Some((*e, suggestion)),
                _ => None,
            }
        })
        .filter(|(e, _)| e.line.is_some() && e.context.is_some())
        .collect();

    if fixes.is_empty() {
        return Ok(0);
    }

    let source = std::fs::read_to_string(file_path)?;
    let mut lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();

    // Sort by line number descending so replacements don't shift earlier lines
    fixes.sort_by(|a, b| b.0.line.cmp(&a.0.line));

    let mut applied = 0;

    for (error, suggestion) in &fixes {
        let line_idx = error.line.unwrap() - 1; // 0-indexed
        if line_idx >= lines.len() {
            continue;
        }

        let context = error.context.as_deref().unwrap();
        let line = &lines[line_idx];

        // Replace the context value with the suggestion on this line
        if let Some(pos) = line.find(context) {
            let mut new_line = String::new();
            new_line.push_str(&line[..pos]);
            new_line.push_str(suggestion);
            new_line.push_str(&line[pos + context.len()..]);
            lines[line_idx] = new_line;
            applied += 1;
        }
    }

    if applied > 0 {
        let mut output = lines.join("\n");
        // Preserve trailing newline if original had one
        if source.ends_with('\n') {
            output.push('\n');
        }
        std::fs::write(file_path, output)?;
    }

    Ok(applied)
}
