//! Conversion utilities from LintError to LSP Diagnostic.

use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString, Position,
    Range, Url,
};

use super::fleet::GitOpsError;
use super::position::to_lsp_position;
use flint_lint::error::{LintError, Severity};

/// Map a rule code to its Fleet documentation URL anchor.
fn doc_url_for_code(code: &str) -> Option<Url> {
    let anchor = match code {
        "required-fields" => "gitops",
        "basic-validation" | "platform-compatibility" => "policies",
        "query-syntax" => "reports",
        "structural" | "structural-validation" | "misplaced-key" => "gitops",
        "deprecation" | "deprecated-keys" => "gitops",
        "duplicate-names" => "gitops",
        "interval-validation" => "reports",
        "security" => "policies",
        "self-reference" => "gitops",
        "label-targeting" => "policies",
        "label-membership" => "labels",
        "date-format" => "macos_updates",
        "hash-format" => "packages",
        "categories" => "self_service-labels-categories-and-setup_experience",
        "file-extension" => "controls",
        "secret-hygiene" => "policies",
        "type-validation" => "policies",
        "yaml-syntax" | "yaml-tabs" | "yaml-duplicate-key" => "gitops",
        _ => return None,
    };
    Url::parse(&format!(
        "https://fleetdm.com/docs/configuration/yaml-files#{}",
        anchor
    ))
    .ok()
}

/// Convert a LintError to an LSP Diagnostic.
pub fn lint_error_to_diagnostic(error: &LintError, source: &str) -> Diagnostic {
    let range = error_to_range(error, source);
    let severity = match error.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    };

    let message = format_message(error);

    // Include suggestion in data for code actions
    let data = error.suggestion.as_ref().map(|s| {
        serde_json::json!({
            "suggestion": s,
            "help": error.help
        })
    });

    // Add DEPRECATED tag for deprecation diagnostics (gives strikethrough in editors)
    let tags = if error.message.contains("deprecated") || error.message.contains("was removed") {
        Some(vec![DiagnosticTag::DEPRECATED])
    } else {
        None
    };

    // Map rule code to diagnostic code + documentation link
    let code = error
        .rule_code
        .as_ref()
        .map(|c| NumberOrString::String(c.clone()));
    let code_description = error
        .rule_code
        .as_ref()
        .and_then(|c| doc_url_for_code(c))
        .map(|href| CodeDescription { href });

    Diagnostic {
        range,
        severity: Some(severity),
        code,
        code_description,
        source: Some("fleet-lint".to_string()),
        message,
        related_information: None,
        tags,
        data,
    }
}

/// Convert error location to LSP Range.
fn error_to_range(error: &LintError, source: &str) -> Range {
    match (error.line, error.column) {
        (Some(line), Some(col)) => {
            let start = to_lsp_position(line, col, source);
            // Estimate end position - highlight the word/context if available
            let end_col = col + error.context.as_ref().map(|c| c.len()).unwrap_or(1);
            let end = to_lsp_position(line, end_col, source);
            Range { start, end }
        }
        (Some(line), None) => {
            // Highlight the entire line
            let start = Position {
                line: (line.saturating_sub(1)) as u32,
                character: 0,
            };
            let line_content = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
            let end = Position {
                line: (line.saturating_sub(1)) as u32,
                character: line_content.len() as u32,
            };
            Range { start, end }
        }
        _ => {
            // No location - highlight first line
            Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            }
        }
    }
}

/// Convert a GitOps dry-run error to an LSP Diagnostic.
///
/// GitOps errors are not line-specific (they come from the server),
/// so they always appear at line 0 with source "fleet-gitops".
pub fn gitops_error_to_diagnostic(error: &GitOpsError) -> Diagnostic {
    let message = if let Some(hint) = &error.hint {
        format!("{}\n\n→ {}", error.message, hint)
    } else {
        error.message.clone()
    };
    Diagnostic {
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
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("fleet-gitops".to_string()),
        message,
        ..Default::default()
    }
}

/// Format the diagnostic message with help text.
fn format_message(error: &LintError) -> String {
    let mut msg = error.message.clone();

    if let Some(help) = &error.help {
        msg.push_str("\n\nHelp: ");
        msg.push_str(help);
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_lint_error_to_diagnostic() {
        let error = LintError {
            severity: Severity::Error,
            message: "Missing required field 'query'".to_string(),
            file: PathBuf::from("test.yml"),
            line: Some(5),
            column: Some(3),
            context: Some("name".to_string()),
            help: Some("Policies must have a query field".to_string()),
            suggestion: Some("query: \"SELECT 1;\"".to_string()),
            rule_code: Some("required-fields".to_string()),
            fix_safety: None,
        };

        let source = "policies:\n  - name: test\n    platform: darwin\n";
        let diagnostic = lint_error_to_diagnostic(&error, source);

        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("fleet-lint".to_string()));
        assert!(diagnostic.message.contains("Missing required field"));
        assert!(diagnostic.message.contains("Help:"));
        assert!(diagnostic.data.is_some());

        // Verify diagnostic code and doc link
        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("required-fields".to_string()))
        );
        assert!(diagnostic.code_description.is_some());
        let desc = diagnostic.code_description.unwrap();
        assert!(desc
            .href
            .as_str()
            .contains("fleetdm.com/docs/configuration/yaml-files"));
    }
}
