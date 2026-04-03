//! Semantic validation rules for Fleet GitOps YAML.
//!
//! These rules validate domain-specific constraints that go beyond structural
//! schema validation — mutual exclusivity, format rules, file extensions, etc.

use std::path::Path;

use super::error::{LintError, Severity};
use super::fleet_config::FleetConfig;
use super::rules::Rule;
use super::yaml_utils::*;

// ============================================================================
// Rule 1: Label Targeting Mutual Exclusivity
// ============================================================================

/// Validates that `labels_include_any` and `labels_include_all` are not both set
/// on the same item. `labels_exclude_any` can coexist with either.
pub struct LabelTargetingRule;

impl Rule for LabelTargetingRule {
    fn name(&self) -> &'static str {
        "label-targeting"
    }
    fn description(&self) -> &'static str {
        "Checks that labels_include_any and labels_include_all are not both specified"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://fleetdm.com/docs/configuration/yaml-files#policies")
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        // All paths where label targeting can appear
        let paths: &[&[&str]] = &[
            &["policies"],
            &["queries"],
            &["reports"],
            &["software", "packages"],
            &["software", "app_store_apps"],
            &["software", "fleet_maintained_apps"],
            &["controls", "scripts"],
        ];

        for path in paths {
            for item in collect_items_at_path(&yaml, path) {
                let has_any = mapping_has_key(item, "labels_include_any");
                let has_all = mapping_has_key(item, "labels_include_all");

                if has_any && has_all {
                    let name = item_display_name(item);
                    errors.push(
                        LintError::error(
                            format!(
                                "'{}' has both labels_include_any and labels_include_all — only one is allowed",
                                name
                            ),
                            file,
                        )
                        .with_help("Use labels_include_any to match hosts with ANY label, or labels_include_all to match hosts with ALL labels")
                    );
                }
            }
        }

        errors
    }
}

// ============================================================================
// Rule 2: Label Membership Type Consistency
// ============================================================================

/// Validates label membership type constraints:
/// - `dynamic`: requires `query`, forbids `hosts`/`criteria`
/// - `manual`: requires `hosts`, forbids `query`/`criteria`
/// - `host_vitals`: requires `criteria`, forbids `query`/`hosts`
pub struct LabelMembershipRule;

impl Rule for LabelMembershipRule {
    fn name(&self) -> &'static str {
        "label-membership"
    }
    fn description(&self) -> &'static str {
        "Checks label membership type consistency (dynamic→query, manual→hosts, host_vitals→criteria)"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://fleetdm.com/docs/configuration/yaml-files#labels")
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        for item in collect_items_at_path(&yaml, &["labels"]) {
            // Skip path/glob references
            if (mapping_has_key(item, "path") || mapping_has_key(item, "paths"))
                && !mapping_has_key(item, "name")
            {
                continue;
            }

            let name = item_display_name(item);
            let membership_type =
                mapping_get_str(item, "label_membership_type").unwrap_or("dynamic"); // Fleet default

            let has_query = mapping_has_key(item, "query");
            let has_hosts = mapping_has_key(item, "hosts");
            let has_criteria = mapping_has_key(item, "criteria");

            match membership_type {
                "dynamic" => {
                    if !has_query {
                        errors.push(
                            LintError::error(
                                format!("Label '{}' is dynamic but missing 'query' field", name),
                                file,
                            )
                            .with_help("Dynamic labels require a SQL query to determine membership")
                            .with_suggestion("query: \"SELECT 1 FROM ...;\""),
                        );
                    }
                    if has_hosts {
                        errors.push(
                            LintError::error(
                                format!("Label '{}' is dynamic but has 'hosts' field", name),
                                file,
                            )
                            .with_help("Dynamic labels use 'query' for membership, not 'hosts'. Use label_membership_type: manual for host lists"),
                        );
                    }
                }
                "manual" => {
                    if has_query {
                        errors.push(
                            LintError::error(
                                format!("Label '{}' is manual but has 'query' field", name),
                                file,
                            )
                            .with_help("Manual labels use 'hosts' for membership, not 'query'. Use label_membership_type: dynamic for SQL queries"),
                        );
                    }
                }
                "host_vitals" => {
                    if !has_criteria {
                        errors.push(
                            LintError::error(
                                format!("Label '{}' is host_vitals but missing 'criteria' field", name),
                                file,
                            )
                            .with_help("host_vitals labels require 'criteria' with 'vital' and 'value' fields"),
                        );
                    }
                    if has_query {
                        errors.push(
                            LintError::error(
                                format!("Label '{}' is host_vitals but has 'query' field", name),
                                file,
                            )
                            .with_help("host_vitals labels use 'criteria', not 'query'"),
                        );
                    }
                }
                _ => {}
            }
        }

        errors
    }
}

// ============================================================================
// Rule 3: Date Format Validation
// ============================================================================

/// Validates that `deadline` fields match YYYY-MM-DD format.
pub struct DateFormatRule;

impl Rule for DateFormatRule {
    fn name(&self) -> &'static str {
        "date-format"
    }
    fn description(&self) -> &'static str {
        "Checks that deadline fields use YYYY-MM-DD format"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn is_fixable(&self) -> bool {
        true
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://fleetdm.com/docs/configuration/yaml-files#macos_updates")
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        let update_paths: &[&[&str]] = &[
            &["controls", "macos_updates"],
            &["controls", "ios_updates"],
            &["controls", "ipados_updates"],
        ];

        for path in update_paths {
            // Walk to the updates mapping
            let mut current = &yaml;
            let mut found = true;
            for &key in *path {
                match current {
                    serde_yaml::Value::Mapping(map) => {
                        match map.get(serde_yaml::Value::String(key.to_string())) {
                            Some(v) => current = v,
                            None => {
                                found = false;
                                break;
                            }
                        }
                    }
                    _ => {
                        found = false;
                        break;
                    }
                }
            }

            if !found {
                continue;
            }

            if let Some(deadline) = mapping_get_str(current, "deadline") {
                if !is_valid_date(deadline) {
                    let section = path.last().unwrap_or(&"updates");
                    errors.push(
                        LintError::error(
                            format!(
                                "{}: deadline '{}' is not a valid YYYY-MM-DD date",
                                section, deadline
                            ),
                            file,
                        )
                        .with_help("Deadline must be in YYYY-MM-DD format (e.g., 2025-06-15)")
                        .with_suggestion("deadline: \"2025-06-15\""),
                    );
                }
            }
        }

        errors
    }
}

/// Validate a date string matches YYYY-MM-DD and is a real date.
fn is_valid_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    let year: u32 = match parts[0].parse() {
        Ok(y) if (2000..=2100).contains(&y) => y,
        _ => return false,
    };
    let month: u32 = match parts[1].parse() {
        Ok(m) if (1..=12).contains(&m) => m,
        _ => return false,
    };
    let day: u32 = match parts[2].parse() {
        Ok(d) if d >= 1 => d,
        _ => return false,
    };

    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };

    day <= max_day
}

// ============================================================================
// Rule 4: Hash Format Validation
// ============================================================================

/// Validates that `hash_sha256` values are 64 lowercase hex characters.
pub struct HashFormatRule;

impl Rule for HashFormatRule {
    fn name(&self) -> &'static str {
        "hash-format"
    }
    fn description(&self) -> &'static str {
        "Checks that hash_sha256 values are valid 64-character lowercase hex strings"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn is_fixable(&self) -> bool {
        true
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://fleetdm.com/docs/configuration/yaml-files#packages")
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        // Check software.packages[].hash_sha256
        for item in collect_items_at_path(&yaml, &["software", "packages"]) {
            if let Some(hash) = mapping_get_str(item, "hash_sha256") {
                check_hash(hash, &item_display_name(item), file, &mut errors);
            }
        }

        // Check policies[].install_software.hash_sha256
        for item in collect_items_at_path(&yaml, &["policies"]) {
            if let Some(install) = item
                .as_mapping()
                .and_then(|m| m.get(serde_yaml::Value::String("install_software".to_string())))
            {
                if let Some(hash) = mapping_get_str(install, "hash_sha256") {
                    check_hash(hash, &item_display_name(item), file, &mut errors);
                }
            }
        }

        errors
    }
}

fn check_hash(hash: &str, item_name: &str, file: &Path, errors: &mut Vec<LintError>) {
    if hash.len() != 64 {
        errors.push(
            LintError::error(
                format!(
                    "'{}': hash_sha256 must be exactly 64 characters (got {})",
                    item_name,
                    hash.len()
                ),
                file,
            )
            .with_help("SHA256 hashes are 64 lowercase hexadecimal characters"),
        );
        return;
    }

    if !hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        if hash.chars().all(|c| c.is_ascii_hexdigit()) {
            // Uppercase hex — suggest lowercase
            errors.push(
                LintError::error(
                    format!("'{}': hash_sha256 must be lowercase hex", item_name),
                    file,
                )
                .with_suggestion(hash.to_lowercase())
                .with_fix_safety(super::error::FixSafety::Safe),
            );
        } else {
            errors.push(
                LintError::error(
                    format!("'{}': hash_sha256 contains invalid characters", item_name),
                    file,
                )
                .with_help("SHA256 hashes must contain only characters 0-9 and a-f"),
            );
        }
    }
}

// ============================================================================
// Rule 5: Categories Validation
// ============================================================================

const VALID_CATEGORIES: &[&str] = &[
    "Browsers",
    "Communication",
    "Developer tools",
    "Productivity",
    "Security",
    "Utilities",
];

/// Validates that `categories` values are from the supported set.
pub struct CategoriesRule;

impl Rule for CategoriesRule {
    fn name(&self) -> &'static str {
        "categories"
    }
    fn description(&self) -> &'static str {
        "Checks that software category values are from the supported set"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn is_fixable(&self) -> bool {
        true
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        let paths: &[&[&str]] = &[
            &["software", "packages"],
            &["software", "app_store_apps"],
            &["software", "fleet_maintained_apps"],
        ];

        for path in paths {
            for item in collect_items_at_path(&yaml, path) {
                let name = item_display_name(item);
                for cat in mapping_get_string_array(item, "categories") {
                    if !VALID_CATEGORIES.contains(&cat) {
                        let suggestion = find_similar_category(cat);
                        let mut err = LintError::warning(
                            format!("'{}': unknown category '{}'", name, cat),
                            file,
                        )
                        .with_help(format!("Valid categories: {}", VALID_CATEGORIES.join(", ")));
                        if let Some(s) = suggestion {
                            err = err.with_suggestion(s.to_string());
                        }
                        errors.push(err);
                    }
                }
            }
        }

        errors
    }
}

fn find_similar_category(input: &str) -> Option<&'static str> {
    let input_lower = input.to_lowercase();
    for cat in VALID_CATEGORIES {
        if cat.to_lowercase() == input_lower {
            return Some(cat); // Case mismatch
        }
        if cat.to_lowercase().contains(&input_lower) || input_lower.contains(&cat.to_lowercase()) {
            return Some(cat);
        }
    }
    // Common aliases
    match input_lower.as_str() {
        "browser" | "web" => Some("Browsers"),
        "chat" | "messaging" | "comms" => Some("Communication"),
        "dev" | "developer" | "development" | "tools" | "devtools" => Some("Developer tools"),
        "office" | "work" => Some("Productivity"),
        "privacy" | "antivirus" | "firewall" => Some("Security"),
        "utility" | "utils" => Some("Utilities"),
        _ => None,
    }
}

// ============================================================================
// Rule 6: File Extension Validation
// ============================================================================

/// Validates that profile/script paths have correct file extensions.
pub struct FileExtensionRule;

impl Rule for FileExtensionRule {
    fn name(&self) -> &'static str {
        "file-extension"
    }
    fn description(&self) -> &'static str {
        "Checks that MDM profile and script paths have valid file extensions"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://fleetdm.com/docs/configuration/yaml-files#controls")
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        let checks: &[(&[&str], &[&str], &str)] = &[
            (
                &["controls", "macos_settings", "custom_settings"],
                &[".mobileconfig", ".json"],
                "macOS profiles",
            ),
            (
                &["controls", "windows_settings", "custom_settings"],
                &[".xml"],
                "Windows profiles",
            ),
            (
                &["controls", "android_settings", "custom_settings"],
                &[".json"],
                "Android profiles",
            ),
            (
                &["controls", "scripts"],
                &[".sh", ".ps1", ".zsh"],
                "scripts",
            ),
        ];

        for (path, valid_exts, context) in checks {
            for item in collect_items_at_path(&yaml, path) {
                if let Some(path_val) = mapping_get_str(item, "path") {
                    if !valid_exts.iter().any(|ext| path_val.ends_with(ext)) {
                        errors.push(
                            LintError::warning(
                                format!("{}: '{}' has unexpected extension", context, path_val),
                                file,
                            )
                            .with_help(format!(
                                "Expected extensions for {}: {}",
                                context,
                                valid_exts.join(", ")
                            )),
                        );
                    }
                }
            }
        }

        errors
    }
}

// ============================================================================
// Rule 7: Secret Hygiene
// ============================================================================

/// Checks that integration credential fields use environment variable references.
pub struct SecretHygieneRule;

impl Rule for SecretHygieneRule {
    fn name(&self) -> &'static str {
        "secret-hygiene"
    }
    fn description(&self) -> &'static str {
        "Checks that API tokens and secrets use environment variable references ($VAR)"
    }
    fn category(&self) -> &'static str {
        "security"
    }
    fn is_fixable(&self) -> bool {
        true
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        // integrations.jira[].api_token
        check_secret_field(
            &yaml,
            &["integrations", "jira"],
            "api_token",
            file,
            &mut errors,
        );
        check_secret_field(
            &yaml,
            &["org_settings", "integrations", "jira"],
            "api_token",
            file,
            &mut errors,
        );

        // integrations.zendesk[].api_token
        check_secret_field(
            &yaml,
            &["integrations", "zendesk"],
            "api_token",
            file,
            &mut errors,
        );
        check_secret_field(
            &yaml,
            &["org_settings", "integrations", "zendesk"],
            "api_token",
            file,
            &mut errors,
        );

        // integrations.google_calendar[].api_key_json
        check_secret_field(
            &yaml,
            &["integrations", "google_calendar"],
            "api_key_json",
            file,
            &mut errors,
        );
        check_secret_field(
            &yaml,
            &["org_settings", "integrations", "google_calendar"],
            "api_key_json",
            file,
            &mut errors,
        );

        errors
    }
}

// ============================================================================
// Rule 8: Path / Paths Reference Validation
// ============================================================================

/// Validates path/paths fields on entities (policies, reports, labels, scripts, etc.):
/// - `path` must NOT contain glob characters (`*?[{`)
/// - `paths` MUST contain glob characters
/// - Cannot have both `path` and `paths` on the same entry
/// - Scripts require `path` or `paths` (no inline allowed)
pub struct PathReferenceRule;

impl Rule for PathReferenceRule {
    fn name(&self) -> &'static str {
        "path-reference"
    }
    fn description(&self) -> &'static str {
        "Validates path/paths fields: glob usage, mutual exclusivity, and script requirements"
    }
    fn category(&self) -> &'static str {
        "semantic"
    }

    fn check(&self, _config: &FleetConfig, file: &Path, source: &str) -> Vec<LintError> {
        let yaml = match parse_yaml(source) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut errors = Vec::new();

        // Check all sections that support path/paths references
        let sections = &["policies", "reports", "queries", "labels"];
        for section in sections {
            for item in collect_items_at_path(&yaml, &[section]) {
                check_path_fields(item, section, false, file, source, &mut errors);
            }
        }

        // Scripts require path or paths (no inline)
        for item in collect_items_at_path(&yaml, &["controls", "scripts"]) {
            check_path_fields(item, "script", true, file, source, &mut errors);
        }

        // Custom settings (profiles) also use path/paths
        for section in &["macos_settings", "windows_settings", "android_settings"] {
            let paths = &["controls", section, "custom_settings"];
            for item in collect_items_at_path(&yaml, paths) {
                check_path_fields(item, "profile", false, file, source, &mut errors);
            }
        }

        errors
    }
}

/// Returns true if the string contains glob metacharacters.
fn contains_glob_meta(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

fn check_path_fields(
    item: &serde_yaml::Value,
    entity_type: &str,
    require_file_ref: bool,
    file: &Path,
    source: &str,
    errors: &mut Vec<LintError>,
) {
    let has_path = mapping_get_str(item, "path").is_some();
    let has_paths = mapping_get_str(item, "paths").is_some();

    // Can't have both path and paths
    if has_path && has_paths {
        let name = item_display_name(item);
        let mut err = LintError::error(
            format!("{entity_type} '{name}' has both 'path' and 'paths' — use one or the other"),
            file,
        )
        .with_help("'path' is for a single file, 'paths' is for glob patterns");

        if let Some(line) = find_key_line(source, "paths", 0) {
            err = err.with_location(line, 0);
        }
        errors.push(err);
        return;
    }

    // path must NOT contain glob characters
    if let Some(path_val) = mapping_get_str(item, "path") {
        if contains_glob_meta(path_val) {
            let name = item_display_name(item);
            let mut err = LintError::error(
                format!("{entity_type} '{name}' 'path' contains glob characters — use 'paths' for glob patterns"),
                file,
            )
            .with_help(format!("Change 'path: {path_val}' to 'paths: {path_val}'"))
            .with_context(path_val.to_string())
            .with_suggestion(format!("paths: {path_val}"));

            if let Some(line) = find_key_line(source, "path", 0) {
                err = err.with_location(line, 0);
            }
            errors.push(err);
        }
    }

    // paths MUST contain glob characters
    if let Some(paths_val) = mapping_get_str(item, "paths") {
        if !contains_glob_meta(paths_val) {
            let name = item_display_name(item);
            let mut err = LintError::error(
                format!("{entity_type} '{name}' 'paths' does not contain glob characters — use 'path' for a specific file"),
                file,
            )
            .with_help(format!("Change 'paths: {paths_val}' to 'path: {paths_val}'"))
            .with_context(paths_val.to_string())
            .with_suggestion(format!("path: {paths_val}"));

            if let Some(line) = find_key_line(source, "paths", 0) {
                err = err.with_location(line, 0);
            }
            errors.push(err);
        }
    }

    // Scripts require path or paths (no inline)
    if require_file_ref && !has_path && !has_paths {
        let name = item_display_name(item);
        let mut err = LintError::error(
            format!("{entity_type} '{name}' has no 'path' or 'paths' field — scripts must reference a file"),
            file,
        ).with_help("Add 'path: ./path/to/script.sh' or 'paths: ./scripts/*.sh'");

        // Try to find the line of this item
        if let Some(name_str) = mapping_get_str(item, "name") {
            if let Some(line) = find_key_line(source, name_str, 0) {
                err = err.with_location(line, 0);
            }
        }
        errors.push(err);
    }
}

fn check_secret_field(
    yaml: &serde_yaml::Value,
    path: &[&str],
    field: &str,
    file: &Path,
    errors: &mut Vec<LintError>,
) {
    for item in collect_items_at_path(yaml, path) {
        if let Some(value) = mapping_get_str(item, field) {
            // Skip empty, env var refs, and 1Password refs
            if value.is_empty() || value.starts_with('$') || value.starts_with("op://") {
                continue;
            }
            errors.push(
                LintError::warning(
                    format!(
                        "Integration '{}' field contains a plain-text value",
                        field
                    ),
                    file,
                )
                .with_help("Use an environment variable ($VAR) or 1Password reference (op://...) for secrets")
                .with_suggestion(format!("${}", field.to_uppercase())),
            );
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn lint(rule: &dyn Rule, source: &str) -> Vec<LintError> {
        let config: FleetConfig = serde_yaml::from_str(source).unwrap_or_default();
        rule.check(&config, &PathBuf::from("test.yml"), source)
    }

    // -- LabelTargetingRule --

    #[test]
    fn test_label_targeting_valid() {
        let errors = lint(
            &LabelTargetingRule,
            "policies:\n  - name: test\n    labels_include_any:\n      - Engineering\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_label_targeting_exclude_with_include_any_valid() {
        let errors = lint(
            &LabelTargetingRule,
            "policies:\n  - name: test\n    labels_include_any:\n      - Eng\n    labels_exclude_any:\n      - QA\n",
        );
        // labels_exclude_any can coexist with labels_include_any
        assert!(errors.is_empty());
    }

    #[test]
    fn test_label_targeting_mutual_exclusion() {
        let errors = lint(
            &LabelTargetingRule,
            "policies:\n  - name: test\n    labels_include_any:\n      - Eng\n    labels_include_all:\n      - QA\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0]
            .message
            .contains("labels_include_any and labels_include_all"));
    }

    // -- LabelMembershipRule --

    #[test]
    fn test_label_membership_dynamic_valid() {
        let errors = lint(
            &LabelMembershipRule,
            "labels:\n  - name: test\n    label_membership_type: dynamic\n    query: \"SELECT 1\"\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_label_membership_manual_with_query() {
        let errors = lint(
            &LabelMembershipRule,
            "labels:\n  - name: test\n    label_membership_type: manual\n    query: \"SELECT 1\"\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("manual but has 'query'"));
    }

    #[test]
    fn test_label_membership_dynamic_missing_query() {
        let errors = lint(
            &LabelMembershipRule,
            "labels:\n  - name: test\n    label_membership_type: dynamic\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("missing 'query'"));
    }

    // -- DateFormatRule --

    #[test]
    fn test_date_format_valid() {
        let errors = lint(
            &DateFormatRule,
            "controls:\n  macos_updates:\n    deadline: \"2025-06-15\"\n    minimum_version: \"15.1\"\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_date_format_invalid() {
        let errors = lint(
            &DateFormatRule,
            "controls:\n  macos_updates:\n    deadline: \"15-06-2025\"\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not a valid YYYY-MM-DD"));
    }

    #[test]
    fn test_date_format_invalid_month() {
        let errors = lint(
            &DateFormatRule,
            "controls:\n  macos_updates:\n    deadline: \"2025-13-01\"\n",
        );
        assert_eq!(errors.len(), 1);
    }

    // -- HashFormatRule --

    #[test]
    fn test_hash_format_valid() {
        let errors = lint(
            &HashFormatRule,
            "software:\n  packages:\n    - path: foo.yml\n      hash_sha256: fd22528a87f3cfdb81aca981953aa5c8d7084581b9209bb69abf69c09a0afaaf\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_hash_format_uppercase() {
        let errors = lint(
            &HashFormatRule,
            "software:\n  packages:\n    - path: foo.yml\n      hash_sha256: FD22528A87F3CFDB81ACA981953AA5C8D7084581B9209BB69ABF69C09A0AFAAF\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("lowercase"));
        assert!(errors[0].suggestion.is_some());
    }

    #[test]
    fn test_hash_format_wrong_length() {
        let errors = lint(
            &HashFormatRule,
            "software:\n  packages:\n    - path: foo.yml\n      hash_sha256: abc123\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("64 characters"));
    }

    // -- CategoriesRule --

    #[test]
    fn test_categories_valid() {
        let errors = lint(
            &CategoriesRule,
            "software:\n  packages:\n    - path: foo.yml\n      categories:\n        - Browsers\n        - Security\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_categories_invalid() {
        let errors = lint(
            &CategoriesRule,
            "software:\n  packages:\n    - path: foo.yml\n      categories:\n        - Gaming\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unknown category 'Gaming'"));
    }

    #[test]
    fn test_categories_case_suggestion() {
        let errors = lint(
            &CategoriesRule,
            "software:\n  packages:\n    - path: foo.yml\n      categories:\n        - browsers\n",
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].suggestion.as_deref(), Some("Browsers"));
    }

    // -- FileExtensionRule --

    #[test]
    fn test_file_extension_valid() {
        let errors = lint(
            &FileExtensionRule,
            "controls:\n  macos_settings:\n    custom_settings:\n      - path: ../lib/profile.mobileconfig\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_file_extension_invalid_macos() {
        let errors = lint(
            &FileExtensionRule,
            "controls:\n  macos_settings:\n    custom_settings:\n      - path: ../lib/profile.xml\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unexpected extension"));
    }

    #[test]
    fn test_file_extension_scripts() {
        let errors = lint(
            &FileExtensionRule,
            "controls:\n  scripts:\n    - path: ../lib/setup.sh\n",
        );
        assert!(errors.is_empty());
    }

    // -- SecretHygieneRule --

    #[test]
    fn test_secret_hygiene_env_var() {
        let errors = lint(
            &SecretHygieneRule,
            "integrations:\n  jira:\n    - url: https://jira.example.com\n      api_token: $JIRA_TOKEN\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_secret_hygiene_plaintext() {
        let errors = lint(
            &SecretHygieneRule,
            "integrations:\n  jira:\n    - url: https://jira.example.com\n      api_token: my-secret-token-123\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("plain-text"));
    }

    #[test]
    fn test_secret_hygiene_op_ref() {
        let errors = lint(
            &SecretHygieneRule,
            "integrations:\n  jira:\n    - url: https://jira.example.com\n      api_token: \"op://Vault/Jira/token\"\n",
        );
        assert!(errors.is_empty());
    }

    // -- PathReferenceRule --

    #[test]
    fn test_path_ref_valid_path() {
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - path: ../lib/policy.yml\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_path_ref_valid_paths_glob() {
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - paths: ../lib/policies/*.yml\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_path_ref_glob_in_path_field() {
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - path: ../lib/policies/*.yml\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("glob characters"));
        assert!(errors[0].message.contains("use 'paths'"));
    }

    #[test]
    fn test_path_ref_no_glob_in_paths_field() {
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - paths: ../lib/policies/specific.yml\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("does not contain glob"));
        assert!(errors[0].message.contains("use 'path'"));
    }

    #[test]
    fn test_path_ref_both_path_and_paths() {
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - path: foo.yml\n    paths: bar/*.yml\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("both 'path' and 'paths'"));
    }

    #[test]
    fn test_path_ref_inline_policy_ok() {
        // Inline policies (no path/paths) are fine
        let errors = lint(
            &PathReferenceRule,
            "policies:\n  - name: test\n    query: SELECT 1\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_path_ref_script_requires_path() {
        let errors = lint(
            &PathReferenceRule,
            "controls:\n  scripts:\n    - name: inline-script\n",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("must reference a file"));
    }

    #[test]
    fn test_path_ref_script_with_path_ok() {
        let errors = lint(
            &PathReferenceRule,
            "controls:\n  scripts:\n    - path: ./scripts/setup.sh\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_path_ref_script_with_glob_ok() {
        let errors = lint(
            &PathReferenceRule,
            "controls:\n  scripts:\n    - paths: ./scripts/*.sh\n",
        );
        assert!(errors.is_empty());
    }

    // -- is_valid_date --

    #[test]
    fn test_valid_dates() {
        assert!(is_valid_date("2025-06-15"));
        assert!(is_valid_date("2024-02-29")); // leap year
        assert!(is_valid_date("2025-12-31"));
    }

    #[test]
    fn test_invalid_dates() {
        assert!(!is_valid_date("2025-13-01")); // month > 12
        assert!(!is_valid_date("2025-02-29")); // not a leap year
        assert!(!is_valid_date("15-06-2025")); // wrong format
        assert!(!is_valid_date("2025/06/15")); // wrong separator
        assert!(!is_valid_date("not-a-date"));
    }
}
