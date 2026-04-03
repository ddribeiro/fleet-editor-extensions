//! Hover provider for Fleet GitOps JSON files.
//!
//! Provides documentation when hovering over field names in:
//! - DEP enrollment profiles (`*.dep.json`)
//! - Apple DDM declaration profiles (`declaration-profiles/*.json`)
//!
//! Uses text-based analysis (no JSON parser) so it works on partial/broken
//! documents during editing.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

/// Provide hover information at a position in a JSON document.
///
/// `file_hint` is the file name or path, used to determine which doc set to use.
pub fn json_hover_at(source: &str, position: Position, file_hint: &str) -> Option<Hover> {
    let line_idx = position.line as usize;
    let col_idx = position.character as usize;

    let line = source.lines().nth(line_idx)?;

    // Find the JSON key at cursor (quoted string before a colon)
    let (key, word_start, word_end) = find_json_key_at(line, col_idx)?;

    // Build the context path (e.g., "Payload.TargetOSVersion")
    let parent_path = determine_json_context(source, line_idx);
    let full_path = if parent_path.is_empty() {
        key.clone()
    } else {
        format!("{}.{}", parent_path, key)
    };

    // Look up docs based on file type
    let doc_set = if is_dep_profile(file_hint) {
        &*DEP_FIELD_DOCS
    } else if is_declaration_profile(file_hint) {
        &*DDM_FIELD_DOCS
    } else {
        // Try both
        if DEP_FIELD_DOCS.contains_key(full_path.as_str())
            || DEP_FIELD_DOCS.contains_key(key.as_str())
        {
            &*DEP_FIELD_DOCS
        } else {
            &*DDM_FIELD_DOCS
        }
    };

    // Look up: try full path first, then just the key
    let doc = doc_set
        .get(full_path.as_str())
        .or_else(|| doc_set.get(key.as_str()))?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.to_string(),
        }),
        range: Some(Range {
            start: Position {
                line: position.line,
                character: word_start as u32,
            },
            end: Position {
                line: position.line,
                character: word_end as u32,
            },
        }),
    })
}

/// Find the JSON key at a cursor position.
///
/// JSON keys are quoted strings followed by `:`. Returns the key name
/// (without quotes), and the start/end columns of the key (including quotes).
fn find_json_key_at(line: &str, col: usize) -> Option<(String, usize, usize)> {
    // Find all "key": patterns on this line
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Find opening quote
        if bytes[i] == b'"' {
            let key_start = i;
            i += 1;
            // Find closing quote (handle escaped quotes)
            let mut key_end = None;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2; // skip escape sequence
                    continue;
                }
                if bytes[i] == b'"' {
                    key_end = Some(i);
                    break;
                }
                i += 1;
            }

            if let Some(end) = key_end {
                // Check if this quoted string is followed by `:`
                let after = &line[end + 1..].trim_start();
                let is_key = after.starts_with(':');

                if is_key && col >= key_start && col <= end {
                    let key_name = &line[key_start + 1..end];
                    return Some((key_name.to_string(), key_start, end + 1));
                }

                // Also match if cursor is on the key name even in value position
                // (but prefer keys over values)
                if !is_key && col >= key_start && col <= end {
                    // This is a string value, not a key — skip
                }
            }
        }
        i += 1;
    }

    // Fallback: try to find an unquoted word that matches a known key
    // (for when cursor is just inside the quotes)
    None
}

/// Walk backwards from the current line to determine the JSON path context.
///
/// Returns a dot-separated path like `"Payload"` for keys nested inside
/// `"Payload": { ... }`.
fn determine_json_context(source: &str, line_idx: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut path_parts: Vec<String> = Vec::new();
    let mut brace_depth: i32 = 0;

    // Walk backwards, tracking brace depth
    for i in (0..line_idx).rev() {
        let line = lines.get(i).unwrap_or(&"");
        let trimmed = line.trim();

        // Count braces on this line
        for ch in trimmed.chars() {
            match ch {
                '}' => brace_depth += 1,
                '{' => brace_depth -= 1,
                _ => {}
            }
        }

        // If we've gone up a nesting level (found an unmatched `{`),
        // extract the key from this line
        if brace_depth < 0 {
            if let Some(key) = extract_json_key(trimmed) {
                path_parts.push(key);
            }
            brace_depth = 0; // reset for next level
        }

        let indent = leading_spaces(line);
        if indent == 0 && !trimmed.is_empty() && trimmed != "{" {
            break;
        }
    }

    path_parts.reverse();
    path_parts.join(".")
}

/// Extract a JSON key from a line like `"Payload": {`.
fn extract_json_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(start) = trimmed.find('"') {
        let rest = &trimmed[start + 1..];
        if let Some(end) = rest.find('"') {
            let key = &rest[..end];
            // Verify it's followed by a colon
            let after = rest[end + 1..].trim_start();
            if after.starts_with(':') {
                return Some(key.to_string());
            }
        }
    }
    None
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_dep_profile(file_hint: &str) -> bool {
    file_hint.contains(".dep.json") || file_hint.contains("enrollment-profile")
}

fn is_declaration_profile(file_hint: &str) -> bool {
    file_hint.contains("declaration-profile")
}

// ============================================================================
// DEP Enrollment Profile Field Docs
// ============================================================================

/// Documentation for Apple DEP (Device Enrollment Program) profile fields.
///
/// Reference: Apple MDM protocol — DEP profile JSON payload.
static DEP_FIELD_DOCS: Lazy<HashMap<&'static str, String>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "profile_name",
        md(
            "profile_name",
            "A human-readable name for the DEP enrollment profile.",
            "string",
            Some("\"profile_name\": \"Automatic enrollment profile\""),
        ),
    );

    m.insert("url", md(
        "url",
        "The URL of the MDM server that the device will enroll with. Fleet populates this automatically.",
        "string",
        None,
    ));

    m.insert("allow_pairing", md(
        "allow_pairing",
        "If `true`, the device can be paired with a host computer (e.g. via USB). Set to `false` for kiosk or shared devices.",
        "boolean",
        Some("\"allow_pairing\": true"),
    ));

    m.insert("is_supervised", md(
        "is_supervised",
        "If `true`, the device is enrolled in **supervised mode**, enabling additional management capabilities (app restrictions, single-app mode, lost mode, etc.). Recommended for organization-owned devices.",
        "boolean",
        Some("\"is_supervised\": true"),
    ));

    m.insert("is_mandatory", md(
        "is_mandatory",
        "If `true`, the user cannot skip MDM enrollment during Setup Assistant. The device must enroll before it can be used.",
        "boolean",
        Some("\"is_mandatory\": true"),
    ));

    m.insert("is_mdm_removable", md(
        "is_mdm_removable",
        "If `true`, the MDM profile can be removed by the device user. Set to `false` to prevent users from unenrolling managed devices.",
        "boolean",
        Some("\"is_mdm_removable\": false"),
    ));

    m.insert("auto_advance_setup", md(
        "auto_advance_setup",
        "If `true`, Setup Assistant screens are automatically advanced without user interaction. Used for zero-touch deployment.",
        "boolean",
        Some("\"auto_advance_setup\": true"),
    ));

    m.insert("await_device_configured", md(
        "await_device_configured",
        "If `true`, the device waits at the \"Configuring...\" screen until the MDM server sends the `DeviceConfigured` command. Useful for ensuring all profiles and apps are installed before the user reaches the home screen.",
        "boolean",
        Some("\"await_device_configured\": true"),
    ));

    m.insert("language", md(
        "language",
        "The [ISO 639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) language code for Setup Assistant (e.g. `en`, `de`, `ja`).",
        "string",
        Some("\"language\": \"en\""),
    ));

    m.insert("region", md(
        "region",
        "The [ISO 3166-1 alpha-2](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2) region code for Setup Assistant (e.g. `US`, `DE`, `GB`).",
        "string",
        Some("\"region\": \"US\""),
    ));

    m.insert("org_magic", md(
        "org_magic",
        "An arbitrary string that the MDM server can use to identify the enrollment profile. Often set to a static value like `\"1\"`.",
        "string",
        Some("\"org_magic\": \"1\""),
    ));

    m.insert(
        "department",
        md(
            "department",
            "The department or group name shown during Setup Assistant enrollment.",
            "string",
            Some("\"department\": \"IT\""),
        ),
    );

    m.insert(
        "support_phone_number",
        md(
            "support_phone_number",
            "IT support phone number displayed during Setup Assistant enrollment.",
            "string",
            Some("\"support_phone_number\": \"+1-555-0100\""),
        ),
    );

    m.insert(
        "support_email_address",
        md(
            "support_email_address",
            "IT support email address displayed during Setup Assistant enrollment.",
            "string",
            Some("\"support_email_address\": \"it@example.com\""),
        ),
    );

    // Full list from github.com/apple/device-management/other/skipkeys.yaml
    m.insert("skip_setup_items", md(
        "skip_setup_items",
        "An array of Setup Assistant panes to skip during enrollment. \
        Each string identifies a pane to bypass.\n\n\
        [Source: apple/device-management](https://github.com/apple/device-management/blob/main/other/skipkeys.yaml)\n\n\
        | Key | Pane | Platforms |\n\
        |---|---|---|\n\
        | `Accessibility` | Accessibility (new-user login only) | macOS 11+ |\n\
        | `ActionButton` | Action Button configuration | iOS 17+ |\n\
        | `Android` | Remove \"Move from Android\" option | iOS 9+ |\n\
        | `Appearance` | Choose Your Look (light/dark) | iOS 13+ / macOS 10.14+ |\n\
        | `AppleID` | Apple Account sign-in | iOS 7+ / macOS 10.9+ |\n\
        | `AppStore` | App Store pane | iOS 14.3+ / macOS 11.1+ |\n\
        | `Biometric` | Touch ID / Face ID setup | iOS 8.1+ / macOS 10.12.4+ |\n\
        | `CameraButton` | Camera Control pane | iOS 18+ |\n\
        | `DeviceToDeviceMigration` | Device-to-device migration | iOS 12.4+ |\n\
        | `Diagnostics` | App Analytics | iOS 7+ / macOS 10.9+ |\n\
        | `DisplayTone` | True Tone display | iOS 9.3.2–15 / macOS 10.13.6–12 |\n\
        | `EnableLockdownMode` | Lockdown Mode | iOS 17.1+ / macOS 14+ |\n\
        | `FileVault` | FileVault disk encryption | macOS 10.10+ |\n\
        | `iCloudDiagnostics` | iCloud Analytics | macOS 10.12.4+ |\n\
        | `iCloudStorage` | iCloud Documents & Desktop | macOS 10.13.4+ |\n\
        | `iMessageAndFaceTime` | iMessage and FaceTime | iOS 12+ |\n\
        | `Intelligence` | Apple Intelligence | iOS 18+ / macOS 15+ |\n\
        | `Location` | Location Services | iOS 7+ / macOS 10.11+ |\n\
        | `Multitasking` | Multitasking | iOS 26+ |\n\
        | `OSShowcase` | OS Showcase pane | iOS 26+ / macOS 26.1+ |\n\
        | `Passcode` | Passcode setup | iOS 7+ / macOS 10.9+ |\n\
        | `Payment` | Apple Pay | iOS 8.1+ / macOS 10.12.4+ |\n\
        | `Privacy` | Privacy consent | iOS 11.3+ / macOS 10.13.4+ |\n\
        | `Restore` | Restore from backup | iOS 7+ / macOS 10.9+ |\n\
        | `Safety` | Safety pane | iOS 16+ |\n\
        | `ScreenTime` | Screen Time | iOS 12+ / macOS 10.15+ |\n\
        | `SIMSetup` | Add cellular plan (eSIM) | iOS 12+ |\n\
        | `Siri` | Siri | iOS 7+ / macOS 10.12+ |\n\
        | `SoftwareUpdate` | Mandatory software update | iOS 12+ / macOS 15.4+ |\n\
        | `TermsOfAddress` | Preferred pronouns | iOS 16+ / macOS 13+ |\n\
        | `TOS` | Terms and Conditions | iOS 7+ / macOS 10.9+ |\n\
        | `UnlockWithWatch` | Unlock with Apple Watch | macOS 15+ |\n\
        | `Welcome` | Get Started pane | iOS 13+ / macOS 15+ |\n\
        | `WebContentFiltering` | Web Content Filtering | iOS 18.2+ |",
        "array of strings",
        Some("\"skip_setup_items\": [\"AppleID\", \"Siri\", \"Payment\"]"),
    ));

    m
});

// ============================================================================
// Apple DDM Declaration Profile Field Docs
// ============================================================================

/// Documentation for Apple Declarative Device Management (DDM) declaration fields.
///
/// Reference: Apple DDM documentation.
static DDM_FIELD_DOCS: Lazy<HashMap<&'static str, String>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert("Type", md(
        "Type",
        "The declaration type identifier. This is a reverse-DNS string that specifies which configuration is being declared.\n\n\
        **Common types:**\n\
        - `com.apple.configuration.softwareupdate.enforcement.specific` — Enforce a specific OS version\n\
        - `com.apple.configuration.passcode.settings` — Passcode requirements\n\
        - `com.apple.configuration.screensharing.connection.group` — Screen sharing\n\
        - `com.apple.configuration.management.status-subscriptions` — Status subscriptions",
        "string (reverse-DNS)",
        Some("\"Type\": \"com.apple.configuration.softwareupdate.enforcement.specific\""),
    ));

    m.insert("Identifier", md(
        "Identifier",
        "A unique identifier for this declaration, typically in reverse-DNS format. Must be unique across all declarations managed by the MDM server.",
        "string (reverse-DNS)",
        Some("\"Identifier\": \"com.example.config.softwareupdate\""),
    ));

    m.insert("ServerToken", md(
        "ServerToken",
        "An opaque token used by the server to track declaration versions. The server updates this value whenever the declaration changes, allowing the device to detect updates.",
        "string",
        None,
    ));

    m.insert("Payload", md(
        "Payload",
        "The declaration-specific payload object. The structure depends on the `Type` field. Contains the actual configuration settings for this declaration.",
        "object",
        Some("\"Payload\": {\n  \"TargetOSVersion\": \"15.0\"\n}"),
    ));

    // Software update enforcement payload fields
    m.insert("Payload.TargetOSVersion", md(
        "TargetOSVersion",
        "The target OS version to enforce (e.g. `\"15.0\"`, `\"26.4\"`). The device will be prompted to update to this version. Use the marketing version number.",
        "string",
        Some("\"TargetOSVersion\": \"15.0\""),
    ));

    m.insert("Payload.TargetBuildVersion", md(
        "TargetBuildVersion",
        "The target build version to enforce (e.g. `\"25E246\"`). More precise than `TargetOSVersion` — ensures the exact build is installed. Find build numbers at [Apple support](https://support.apple.com/en-us/100100).",
        "string",
        Some("\"TargetBuildVersion\": \"25E246\""),
    ));

    m.insert("Payload.TargetLocalDateTime", md(
        "TargetLocalDateTime",
        "The local date and time by which the update must be installed (`ISO 8601`). After this deadline, the device will force-install the update. The user receives increasingly urgent notifications as the deadline approaches.",
        "string (ISO 8601)",
        Some("\"TargetLocalDateTime\": \"2026-07-12T13:50:00\""),
    ));

    m.insert("Payload.DetailsURL", md(
        "DetailsURL",
        "A URL shown to the user in the update notification. Typically links to release notes or an internal IT page explaining why the update is required.",
        "string (URL)",
        Some("\"DetailsURL\": \"https://support.apple.com/en-us/100100\""),
    ));

    // Passcode settings payload fields
    m.insert("Payload.MaxFailedAttempts", md(
        "MaxFailedAttempts",
        "Maximum number of failed passcode attempts before the device is wiped or locked. Typical values: `6`–`11`.",
        "integer",
        Some("\"MaxFailedAttempts\": 10"),
    ));

    m.insert("Payload.MaxInactivity", md(
        "MaxInactivity",
        "Maximum number of minutes of inactivity before the device auto-locks. Use `0` for no auto-lock restriction.",
        "integer (minutes)",
        Some("\"MaxInactivity\": 5"),
    ));

    m.insert("Payload.MaxPINAgeInDays", md(
        "MaxPINAgeInDays",
        "Maximum age of the passcode in days before the user must change it. Use `0` for no expiration.",
        "integer (days)",
        Some("\"MaxPINAgeInDays\": 90"),
    ));

    m.insert(
        "Payload.MinLength",
        md(
            "MinLength",
            "Minimum length of the device passcode. Apple default is `0` (no minimum).",
            "integer",
            Some("\"MinLength\": 6"),
        ),
    );

    m.insert(
        "Payload.RequireAlphanumeric",
        md(
            "RequireAlphanumeric",
            "If `true`, the passcode must contain both letters and numbers (not just digits).",
            "boolean",
            Some("\"RequireAlphanumeric\": true"),
        ),
    );

    m
});

// ============================================================================
// Helpers
// ============================================================================

/// Build a markdown hover string for a JSON field.
fn md(name: &str, description: &str, field_type: &str, example: Option<&str>) -> String {
    let mut s = format!(
        "**{}**\n\n{}\n\n**Type:** `{}`",
        name, description, field_type
    );
    if let Some(ex) = example {
        s.push_str(&format!("\n\n**Example:**\n```json\n{}\n```", ex));
    }
    s
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_json_key_at() {
        let line = r#"  "profile_name": "Automatic enrollment profile","#;
        let result = find_json_key_at(line, 5);
        assert!(result.is_some());
        let (key, start, end) = result.unwrap();
        assert_eq!(key, "profile_name");
        assert_eq!(start, 2); // opening quote
        assert_eq!(end, 16); // after closing quote
    }

    #[test]
    fn test_find_json_key_at_cursor_on_quote() {
        let line = r#"  "allow_pairing": true,"#;
        let result = find_json_key_at(line, 2); // on opening quote
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "allow_pairing");
    }

    #[test]
    fn test_find_json_key_not_value() {
        let line = r#"  "profile_name": "Automatic enrollment profile","#;
        // Cursor on the value string should return None
        let result = find_json_key_at(line, 25);
        assert!(result.is_none());
    }

    #[test]
    fn test_determine_json_context_top_level() {
        let source = r#"{
  "profile_name": "test",
  "allow_pairing": true
}"#;
        let ctx = determine_json_context(source, 1);
        assert!(ctx.is_empty(), "Top-level keys should have empty context");
    }

    #[test]
    fn test_determine_json_context_nested() {
        let source = r#"{
  "Type": "com.apple.configuration.softwareupdate.enforcement.specific",
  "Payload": {
    "TargetOSVersion": "15.0"
  }
}"#;
        let ctx = determine_json_context(source, 3);
        assert_eq!(ctx, "Payload");
    }

    #[test]
    fn test_hover_dep_profile() {
        let source = r#"{
  "profile_name": "Automatic enrollment profile",
  "allow_pairing": true,
  "is_supervised": true
}"#;
        let hover = json_hover_at(
            source,
            Position {
                line: 1,
                character: 5,
            },
            "automatic-enrollment.dep.json",
        );
        assert!(hover.is_some());
        let content = match hover.unwrap().contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("Expected markup"),
        };
        assert!(content.contains("profile_name"));
        assert!(content.contains("human-readable"));
    }

    #[test]
    fn test_hover_ddm_payload_field() {
        let source = r#"{
  "Type": "com.apple.configuration.softwareupdate.enforcement.specific",
  "Payload": {
    "TargetOSVersion": "26.4"
  }
}"#;
        let hover = json_hover_at(
            source,
            Position {
                line: 3,
                character: 8,
            },
            "declaration-profiles/software-update.json",
        );
        assert!(hover.is_some());
        let content = match hover.unwrap().contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("Expected markup"),
        };
        assert!(content.contains("TargetOSVersion"));
        assert!(content.contains("target OS version"));
    }

    #[test]
    fn test_hover_skip_setup_items() {
        let source = r#"{
  "skip_setup_items": ["AppleID", "Siri"]
}"#;
        let hover = json_hover_at(
            source,
            Position {
                line: 1,
                character: 8,
            },
            "enrollment.dep.json",
        );
        assert!(hover.is_some());
        let content = match hover.unwrap().contents {
            HoverContents::Markup(m) => m.value,
            _ => panic!("Expected markup"),
        };
        assert!(content.contains("skip_setup_items"));
        assert!(content.contains("Setup Assistant"));
    }

    #[test]
    fn test_is_dep_profile() {
        assert!(is_dep_profile("automatic-enrollment.dep.json"));
        assert!(is_dep_profile("/path/to/enrollment-profiles/auto.json"));
        assert!(!is_dep_profile("declaration-profiles/update.json"));
    }

    #[test]
    fn test_is_declaration_profile() {
        assert!(is_declaration_profile(
            "declaration-profiles/software-update.json"
        ));
        assert!(!is_declaration_profile("enrollment.dep.json"));
    }
}
