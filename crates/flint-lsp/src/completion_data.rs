//! Completion data loaded from TOML files at compile time.
//!
//! This module parses `data/completions.toml` and `data/fma-registry.toml`,
//! providing typed access to field definitions, block snippets, glob patterns,
//! categories, labels, and Fleet Maintained App slugs.
//!
//! See ADR-009 for design rationale.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::path::Path;

// ============================================================================
// Raw TOML sources (embedded at compile time)
// ============================================================================

const COMPLETIONS_TOML: &str = include_str!("../data/completions.toml");
const FMA_REGISTRY_TOML: &str = include_str!("../data/fma-registry.toml");

// ============================================================================
// Data types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CompletionData {
    pub categories: Categories,
    #[serde(default)]
    pub labels: Vec<LabelEntry>,
    #[serde(default)]
    pub fields: Vec<FieldEntry>,
    #[serde(default)]
    pub blocks: Vec<BlockEntry>,
    #[serde(default)]
    pub globs: Vec<GlobEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Categories {
    pub values: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LabelEntry {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct FieldEntry {
    pub context: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Deserialize)]
pub struct BlockEntry {
    pub context: String,
    pub name: String,
    pub description: String,
    pub snippet: String,
}

#[derive(Debug, Deserialize)]
pub struct GlobEntry {
    pub context: String,
    pub pattern: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct FmaRegistry {
    #[serde(default)]
    pub fma: Vec<FmaEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FmaEntry {
    pub name: String,
    pub platforms: Vec<String>,
}

// ============================================================================
// Parsed statics
// ============================================================================

pub static COMPLETION_DATA: Lazy<CompletionData> =
    Lazy::new(|| toml::from_str(COMPLETIONS_TOML).expect("Failed to parse completions.toml"));

pub static FMA_REGISTRY: Lazy<FmaRegistry> =
    Lazy::new(|| toml::from_str(FMA_REGISTRY_TOML).expect("Failed to parse fma-registry.toml"));

// ============================================================================
// Query helpers
// ============================================================================

/// Get all field definitions for a given completion context.
pub fn fields_for_context(context: &str) -> Vec<&'static FieldEntry> {
    COMPLETION_DATA
        .fields
        .iter()
        .filter(|f| f.context == context)
        .collect()
}

/// Get all block snippet templates for a given completion context.
pub fn blocks_for_context(context: &str) -> Vec<&'static BlockEntry> {
    COMPLETION_DATA
        .blocks
        .iter()
        .filter(|b| b.context == context)
        .collect()
}

/// Get all glob patterns for a given context.
///
/// Returns patterns with `{base}` still unresolved — call `resolve_base()`
/// to substitute the correct relative path.
pub fn globs_for_context(context: &str) -> Vec<&'static GlobEntry> {
    COMPLETION_DATA
        .globs
        .iter()
        .filter(|g| g.context == context)
        .collect()
}

/// Get all FMA slugs for a given platform (e.g., "darwin", "windows").
///
/// Returns full slugs like `["slack/darwin", "google-chrome/darwin"]`.
pub fn fma_slugs_for_platform(platform: &str) -> Vec<String> {
    FMA_REGISTRY
        .fma
        .iter()
        .filter(|app| app.platforms.iter().any(|p| p == platform))
        .map(|app| format!("{}/{}", app.name, platform))
        .collect()
}

/// Get all FMA slugs across all platforms.
///
/// Returns full slugs like `["slack/darwin", "slack/windows", ...]`.
pub fn all_fma_slugs() -> Vec<String> {
    FMA_REGISTRY
        .fma
        .iter()
        .flat_map(|app| {
            app.platforms
                .iter()
                .map(move |p| format!("{}/{}", app.name, p))
        })
        .collect()
}

/// Check if a slug is a known Fleet Maintained App.
pub fn is_valid_fma_slug(slug: &str) -> bool {
    if let Some((name, platform)) = slug.rsplit_once('/') {
        FMA_REGISTRY
            .fma
            .iter()
            .any(|app| app.name == name && app.platforms.iter().any(|p| p == platform))
    } else {
        false
    }
}

/// Find the closest matching FMA slug for typo suggestions.
pub fn find_similar_fma_slug(input: &str) -> Option<String> {
    let input_lower = input.to_lowercase();
    let all = all_fma_slugs();

    // Exact case-insensitive match
    if let Some(s) = all.iter().find(|s| s.to_lowercase() == input_lower) {
        return Some(s.clone());
    }

    // Prefix match (user typed "sla" → "slack/darwin")
    if let Some(s) = all
        .iter()
        .find(|s| s.to_lowercase().starts_with(&input_lower))
    {
        return Some(s.clone());
    }

    // Contains match
    if let Some(s) = all.iter().find(|s| s.to_lowercase().contains(&input_lower)) {
        return Some(s.clone());
    }

    None
}

/// Resolve the `{base}` placeholder in a pattern or snippet.
///
/// Detects whether the workspace uses `platforms/` or `lib/` and computes
/// the relative path from `file_path` to the lib directory.
///
/// Returns the resolved string with `{base}` replaced.
pub fn resolve_base(template: &str, file_path: &Path, workspace_root: &Path) -> String {
    let base = compute_base_path(file_path, workspace_root);
    template.replace("{base}", &base)
}

/// Compute the `{base}` value for a given file in a workspace.
///
/// Examples:
/// - `fleets/workstations.yml` in a `platforms/` repo → `../platforms`
/// - `teams/ops.yml` in a `lib/` repo → `../lib`
/// - `default.yml` in a `platforms/` repo → `./platforms`
fn compute_base_path(file_path: &Path, workspace_root: &Path) -> String {
    // Detect which convention the repo uses
    let dir_name = if workspace_root.join("platforms").is_dir() {
        "platforms"
    } else if workspace_root.join("lib").is_dir() {
        "lib"
    } else {
        "platforms" // default fallback
    };

    // Compute relative path from file's directory to workspace root
    let file_dir = if file_path.is_file() {
        file_path.parent().unwrap_or(workspace_root)
    } else {
        file_path
    };

    // Count how many levels deep the file is relative to workspace root
    let rel = pathdiff_relative(file_dir, workspace_root);
    let depth = rel.components().count();

    if depth == 0 {
        format!("./{}", dir_name)
    } else {
        let ups = vec![".."; depth].join("/");
        format!("{}/{}", ups, dir_name)
    }
}

/// Simple relative path computation (file_dir relative to base).
fn pathdiff_relative(from: &Path, to: &Path) -> std::path::PathBuf {
    // Normalize both paths
    let from = from.canonicalize().unwrap_or_else(|_| from.to_path_buf());
    let to = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    if let Ok(rel) = from.strip_prefix(&to) {
        rel.to_path_buf()
    } else {
        // Fallback: assume one level deep
        std::path::PathBuf::from(".")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Data loading ───────────────────────────────────────────

    #[test]
    fn completions_toml_parses() {
        // Force parse — panics if TOML is malformed
        let _ = &*COMPLETION_DATA;
    }

    #[test]
    fn fma_registry_parses() {
        let _ = &*FMA_REGISTRY;
    }

    #[test]
    fn categories_non_empty() {
        assert!(
            !COMPLETION_DATA.categories.values.is_empty(),
            "Categories list must not be empty"
        );
    }

    #[test]
    fn labels_non_empty() {
        assert!(
            !COMPLETION_DATA.labels.is_empty(),
            "Labels list must not be empty"
        );
    }

    #[test]
    fn fields_non_empty() {
        assert!(
            !COMPLETION_DATA.fields.is_empty(),
            "Fields list must not be empty"
        );
    }

    #[test]
    fn fma_registry_has_entries() {
        assert!(
            FMA_REGISTRY.fma.len() > 200,
            "FMA registry should have 200+ apps, got {}",
            FMA_REGISTRY.fma.len()
        );
    }

    // ── Field queries ──────────────────────────────────────────

    #[test]
    fn fields_for_fleet_maintained_apps() {
        let fields = fields_for_context("fleet_maintained_apps");
        assert!(!fields.is_empty());
        assert!(fields.iter().any(|f| f.name == "slug"));
        assert!(fields.iter().any(|f| f.name == "self_service"));
    }

    #[test]
    fn fields_for_packages() {
        let fields = fields_for_context("packages");
        assert!(fields.iter().any(|f| f.name == "path" && f.required));
    }

    #[test]
    fn blocks_for_fleet_maintained_apps() {
        let blocks = blocks_for_context("fleet_maintained_apps");
        assert!(blocks.len() >= 3, "Should have macOS, Windows, ARM blocks");
    }

    #[test]
    fn globs_for_scripts() {
        let globs = globs_for_context("scripts");
        assert!(globs.len() >= 3, "Should have macOS, Windows, Linux globs");
        assert!(globs.iter().all(|g| g.pattern.contains("{base}")));
    }

    // ── FMA queries ────────────────────────────────────────────

    #[test]
    fn fma_slugs_for_darwin() {
        let slugs = fma_slugs_for_platform("darwin");
        assert!(slugs.contains(&"slack/darwin".to_string()));
        assert!(!slugs.contains(&"slack/windows".to_string()));
    }

    #[test]
    fn fma_slug_validation() {
        assert!(is_valid_fma_slug("slack/darwin"));
        assert!(is_valid_fma_slug("slack/windows"));
        assert!(!is_valid_fma_slug("slack/linux"));
        assert!(!is_valid_fma_slug("nonexistent/darwin"));
        assert!(!is_valid_fma_slug("invalid"));
    }

    #[test]
    fn fma_slug_suggestion() {
        // Prefix match
        assert_eq!(
            find_similar_fma_slug("sla"),
            Some("slack/darwin".to_string())
        );
        // Contains match
        assert!(find_similar_fma_slug("chrome").is_some());
    }

    // ── Snippet indentation validation ─────────────────────────

    #[test]
    fn all_snippets_have_correct_indentation() {
        for block in &COMPLETION_DATA.blocks {
            let snippet = block.snippet.strip_prefix('\n').unwrap_or(&block.snippet);
            let lines: Vec<&str> = snippet.lines().collect();
            assert!(
                !lines.is_empty(),
                "Block '{}' has empty snippet",
                block.name
            );

            // Line 1: must have 0 leading spaces
            let first_indent = lines[0].len() - lines[0].trim_start().len();
            assert_eq!(
                first_indent, 0,
                "Block '{}' line 1 has {} leading spaces (must be 0)",
                block.name, first_indent
            );

            for (i, line) in lines.iter().enumerate().skip(1) {
                if line.trim().is_empty() {
                    continue;
                }
                let indent = line.len() - line.trim_start().len();

                // Continuation lines must be indented
                assert!(
                    indent > 0,
                    "Block '{}' line {} has 0 indent (must be > 0)",
                    block.name,
                    i + 1
                );

                // Must be multiple of 2
                assert!(
                    indent % 2 == 0,
                    "Block '{}' line {} has {} spaces (not a multiple of 2)",
                    block.name,
                    i + 1,
                    indent
                );
            }
        }
    }

    #[test]
    fn all_fields_have_non_empty_names() {
        for field in &COMPLETION_DATA.fields {
            assert!(!field.name.is_empty(), "Field has empty name");
            assert!(
                !field.description.is_empty(),
                "Field '{}' has empty description",
                field.name
            );
            assert!(
                !field.context.is_empty(),
                "Field '{}' has empty context",
                field.name
            );
        }
    }

    #[test]
    fn all_globs_use_base_placeholder() {
        for glob in &COMPLETION_DATA.globs {
            // Labels globs are at repo root, not inside {base}
            if glob.context == "labels" {
                continue;
            }
            assert!(
                glob.pattern.contains("{base}"),
                "Glob '{}' in context '{}' doesn't use {{base}} placeholder",
                glob.pattern,
                glob.context
            );
        }
    }

    #[test]
    fn fma_no_duplicate_names() {
        let mut seen = std::collections::HashSet::new();
        for app in &FMA_REGISTRY.fma {
            assert!(seen.insert(&app.name), "Duplicate FMA entry: {}", app.name);
        }
    }

    #[test]
    fn fma_valid_platforms() {
        let valid = ["darwin", "windows"];
        for app in &FMA_REGISTRY.fma {
            for platform in &app.platforms {
                assert!(
                    valid.contains(&platform.as_str()),
                    "FMA '{}' has invalid platform '{}'",
                    app.name,
                    platform
                );
            }
        }
    }

    // ── Base path resolution ───────────────────────────────────

    #[test]
    fn resolve_base_in_pattern() {
        let resolved = resolve_base(
            "{base}/macos/scripts/*.sh",
            Path::new("/repo/fleets/workstations.yml"),
            Path::new("/repo"),
        );
        // Should contain the dir name, not {base}
        assert!(!resolved.contains("{base}"));
        assert!(resolved.contains("platforms") || resolved.contains("lib"));
    }
}
