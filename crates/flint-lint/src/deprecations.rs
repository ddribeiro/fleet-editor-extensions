//! Deprecation table and registry for Fleet GitOps key/directory renames.
//!
//! Single source of truth for all deprecations. Each entry is gated by version:
//! - **Dormant**: version < `deprecated_in` — no diagnostics
//! - **Warning**: `deprecated_in` <= version < `error_in` — emit warning
//! - **Error**: `error_in` <= version < `removed_in` — emit error
//! - **Removed**: version >= `removed_in` — key is gone, falls through to unknown
//!
//! All entries start with `deprecated_in = v99.0.0` (dormant). When Fleet ships
//! a rename, change the constant to the actual version to activate.

use once_cell::sync::Lazy;

use super::version::Version;

/// The kind of deprecation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeprecationKind {
    /// A YAML key was renamed.
    KeyRename {
        old_key: &'static str,
        new_key: &'static str,
        /// Dot-separated parent path where this key appears (empty = top level).
        context_path: &'static str,
    },
    /// A directory was renamed.
    DirectoryRename {
        old_dir: &'static str,
        new_dir: &'static str,
    },
    /// A file was renamed.
    FileRename {
        old_name: &'static str,
        new_name: &'static str,
    },
}

/// Which phase a deprecation is in for a given target version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeprecationPhase {
    /// Not yet active — no diagnostics emitted.
    Dormant,
    /// Active as a warning (grace period).
    Warning,
    /// Active as an error (grace period ended).
    Error,
    /// Fully removed — the old key/dir is no longer recognized at all.
    Removed,
}

/// A single deprecation entry.
#[derive(Debug, Clone)]
pub struct Deprecation {
    /// Unique identifier, e.g. `"team-settings-to-settings"`.
    pub id: &'static str,
    /// What changed.
    pub kind: DeprecationKind,
    /// Version when warnings start.
    pub deprecated_in: Version,
    /// Version when errors start (grace period end). `None` = no error phase.
    pub error_in: Option<Version>,
    /// Version when the old name is fully removed. `None` = never removed.
    pub removed_in: Option<Version>,
    /// Human-readable description of the change.
    pub description: &'static str,
    /// Glob patterns for files this deprecation applies to (empty = all files).
    pub file_patterns: &'static [&'static str],
}

impl Deprecation {
    /// Determine which phase this deprecation is in for a given target version.
    pub fn phase_for_version(&self, target: &Version) -> DeprecationPhase {
        if let Some(ref removed) = self.removed_in {
            if target >= removed {
                return DeprecationPhase::Removed;
            }
        }
        if let Some(ref error) = self.error_in {
            if target >= error {
                return DeprecationPhase::Error;
            }
        }
        if target >= &self.deprecated_in {
            return DeprecationPhase::Warning;
        }
        DeprecationPhase::Dormant
    }
}

/// Registry of all known deprecations.
pub struct DeprecationRegistry {
    entries: Vec<Deprecation>,
}

impl DeprecationRegistry {
    /// Find a deprecated key by name and context path.
    ///
    /// `context_path` is the dot-separated path to the parent mapping
    /// (empty string for top-level keys).
    pub fn find_deprecated_key(&self, key: &str, context_path: &str) -> Option<&Deprecation> {
        self.entries.iter().find(|d| match &d.kind {
            DeprecationKind::KeyRename {
                old_key,
                context_path: cp,
                ..
            } => *old_key == key && *cp == context_path,
            _ => false,
        })
    }

    /// Find a deprecated directory by name.
    pub fn find_deprecated_directory(&self, dir_name: &str) -> Option<&Deprecation> {
        self.entries.iter().find(|d| match &d.kind {
            DeprecationKind::DirectoryRename { old_dir, .. } => *old_dir == dir_name,
            _ => false,
        })
    }

    /// Return all deprecations that are non-dormant for a given version.
    pub fn active_deprecations(&self, version: &Version) -> Vec<&Deprecation> {
        self.entries
            .iter()
            .filter(|d| d.phase_for_version(version) != DeprecationPhase::Dormant)
            .collect()
    }

    /// Return all directory rename deprecations active for a given version.
    pub fn active_directory_renames(&self, version: &Version) -> Vec<&Deprecation> {
        self.entries
            .iter()
            .filter(|d| matches!(&d.kind, DeprecationKind::DirectoryRename { .. }))
            .filter(|d| d.phase_for_version(version) != DeprecationPhase::Dormant)
            .collect()
    }

    /// Return all file rename deprecations active for a given version.
    pub fn active_file_renames(&self, version: &Version) -> Vec<&Deprecation> {
        self.entries
            .iter()
            .filter(|d| matches!(&d.kind, DeprecationKind::FileRename { .. }))
            .filter(|d| d.phase_for_version(version) != DeprecationPhase::Dormant)
            .collect()
    }

    /// Get all entries in the registry.
    pub fn entries(&self) -> &[Deprecation] {
        &self.entries
    }
}

// ---------------------------------------------------------------------------
// Dormant version constant — change this to activate deprecations
// ---------------------------------------------------------------------------

/// Version when deprecation warnings start (grace period).
fn deprecated_version() -> Version {
    Version::new(4, 80, 1)
}

/// Version when deprecation errors start (grace period ended).
/// NOTE: This is a projected version — Fleet has not announced an exact date.
/// Update when Fleet confirms the mandatory cutover version.
fn mandatory_version() -> Version {
    Version::new(4, 88, 0)
}

/// Global deprecation registry.
pub static DEPRECATION_REGISTRY: Lazy<DeprecationRegistry> = Lazy::new(|| {
    DeprecationRegistry {
        entries: vec![
            // teams/ directory -> fleets/
            Deprecation {
                id: "teams-dir-to-fleets-dir",
                kind: DeprecationKind::DirectoryRename {
                    old_dir: "teams",
                    new_dir: "fleets",
                },
                deprecated_in: deprecated_version(),
                error_in: Some(mandatory_version()),
                removed_in: None,
                description: "The 'teams/' directory is being renamed to 'fleets/'",
                file_patterns: &[],
            },
            // team_settings -> settings
            Deprecation {
                id: "team-settings-to-settings",
                kind: DeprecationKind::KeyRename {
                    old_key: "team_settings",
                    new_key: "settings",
                    context_path: "",
                },
                deprecated_in: deprecated_version(),
                error_in: Some(mandatory_version()),
                removed_in: None,
                description: "The 'team_settings' key is being renamed to 'settings'",
                file_patterns: &[],
            },
            // queries -> reports
            Deprecation {
                id: "queries-to-reports",
                kind: DeprecationKind::KeyRename {
                    old_key: "queries",
                    new_key: "reports",
                    context_path: "",
                },
                deprecated_in: deprecated_version(),
                error_in: Some(mandatory_version()),
                removed_in: None,
                description: "The 'queries' key is being renamed to 'reports'",
                file_patterns: &[],
            },
            // no-team.yml -> unassigned.yml
            Deprecation {
                id: "no-team-to-unassigned",
                kind: DeprecationKind::FileRename {
                    old_name: "no-team.yml",
                    new_name: "unassigned.yml",
                },
                deprecated_in: deprecated_version(),
                error_in: Some(mandatory_version()),
                removed_in: None,
                description: "The 'no-team.yml' file is being renamed to 'unassigned.yml'",
                file_patterns: &[],
            },
        ],
    }
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dormant_phase() {
        let dep = Deprecation {
            id: "test",
            kind: DeprecationKind::KeyRename {
                old_key: "old",
                new_key: "new",
                context_path: "",
            },
            deprecated_in: Version::new(5, 0, 0),
            error_in: Some(Version::new(6, 0, 0)),
            removed_in: Some(Version::new(7, 0, 0)),
            description: "test",
            file_patterns: &[],
        };

        assert_eq!(
            dep.phase_for_version(&Version::new(4, 0, 0)),
            DeprecationPhase::Dormant
        );
        assert_eq!(
            dep.phase_for_version(&Version::new(4, 99, 99)),
            DeprecationPhase::Dormant
        );
    }

    #[test]
    fn test_warning_phase() {
        let dep = Deprecation {
            id: "test",
            kind: DeprecationKind::KeyRename {
                old_key: "old",
                new_key: "new",
                context_path: "",
            },
            deprecated_in: Version::new(5, 0, 0),
            error_in: Some(Version::new(6, 0, 0)),
            removed_in: Some(Version::new(7, 0, 0)),
            description: "test",
            file_patterns: &[],
        };

        assert_eq!(
            dep.phase_for_version(&Version::new(5, 0, 0)),
            DeprecationPhase::Warning
        );
        assert_eq!(
            dep.phase_for_version(&Version::new(5, 50, 0)),
            DeprecationPhase::Warning
        );
    }

    #[test]
    fn test_error_phase() {
        let dep = Deprecation {
            id: "test",
            kind: DeprecationKind::KeyRename {
                old_key: "old",
                new_key: "new",
                context_path: "",
            },
            deprecated_in: Version::new(5, 0, 0),
            error_in: Some(Version::new(6, 0, 0)),
            removed_in: Some(Version::new(7, 0, 0)),
            description: "test",
            file_patterns: &[],
        };

        assert_eq!(
            dep.phase_for_version(&Version::new(6, 0, 0)),
            DeprecationPhase::Error
        );
        assert_eq!(
            dep.phase_for_version(&Version::new(6, 50, 0)),
            DeprecationPhase::Error
        );
    }

    #[test]
    fn test_removed_phase() {
        let dep = Deprecation {
            id: "test",
            kind: DeprecationKind::KeyRename {
                old_key: "old",
                new_key: "new",
                context_path: "",
            },
            deprecated_in: Version::new(5, 0, 0),
            error_in: Some(Version::new(6, 0, 0)),
            removed_in: Some(Version::new(7, 0, 0)),
            description: "test",
            file_patterns: &[],
        };

        assert_eq!(
            dep.phase_for_version(&Version::new(7, 0, 0)),
            DeprecationPhase::Removed
        );
        assert_eq!(
            dep.phase_for_version(&Version::new(8, 0, 0)),
            DeprecationPhase::Removed
        );
    }

    #[test]
    fn test_no_error_in_skips_to_removed() {
        let dep = Deprecation {
            id: "test",
            kind: DeprecationKind::KeyRename {
                old_key: "old",
                new_key: "new",
                context_path: "",
            },
            deprecated_in: Version::new(5, 0, 0),
            error_in: None,
            removed_in: Some(Version::new(7, 0, 0)),
            description: "test",
            file_patterns: &[],
        };

        // Warning phase spans from 5.0.0 to 7.0.0 (no error phase)
        assert_eq!(
            dep.phase_for_version(&Version::new(6, 0, 0)),
            DeprecationPhase::Warning
        );
        assert_eq!(
            dep.phase_for_version(&Version::new(7, 0, 0)),
            DeprecationPhase::Removed
        );
    }

    #[test]
    fn test_find_deprecated_key() {
        let registry = &*DEPRECATION_REGISTRY;

        // Should find team_settings at top level
        let dep = registry.find_deprecated_key("team_settings", "");
        assert!(dep.is_some());
        assert_eq!(dep.unwrap().id, "team-settings-to-settings");

        // Should find queries at top level
        let dep = registry.find_deprecated_key("queries", "");
        assert!(dep.is_some());
        assert_eq!(dep.unwrap().id, "queries-to-reports");

        // Should not find unknown keys
        assert!(registry.find_deprecated_key("foobar", "").is_none());

        // Should not match at wrong context path
        assert!(registry
            .find_deprecated_key("team_settings", "org_settings")
            .is_none());
    }

    #[test]
    fn test_find_deprecated_directory() {
        let registry = &*DEPRECATION_REGISTRY;

        let dep = registry.find_deprecated_directory("teams");
        assert!(dep.is_some());
        assert_eq!(dep.unwrap().id, "teams-dir-to-fleets-dir");

        assert!(registry.find_deprecated_directory("foobar").is_none());
    }

    #[test]
    fn test_active_deprecations_dormant() {
        let registry = &*DEPRECATION_REGISTRY;

        // With a version below v99.0.0, nothing should be active
        let active = registry.active_deprecations(&Version::new(4, 80, 0));
        assert!(
            active.is_empty(),
            "Expected no active deprecations at v4.80.0 (dormant), got {}",
            active.len()
        );
    }

    #[test]
    fn test_active_deprecations_warning() {
        let registry = &*DEPRECATION_REGISTRY;

        // At v4.85.0 (between deprecated_in=4.80.1 and error_in=4.88.0), all should be active as warnings
        let active = registry.active_deprecations(&Version::new(4, 85, 0));
        assert_eq!(active.len(), 4);
        for dep in &active {
            assert_eq!(
                dep.phase_for_version(&Version::new(4, 85, 0)),
                DeprecationPhase::Warning
            );
        }
    }

    #[test]
    fn test_active_deprecations_error() {
        let registry = &*DEPRECATION_REGISTRY;

        // At v4.88.0 (error_in version), all should be active as errors
        let active = registry.active_deprecations(&Version::new(4, 88, 0));
        assert_eq!(active.len(), 4);
        for dep in &active {
            assert_eq!(
                dep.phase_for_version(&Version::new(4, 88, 0)),
                DeprecationPhase::Error
            );
        }
    }
}
