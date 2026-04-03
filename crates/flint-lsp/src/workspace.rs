//! Workspace-level validation for Fleet GitOps YAML files.
//!
//! Provides cross-file validation including:
//! - Path reference validation (checking that referenced files exist)
//! - Go-to-definition for path references

use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentLink, GotoDefinitionResponse, Location, Position,
    Range, Url,
};

/// Check path references in a document and return diagnostics for invalid paths.
pub fn validate_path_references(
    source: &str,
    file_path: &Path,
    workspace_root: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim().trim_start_matches('-').trim();

        // Check for path: references
        if trimmed.starts_with("path:") {
            if let Some(path_value) = extract_path_value(trimmed) {
                // Calculate character positions for the path value
                let path_start = line.find(&path_value).unwrap_or(0) as u32;
                let path_end = path_start + path_value.len() as u32;
                let range = Range {
                    start: Position {
                        line: line_idx as u32,
                        character: path_start,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: path_end,
                    },
                };

                // Check for malformed path syntax first
                if let Some(msg) = check_malformed_path(&path_value) {
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("fleet-lint".to_string()),
                        message: msg,
                        ..Default::default()
                    });
                    continue;
                }

                // Determine base directory for resolution
                let base_dir = if let Some(root) = workspace_root {
                    root.to_path_buf()
                } else {
                    file_path.parent().unwrap_or(Path::new(".")).to_path_buf()
                };

                let resolved_path = base_dir.join(&path_value);

                if !resolved_path.exists() {
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("fleet-lint".to_string()),
                        message: format!("Referenced file not found: {}", path_value),
                        ..Default::default()
                    });
                }
            }
        }
    }

    diagnostics
}

/// Extract path value from a line like "path: lib/policies.yml"
fn extract_path_value(line: &str) -> Option<String> {
    let value = line.strip_prefix("path:")?.trim();
    // Remove quotes if present
    let value = value.trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Check if a path value is malformed and return an error message if so.
fn check_malformed_path(path: &str) -> Option<String> {
    // Shell source command prefix: `. ./script.sh` or `source ./script.sh`
    if path.starts_with(". ") {
        let suggested = path
            .strip_prefix(". ")
            .expect("starts_with checked above")
            .trim();
        return Some(format!(
            "Path starts with `. ` (shell source command). Did you mean `{}`?",
            suggested
        ));
    }
    if path.starts_with("source ") {
        let suggested = path
            .strip_prefix("source ")
            .expect("starts_with checked above")
            .trim();
        return Some(format!(
            "Path starts with `source ` (shell command). Did you mean `{}`?",
            suggested
        ));
    }

    // Shell execution prefixes: `bash ./script.sh`, `sh ./script.sh`, etc.
    for prefix in &["bash ", "sh ", "zsh ", "/bin/bash ", "/bin/sh "] {
        if path.starts_with(prefix) {
            let suggested = path
                .strip_prefix(prefix)
                .expect("starts_with checked above")
                .trim();
            return Some(format!(
                "Path starts with `{}` (shell command). Use the path only: `{}`",
                prefix.trim(),
                suggested
            ));
        }
    }

    // Absolute paths (should be relative)
    if path.starts_with('/') {
        return Some(
            "Path is absolute. Use a relative path from the gitops repo root.".to_string(),
        );
    }

    // Path traversal outside repo — only flag ../../ (two levels up) or deeper,
    // as ../ is normal for fleet YAML referencing sibling directories
    // (e.g., fleets/workstations.yml -> ../platforms/macos/policies/*.yml)
    if path.starts_with("../../") || path.contains("/../../") {
        return Some(
            "Path traverses multiple levels up. Verify it stays within the repo root.".to_string(),
        );
    }

    None
}

/// Get go-to-definition location for path references.
pub fn get_path_definition(
    source: &str,
    position: Position,
    file_path: &Path,
    workspace_root: Option<&Path>,
) -> Option<GotoDefinitionResponse> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let trimmed = line.trim().trim_start_matches('-').trim();

    // Check if cursor is on a path: reference
    if !trimmed.starts_with("path:") {
        return None;
    }

    let path_value = extract_path_value(trimmed)?;

    // Check if cursor is actually on the path value (not the key)
    let value_start = line.find(&path_value)? as u32;
    let value_end = value_start + path_value.len() as u32;

    if position.character < value_start || position.character > value_end {
        return None;
    }

    // Resolve the path
    let base_dir = if let Some(root) = workspace_root {
        root.to_path_buf()
    } else {
        file_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let resolved_path = base_dir.join(&path_value);

    if !resolved_path.exists() {
        return None;
    }

    // Convert to URI
    let uri = Url::from_file_path(&resolved_path).ok()?;

    Some(GotoDefinitionResponse::Scalar(Location {
        uri,
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
    }))
}

/// Find all files in a workspace that are Fleet GitOps YAML files.
pub fn find_fleet_files(workspace_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(workspace_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_fleet_yaml(&path) {
                files.push(path);
            } else if path.is_dir() {
                // Recursively scan subdirectories
                files.extend(find_fleet_files(&path));
            }
        }
    }

    files
}

/// Check if a file is likely a Fleet GitOps YAML file.
fn is_fleet_yaml(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if ext != "yml" && ext != "yaml" {
            return false;
        }

        // Check for common Fleet file patterns
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Common Fleet GitOps file names
            if name == "default.yml"
                || name == "team.yml"
                || name.contains("policies")
                || name.contains("queries")
                || name.contains("labels")
            {
                return true;
            }
        }

        // Check if it's in a known Fleet directory
        if let Some(parent) = path.parent() {
            let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                parent_name,
                "fleets" | "teams" | "lib" | "labels" | "platforms"
            ) {
                return true;
            }

            // Check grandparent for nested directories (fleets/*, platforms/*)
            if let Some(grandparent) = parent.parent() {
                let grandparent_name = grandparent
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if matches!(grandparent_name, "fleets" | "teams" | "platforms") {
                    return true;
                }

                // Check great-grandparent for platforms/*/labels/, platforms/*/lib/
                if grandparent_name == "labels" || grandparent_name == "lib" {
                    if let Some(ggp) = grandparent.parent() {
                        if let Some(ggp_parent) = ggp.parent() {
                            let ggp_name = ggp_parent
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if ggp_name == "platforms" {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Fall back to checking file content (first few lines)
        if let Ok(content) = std::fs::read_to_string(path) {
            let first_lines: String = content.lines().take(10).collect::<Vec<_>>().join("\n");
            return first_lines.contains("policies:")
                || first_lines.contains("queries:")
                || first_lines.contains("reports:")
                || first_lines.contains("labels:")
                || first_lines.contains("agent_options:")
                || first_lines.contains("controls:");
        }
    }

    false
}

/// Generate document links for all path: references in a document.
///
/// Makes `path:` values clickable in the editor, navigating to the referenced file.
pub fn document_links(
    source: &str,
    file_path: &Path,
    workspace_root: Option<&Path>,
) -> Vec<DocumentLink> {
    let refs = extract_path_references(source, file_path);
    let lines: Vec<&str> = source.lines().collect();

    refs.into_iter()
        .filter_map(|path_ref| {
            let line = lines.get(path_ref.line)?;
            let path_start = line.find(&path_ref.path_value)? as u32;
            let path_end = path_start + path_ref.path_value.len() as u32;

            // Resolve against workspace root if available, otherwise file parent
            let base_dir =
                workspace_root.unwrap_or_else(|| file_path.parent().unwrap_or(Path::new(".")));
            let resolved = base_dir.join(&path_ref.path_value);
            let target = Url::from_file_path(&resolved).ok()?;

            Some(DocumentLink {
                range: Range {
                    start: Position {
                        line: path_ref.line as u32,
                        character: path_start,
                    },
                    end: Position {
                        line: path_ref.line as u32,
                        character: path_end,
                    },
                },
                target: Some(target),
                tooltip: Some(format!("Open {}", path_ref.path_value)),
                data: None,
            })
        })
        .collect()
}

/// PathReference represents a reference from one file to another.
#[derive(Debug, Clone)]
pub struct PathReference {
    /// Source file containing the reference
    pub source_file: PathBuf,
    /// Line number in source file (0-indexed)
    pub line: usize,
    /// The path value as written in the file
    pub path_value: String,
    /// Resolved absolute path (if resolvable)
    pub resolved_path: Option<PathBuf>,
}

/// Extract all path references from a document.
pub fn extract_path_references(source: &str, file_path: &Path) -> Vec<PathReference> {
    let mut refs = Vec::new();
    let base_dir = file_path.parent().unwrap_or(Path::new("."));

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim().trim_start_matches('-').trim();

        if trimmed.starts_with("path:") {
            if let Some(path_value) = extract_path_value(trimmed) {
                let resolved = base_dir.join(&path_value);
                refs.push(PathReference {
                    source_file: file_path.to_path_buf(),
                    line: line_idx,
                    path_value: path_value.clone(),
                    resolved_path: if resolved.exists() {
                        Some(resolved)
                    } else {
                        None
                    },
                });
            }
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_path_value() {
        assert_eq!(
            extract_path_value("path: lib/policies.yml"),
            Some("lib/policies.yml".to_string())
        );
        assert_eq!(
            extract_path_value("path: \"lib/policies.yml\""),
            Some("lib/policies.yml".to_string())
        );
        assert_eq!(extract_path_value("path:"), None);
        assert_eq!(extract_path_value("name: test"), None);
    }

    #[test]
    fn test_validate_path_references() {
        let temp_dir = TempDir::new().unwrap();

        // Create a referenced file
        let lib_dir = temp_dir.path().join("lib");
        fs::create_dir(&lib_dir).unwrap();
        fs::write(lib_dir.join("policies.yml"), "policies:\n  - name: test").unwrap();

        let source = r#"policies:
  - path: lib/policies.yml
  - path: lib/missing.yml
"#;

        let main_file = temp_dir.path().join("default.yml");
        fs::write(&main_file, source).unwrap();

        let diagnostics = validate_path_references(source, &main_file, Some(temp_dir.path()));

        // Should have 1 diagnostic for the missing file
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("missing.yml"));
    }

    #[test]
    fn test_check_malformed_path() {
        // Shell source prefix
        let msg = check_malformed_path(". ./lib/scripts/uninstall.sh");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("./lib/scripts/uninstall.sh"));

        // `source` prefix
        let msg = check_malformed_path("source ./lib/scripts/uninstall.sh");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("./lib/scripts/uninstall.sh"));

        // Shell interpreter prefix
        let msg = check_malformed_path("bash ./lib/scripts/uninstall.sh");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("./lib/scripts/uninstall.sh"));

        // Absolute path
        let msg = check_malformed_path("/usr/local/bin/script.sh");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("absolute"));

        // Path traversal
        let msg = check_malformed_path("../../etc/passwd");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("traverses"));

        // Valid relative path — no error
        assert!(check_malformed_path("lib/scripts/uninstall.sh").is_none());
        assert!(check_malformed_path("./lib/scripts/uninstall.sh").is_none());
    }

    #[test]
    fn test_malformed_path_diagnostic() {
        let source = r#"controls:
  scripts:
    - path: . ./lib/macos/scripts/_uninstall-santa.sh
"#;
        let diagnostics = validate_path_references(source, Path::new("/fake/team.yml"), None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("shell source command"));
        assert!(diagnostics[0]
            .message
            .contains("./lib/macos/scripts/_uninstall-santa.sh"));
    }

    #[test]
    fn test_extract_path_references() {
        let source = r#"policies:
  - path: lib/policies.yml
  - name: Local Policy
    query: SELECT 1
  - path: lib/more-policies.yml
"#;

        let refs = extract_path_references(source, Path::new("/fake/default.yml"));

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].path_value, "lib/policies.yml");
        assert_eq!(refs[0].line, 1);
        assert_eq!(refs[1].path_value, "lib/more-policies.yml");
        assert_eq!(refs[1].line, 4);
    }
}
