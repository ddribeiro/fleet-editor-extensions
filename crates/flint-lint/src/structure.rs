//! Schema tree definitions for structural YAML validation.
//!
//! Defines valid YAML structure for each Fleet GitOps file type,
//! sourced from the strict JSON schema. Used by `StructuralValidationRule`
//! to detect unknown keys, misplaced keys, and missing wrappers.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Describes the valid structure at a single level of the YAML tree.
#[derive(Debug, Clone)]
pub enum SchemaNode {
    /// A mapping with known child keys.
    Mapping(HashMap<&'static str, SchemaNode>),
    /// An array whose items follow a given schema.
    Array(Box<SchemaNode>),
    /// An array whose items can be one of several schemas (oneOf).
    ArrayOneOf(Vec<SchemaNode>),
    /// A scalar or opaque value (no children to validate).
    Leaf,
    /// A boolean value — only `true`/`false` (and YAML 1.1 equivalents) are valid.
    BooleanLeaf,
    /// A mapping that allows arbitrary keys (e.g. `additionalProperties: true`).
    OpenMapping,
}

impl SchemaNode {
    /// Get valid child keys if this is a Mapping node.
    pub fn valid_keys(&self) -> Option<Vec<&'static str>> {
        match self {
            SchemaNode::Mapping(children) => Some(children.keys().copied().collect()),
            _ => None,
        }
    }

    /// Look up a child key in a Mapping node.
    pub fn get_child(&self, key: &str) -> Option<&SchemaNode> {
        match self {
            SchemaNode::Mapping(children) => children.get(key),
            _ => None,
        }
    }

    /// Check if this node allows arbitrary keys.
    pub fn allows_unknown(&self) -> bool {
        matches!(
            self,
            SchemaNode::OpenMapping | SchemaNode::Leaf | SchemaNode::BooleanLeaf
        )
    }
}

/// Global registry mapping every known key name to the path(s) where it's valid.
/// Used for misplaced-key detection.
#[derive(Debug)]
pub struct KeyRegistry {
    /// key_name -> list of dot-separated paths where it's valid
    entries: HashMap<&'static str, Vec<&'static str>>,
}

impl KeyRegistry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn register(&mut self, key: &'static str, path: &'static str) {
        self.entries.entry(key).or_default().push(path);
    }

    /// Look up the valid paths for a key name.
    pub fn lookup(&self, key: &str) -> Option<&[&'static str]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    /// Get all known key names.
    pub fn all_keys(&self) -> Vec<&'static str> {
        self.entries.keys().copied().collect()
    }
}

// ---------------------------------------------------------------------------
// Helper macros / builders
// ---------------------------------------------------------------------------

fn mapping(children: Vec<(&'static str, SchemaNode)>) -> SchemaNode {
    SchemaNode::Mapping(children.into_iter().collect())
}

fn array(item: SchemaNode) -> SchemaNode {
    SchemaNode::Array(Box::new(item))
}

fn array_one_of(variants: Vec<SchemaNode>) -> SchemaNode {
    SchemaNode::ArrayOneOf(variants)
}

fn leaf() -> SchemaNode {
    SchemaNode::Leaf
}

fn boolean_leaf() -> SchemaNode {
    SchemaNode::BooleanLeaf
}

fn open_mapping() -> SchemaNode {
    SchemaNode::OpenMapping
}

// ---------------------------------------------------------------------------
// Shared sub-schemas (matching the strict JSON schema $defs)
// ---------------------------------------------------------------------------

fn path_ref_schema() -> SchemaNode {
    mapping(vec![("path", leaf()), ("paths", leaf())])
}

fn controls_script_targeting() -> SchemaNode {
    mapping(vec![
        ("path", leaf()),
        ("paths", leaf()),
        ("labels_include_all", array(leaf())),
        ("labels_include_any", array(leaf())),
        ("labels_exclude_any", array(leaf())),
    ])
}

fn macos_updates() -> SchemaNode {
    mapping(vec![
        ("deadline", leaf()),
        ("minimum_version", leaf()),
        ("update_new_hosts", boolean_leaf()),
    ])
}

fn windows_updates() -> SchemaNode {
    mapping(vec![
        ("deadline_days", leaf()),
        ("grace_period_days", leaf()),
    ])
}

fn os_settings() -> SchemaNode {
    mapping(vec![
        ("custom_settings", array(controls_script_targeting())),
        ("configuration_profiles", array(controls_script_targeting())), // rename of custom_settings
    ])
}

fn android_settings_schema() -> SchemaNode {
    mapping(vec![
        ("custom_settings", array(controls_script_targeting())),
        ("configuration_profiles", array(controls_script_targeting())), // rename of custom_settings
        (
            "certificates",
            array(mapping(vec![
                ("name", leaf()),
                ("certificate_authority_name", leaf()),
                ("subject_name", leaf()),
            ])),
        ),
    ])
}

fn macos_setup() -> SchemaNode {
    mapping(vec![
        // Old names (json tags)
        ("bootstrap_package", leaf()),
        ("manual_agent_install", boolean_leaf()),
        ("enable_end_user_authentication", boolean_leaf()),
        ("lock_end_user_info", boolean_leaf()),
        ("require_all_software", boolean_leaf()),
        ("enable_release_device_manually", boolean_leaf()),
        ("macos_setup_assistant", leaf()),
        ("script", leaf()),
        // New names (renameto tags from Go code)
        ("macos_bootstrap_package", leaf()),
        ("macos_manual_agent_install", boolean_leaf()),
        ("apple_enable_release_device_manually", boolean_leaf()),
        ("apple_setup_assistant", leaf()),
        ("macos_script", leaf()),
    ])
}

fn macos_migration() -> SchemaNode {
    mapping(vec![
        ("enable", boolean_leaf()),
        ("mode", leaf()),
        ("webhook_url", leaf()),
    ])
}

fn software_asset() -> SchemaNode {
    mapping(vec![("path", leaf())])
}

fn software_package_item() -> SchemaNode {
    mapping(vec![
        ("path", leaf()),
        ("url", leaf()),
        ("hash_sha256", leaf()),
        ("display_name", leaf()),
        ("self_service", boolean_leaf()),
        ("setup_experience", boolean_leaf()),
        ("categories", array(leaf())),
        ("labels_include_any", array(leaf())),
        ("labels_exclude_any", array(leaf())),
        ("labels_include_all", array(leaf())),
        ("pre_install_query", software_asset()),
        ("install_script", software_asset()),
        ("uninstall_script", software_asset()),
        ("post_install_script", software_asset()),
        ("icon", software_asset()),
    ])
}

fn app_store_app_item() -> SchemaNode {
    mapping(vec![
        ("app_store_id", leaf()),
        ("platform", leaf()),
        ("display_name", leaf()),
        ("self_service", boolean_leaf()),
        ("setup_experience", boolean_leaf()),
        ("categories", array(leaf())),
        ("labels_include_any", array(leaf())),
        ("labels_exclude_any", array(leaf())),
        ("labels_include_all", array(leaf())),
        ("icon", software_asset()),
        ("configuration", software_asset()),
        ("auto_update_enabled", boolean_leaf()),
        ("auto_update_window_start", leaf()),
        ("auto_update_window_end", leaf()),
    ])
}

fn fleet_maintained_app_item() -> SchemaNode {
    mapping(vec![
        ("slug", leaf()),
        ("version", leaf()),
        ("display_name", leaf()),
        ("self_service", boolean_leaf()),
        ("setup_experience", boolean_leaf()),
        ("categories", array(leaf())),
        ("labels_include_any", array(leaf())),
        ("labels_exclude_any", array(leaf())),
        ("labels_include_all", array(leaf())),
        ("pre_install_query", software_asset()),
        ("install_script", software_asset()),
        ("uninstall_script", software_asset()),
        ("post_install_script", software_asset()),
        ("icon", software_asset()),
    ])
}

fn software_schema() -> SchemaNode {
    mapping(vec![
        ("packages", array(software_package_item())),
        ("app_store_apps", array(app_store_app_item())),
        ("fleet_maintained_apps", array(fleet_maintained_app_item())),
    ])
}

fn controls_schema() -> SchemaNode {
    mapping(vec![
        ("scripts", array(controls_script_targeting())),
        ("windows_enabled_and_configured", boolean_leaf()),
        ("windows_entra_tenant_ids", array(leaf())),
        ("enable_turn_on_windows_mdm_manually", boolean_leaf()),
        ("windows_migration_enabled", boolean_leaf()),
        ("enable_disk_encryption", boolean_leaf()),
        ("enable_recovery_lock_password", boolean_leaf()),
        ("volume_purchasing_program", open_mapping()),
        ("windows_require_bitlocker_pin", boolean_leaf()),
        ("macos_updates", macos_updates()),
        ("ios_updates", macos_updates()),
        ("ipados_updates", macos_updates()),
        ("windows_updates", windows_updates()),
        ("macos_settings", os_settings()),
        ("apple_settings", os_settings()), // rename of macos_settings
        ("windows_settings", os_settings()),
        ("android_settings", android_settings_schema()),
        ("macos_setup", macos_setup()),
        ("setup_experience", macos_setup()), // rename of macos_setup
        ("macos_migration", macos_migration()),
    ])
}

fn policy_inline_strict() -> SchemaNode {
    mapping(vec![
        ("name", leaf()),
        ("description", leaf()),
        ("resolution", leaf()),
        ("query", leaf()),
        ("platform", leaf()),
        ("critical", boolean_leaf()),
        ("calendar_events_enabled", boolean_leaf()),
        ("conditional_access_enabled", boolean_leaf()),
        ("conditional_access_bypass_enabled", boolean_leaf()),
        ("software_title_id", leaf()),
        ("script_id", leaf()),
        ("labels_include_any", array(leaf())),
        ("labels_include_all", array(leaf())),
        ("labels_exclude_any", array(leaf())),
        ("run_script", mapping(vec![("path", leaf())])),
        (
            "install_software",
            mapping(vec![("package_path", leaf()), ("hash_sha256", leaf())]),
        ),
    ])
}

fn query_inline_strict() -> SchemaNode {
    mapping(vec![
        ("name", leaf()),
        ("description", leaf()),
        ("query", leaf()),
        ("platform", leaf()),
        ("interval", leaf()),
        ("logging", leaf()),
        ("min_osquery_version", leaf()),
        ("observer_can_run", boolean_leaf()),
        ("automations_enabled", boolean_leaf()),
        ("discard_data", boolean_leaf()),
        ("labels_include_any", array(leaf())),
        ("labels_include_all", array(leaf())),
        ("labels_exclude_any", array(leaf())),
    ])
}

fn label_inline_strict() -> SchemaNode {
    mapping(vec![
        ("name", leaf()),
        ("description", leaf()),
        ("platform", leaf()),
        ("label_membership_type", leaf()),
        ("query", leaf()),
        ("hosts", array(leaf())),
        ("host_ids", array(leaf())),
        // host_vitals criteria — supports nested and/or logic
        // json:"criteria" in Fleet Go source (HostVitalCriteria struct)
        ("criteria", host_vital_criteria()),
        // Some GitOps repos use "host_vitals" as an alias
        ("host_vitals", open_mapping()),
    ])
}

fn host_vital_criteria() -> SchemaNode {
    // HostVitalCriteria can be a single {vital, value, operator} or
    // nested via {and: [...]} / {or: [...]}. Using open_mapping to
    // support the recursive structure.
    open_mapping()
}

fn integrations_strict() -> SchemaNode {
    mapping(vec![
        ("conditional_access_enabled", boolean_leaf()),
        ("enable_conditional_access", boolean_leaf()),
        ("enable_conditional_access_bypass", boolean_leaf()),
        ("webhooks_and_tickets_enabled", boolean_leaf()),
        (
            "google_calendar",
            array(mapping(vec![
                ("api_key_json", leaf()),
                ("domain", leaf()),
                ("enable_calendar_events", boolean_leaf()),
                ("webhook_url", leaf()),
            ])),
        ),
        (
            "jira",
            array(mapping(vec![
                ("url", leaf()),
                ("username", leaf()),
                ("api_token", leaf()),
                ("project_key", leaf()),
                ("enable_failing_policies", boolean_leaf()),
                ("enable_software_vulnerabilities", boolean_leaf()),
            ])),
        ),
        (
            "zendesk",
            array(mapping(vec![
                ("url", leaf()),
                ("email", leaf()),
                ("api_token", leaf()),
                ("group_id", leaf()),
                ("enable_failing_policies", boolean_leaf()),
                ("enable_software_vulnerabilities", boolean_leaf()),
            ])),
        ),
    ])
}

fn webhook_settings_strict() -> SchemaNode {
    mapping(vec![
        (
            "activities_webhook",
            mapping(vec![
                ("enable_activities_webhook", boolean_leaf()),
                ("destination_url", leaf()),
            ]),
        ),
        (
            "failing_policies_webhook",
            mapping(vec![
                ("enable_failing_policies_webhook", boolean_leaf()),
                ("destination_url", leaf()),
                ("policy_ids", array(leaf())),
                ("host_batch_size", leaf()),
            ]),
        ),
        (
            "host_status_webhook",
            mapping(vec![
                ("enable_host_status_webhook", boolean_leaf()),
                ("destination_url", leaf()),
                ("days_count", leaf()),
                ("host_percentage", leaf()),
            ]),
        ),
        (
            "vulnerabilities_webhook",
            mapping(vec![
                ("enable_vulnerabilities_webhook", boolean_leaf()),
                ("destination_url", leaf()),
                ("host_batch_size", leaf()),
            ]),
        ),
        ("interval", leaf()),
    ])
}

fn org_settings_strict() -> SchemaNode {
    mapping(vec![
        (
            "features",
            mapping(vec![
                ("additional_queries", open_mapping()),
                ("enable_host_users", boolean_leaf()),
                ("enable_software_inventory", boolean_leaf()),
                ("detail_query_overrides", open_mapping()),
                ("osquery_detail", open_mapping()),
                ("osquery_policy", open_mapping()),
            ]),
        ),
        (
            "fleet_desktop",
            mapping(vec![
                ("transparency_url", leaf()),
                ("alternative_browser_host", leaf()),
            ]),
        ),
        (
            "host_expiry_settings",
            mapping(vec![
                ("host_expiry_enabled", boolean_leaf()),
                ("host_expiry_window", leaf()),
            ]),
        ),
        (
            "org_info",
            mapping(vec![
                ("org_name", leaf()),
                ("org_logo_url", leaf()),
                ("org_logo_url_light_background", leaf()),
                ("contact_url", leaf()),
            ]),
        ),
        ("secrets", array(mapping(vec![("secret", leaf())]))),
        (
            "server_settings",
            mapping(vec![
                ("ai_features_disabled", boolean_leaf()),
                ("deferred_save_host", boolean_leaf()),
                ("enable_analytics", boolean_leaf()),
                ("live_query_disabled", boolean_leaf()),
                ("live_reporting_disabled", boolean_leaf()),
                ("query_reports_disabled", boolean_leaf()),
                ("discard_reports_data", boolean_leaf()),
                ("query_report_cap", leaf()),
                ("report_cap", leaf()),
                ("scripts_disabled", boolean_leaf()),
                ("server_url", leaf()),
            ]),
        ),
        (
            "sso_settings",
            mapping(vec![
                ("enable_sso", boolean_leaf()),
                ("idp_name", leaf()),
                ("idp_image_url", leaf()),
                ("entity_id", leaf()),
                ("metadata", leaf()),
                ("metadata_url", leaf()),
                ("enable_jit_provisioning", boolean_leaf()),
                ("enable_jit_role_sync", boolean_leaf()),
                ("enable_sso_idp_login", boolean_leaf()),
                ("issuer_uri", leaf()),
                ("sso_server_url", leaf()),
            ]),
        ),
        ("integrations", integrations_strict()),
        ("certificate_authorities", open_mapping()),
        ("webhook_settings", webhook_settings_strict()),
        ("mdm", open_mapping()),
        (
            "smtp_settings",
            mapping(vec![
                ("authentication_method", leaf()),
                ("authentication_type", leaf()),
                ("domain", leaf()),
                ("enable_smtp", boolean_leaf()),
                ("enable_ssl_tls", boolean_leaf()),
                ("enable_start_tls", boolean_leaf()),
                ("password", leaf()),
                ("port", leaf()),
                ("sender_address", leaf()),
                ("server", leaf()),
                ("user_name", leaf()),
                ("verify_ssl_certs", boolean_leaf()),
            ]),
        ),
        (
            "vulnerability_settings",
            mapping(vec![
                ("databases_path", leaf()),
                ("periodicity", leaf()),
                ("cpe_database_url", leaf()),
                ("cpe_translations_url", leaf()),
                ("cve_feed_prefix_url", leaf()),
                ("disable_data_sync", boolean_leaf()),
                ("disable_win_os_vulnerabilities", boolean_leaf()),
                ("recent_vulnerability_max_age", leaf()),
            ]),
        ),
        (
            "activity_expiry_settings",
            mapping(vec![
                ("activity_expiry_enabled", boolean_leaf()),
                ("activity_expiry_window", leaf()),
            ]),
        ),
        ("yara_rules", array(open_mapping())),
        (
            "gitops",
            mapping(vec![
                ("gitops_mode_enabled", boolean_leaf()),
                ("repository_url", leaf()),
            ]),
        ),
    ])
}

fn team_settings_strict() -> SchemaNode {
    mapping(vec![
        (
            "features",
            mapping(vec![
                ("additional_queries", open_mapping()),
                ("enable_host_users", boolean_leaf()),
                ("enable_software_inventory", boolean_leaf()),
                ("detail_query_overrides", open_mapping()),
                ("osquery_detail", open_mapping()),
                ("osquery_policy", open_mapping()),
            ]),
        ),
        (
            "host_expiry_settings",
            mapping(vec![
                ("host_expiry_enabled", boolean_leaf()),
                ("host_expiry_window", leaf()),
            ]),
        ),
        ("secrets", array(mapping(vec![("secret", leaf())]))),
        ("integrations", integrations_strict()),
        ("webhook_settings", webhook_settings_strict()),
    ])
}

fn agent_options_inline() -> SchemaNode {
    mapping(vec![
        ("config", open_mapping()),
        ("overrides", open_mapping()),
        ("command_line_flags", open_mapping()),
        ("update_channels", open_mapping()),
        ("script_execution_timeout", open_mapping()),
        ("extensions", open_mapping()),
    ])
}

// ---------------------------------------------------------------------------
// Top-level schemas for each file type
// ---------------------------------------------------------------------------

/// Schema for `default.yml` files.
pub fn default_schema() -> SchemaNode {
    mapping(vec![
        (
            "labels",
            array_one_of(vec![label_inline_strict(), path_ref_schema()]),
        ),
        (
            "policies",
            array_one_of(vec![policy_inline_strict(), path_ref_schema()]),
        ),
        (
            "queries",
            array_one_of(vec![query_inline_strict(), path_ref_schema()]),
        ),
        (
            "reports",
            array_one_of(vec![query_inline_strict(), path_ref_schema()]),
        ),
        ("agent_options", agent_options_inline()),
        ("controls", controls_schema()),
        ("software", software_schema()),
        ("org_settings", org_settings_strict()),
        ("team_settings", team_settings_strict()),
        ("settings", team_settings_strict()),
    ])
}

/// Schema for `fleets/*.yml` (and legacy `teams/*.yml`) files. Same structure as default.
pub fn fleet_schema() -> SchemaNode {
    mapping(vec![
        ("name", leaf()),
        (
            "labels",
            array_one_of(vec![label_inline_strict(), path_ref_schema()]),
        ),
        (
            "policies",
            array_one_of(vec![policy_inline_strict(), path_ref_schema()]),
        ),
        (
            "queries",
            array_one_of(vec![query_inline_strict(), path_ref_schema()]),
        ),
        (
            "reports",
            array_one_of(vec![query_inline_strict(), path_ref_schema()]),
        ),
        ("agent_options", agent_options_inline()),
        ("controls", controls_schema()),
        ("software", software_schema()),
        ("team_settings", team_settings_strict()),
        ("settings", team_settings_strict()),
    ])
}

/// Schema for `lib/policies/*.yml` files (array of policies).
pub fn policy_schema() -> SchemaNode {
    array_one_of(vec![policy_inline_strict(), path_ref_schema()])
}

/// Schema for `lib/queries/*.yml` files (array of queries).
pub fn query_schema() -> SchemaNode {
    array_one_of(vec![query_inline_strict(), path_ref_schema()])
}

/// Schema for `lib/labels/*.yml` files (array of labels).
pub fn label_schema() -> SchemaNode {
    array_one_of(vec![label_inline_strict(), path_ref_schema()])
}

// ---------------------------------------------------------------------------
// Static instances
// ---------------------------------------------------------------------------

pub static DEFAULT_SCHEMA: Lazy<SchemaNode> = Lazy::new(default_schema);
pub static FLEET_SCHEMA: Lazy<SchemaNode> = Lazy::new(fleet_schema);
pub static POLICY_SCHEMA: Lazy<SchemaNode> = Lazy::new(policy_schema);
pub static QUERY_SCHEMA: Lazy<SchemaNode> = Lazy::new(query_schema);
pub static LABEL_SCHEMA: Lazy<SchemaNode> = Lazy::new(label_schema);

pub static KEY_REGISTRY: Lazy<KeyRegistry> = Lazy::new(|| {
    let mut reg = KeyRegistry::new();

    // Top-level keys (default.yml)
    reg.register("labels", "");
    reg.register("policies", "");
    reg.register("queries", "");
    reg.register("reports", "");
    reg.register("agent_options", "");
    reg.register("controls", "");
    reg.register("software", "");
    reg.register("org_settings", "");
    reg.register("team_settings", "");
    reg.register("settings", "");
    reg.register("name", "");

    // controls children
    reg.register("scripts", "controls");
    reg.register("windows_enabled_and_configured", "controls");
    reg.register("windows_entra_tenant_ids", "controls");
    reg.register("enable_turn_on_windows_mdm_manually", "controls");
    reg.register("windows_migration_enabled", "controls");
    reg.register("enable_disk_encryption", "controls");
    reg.register("windows_require_bitlocker_pin", "controls");
    reg.register("macos_updates", "controls");
    reg.register("ios_updates", "controls");
    reg.register("ipados_updates", "controls");
    reg.register("windows_updates", "controls");
    reg.register("macos_settings", "controls");
    reg.register("apple_settings", "controls"); // rename of macos_settings
    reg.register("windows_settings", "controls");
    reg.register("android_settings", "controls");
    reg.register("macos_setup", "controls");
    reg.register("setup_experience", "controls"); // rename of macos_setup
    reg.register("macos_migration", "controls");

    // controls.macos_settings / apple_settings / windows_settings children
    reg.register("custom_settings", "controls.macos_settings");
    reg.register("configuration_profiles", "controls.macos_settings");
    reg.register("custom_settings", "controls.apple_settings");
    reg.register("configuration_profiles", "controls.apple_settings");
    reg.register("custom_settings", "controls.windows_settings");
    reg.register("configuration_profiles", "controls.windows_settings");

    // controls.android_settings children
    reg.register("custom_settings", "controls.android_settings");
    reg.register("configuration_profiles", "controls.android_settings");
    reg.register("certificates", "controls.android_settings");

    // controls.android_settings.certificates[] fields
    reg.register("name", "controls.android_settings.certificates[]");
    reg.register(
        "certificate_authority_name",
        "controls.android_settings.certificates[]",
    );
    reg.register("subject_name", "controls.android_settings.certificates[]");

    // controls.macos_updates / ios_updates / ipados_updates children
    reg.register("deadline", "controls.macos_updates");
    reg.register("minimum_version", "controls.macos_updates");
    reg.register("update_new_hosts", "controls.macos_updates");
    reg.register("deadline", "controls.ios_updates");
    reg.register("minimum_version", "controls.ios_updates");
    reg.register("update_new_hosts", "controls.ios_updates");
    reg.register("deadline", "controls.ipados_updates");
    reg.register("minimum_version", "controls.ipados_updates");
    reg.register("update_new_hosts", "controls.ipados_updates");

    // controls.windows_updates children
    reg.register("deadline_days", "controls.windows_updates");
    reg.register("grace_period_days", "controls.windows_updates");

    // controls.macos_setup / setup_experience children
    for parent in &["controls.macos_setup", "controls.setup_experience"] {
        // Old names (json tags)
        reg.register("bootstrap_package", parent);
        reg.register("manual_agent_install", parent);
        reg.register("enable_end_user_authentication", parent);
        reg.register("lock_end_user_info", parent);
        reg.register("require_all_software", parent);
        reg.register("enable_release_device_manually", parent);
        reg.register("macos_setup_assistant", parent);
        reg.register("script", parent);
        // New names (renameto tags)
        reg.register("macos_bootstrap_package", parent);
        reg.register("macos_manual_agent_install", parent);
        reg.register("apple_enable_release_device_manually", parent);
        reg.register("apple_setup_assistant", parent);
        reg.register("macos_script", parent);
    }

    // controls.macos_migration children
    reg.register("enable", "controls.macos_migration");
    reg.register("mode", "controls.macos_migration");
    reg.register("webhook_url", "controls.macos_migration");

    // org_settings children
    reg.register("features", "org_settings");
    reg.register("fleet_desktop", "org_settings");
    reg.register("host_expiry_settings", "org_settings");
    reg.register("org_info", "org_settings");
    reg.register("secrets", "org_settings");
    reg.register("server_settings", "org_settings");
    reg.register("sso_settings", "org_settings");
    reg.register("integrations", "org_settings");
    reg.register("certificate_authorities", "org_settings");
    reg.register("gitops", "org_settings");

    // org_settings.features children
    reg.register("additional_queries", "org_settings.features");
    reg.register("enable_host_users", "org_settings.features");
    reg.register("enable_software_inventory", "org_settings.features");

    // org_settings.fleet_desktop children
    reg.register("transparency_url", "org_settings.fleet_desktop");
    reg.register("alternative_browser_host", "org_settings.fleet_desktop");

    // org_settings.host_expiry_settings children
    reg.register("host_expiry_enabled", "org_settings.host_expiry_settings");
    reg.register("host_expiry_window", "org_settings.host_expiry_settings");

    // org_settings.org_info children
    reg.register("org_name", "org_settings.org_info");
    reg.register("org_logo_url", "org_settings.org_info");
    reg.register("org_logo_url_light_background", "org_settings.org_info");
    reg.register("contact_url", "org_settings.org_info");

    // org_settings.server_settings children
    reg.register("ai_features_disabled", "org_settings.server_settings");
    reg.register("enable_analytics", "org_settings.server_settings");
    reg.register("live_query_disabled", "org_settings.server_settings");
    reg.register("live_reporting_disabled", "org_settings.server_settings");
    reg.register("query_reports_disabled", "org_settings.server_settings");
    reg.register("discard_reports_data", "org_settings.server_settings");
    reg.register("query_report_cap", "org_settings.server_settings");
    reg.register("report_cap", "org_settings.server_settings");
    reg.register("scripts_disabled", "org_settings.server_settings");
    reg.register("server_url", "org_settings.server_settings");
    reg.register("discard_reports_data", "org_settings.server_settings");

    // org_settings.sso_settings children
    reg.register("enable_sso", "org_settings.sso_settings");
    reg.register("idp_name", "org_settings.sso_settings");
    reg.register("idp_image_url", "org_settings.sso_settings");
    reg.register("entity_id", "org_settings.sso_settings");
    reg.register("metadata", "org_settings.sso_settings");
    reg.register("metadata_url", "org_settings.sso_settings");
    reg.register("enable_jit_provisioning", "org_settings.sso_settings");
    reg.register("enable_jit_role_sync", "org_settings.sso_settings");
    reg.register("enable_sso_idp_login", "org_settings.sso_settings");
    reg.register("issuer_uri", "org_settings.sso_settings");
    reg.register("sso_server_url", "org_settings.sso_settings");

    // org_settings.vulnerability_settings children
    reg.register("vulnerability_settings", "org_settings");
    reg.register("databases_path", "org_settings.vulnerability_settings");
    reg.register("periodicity", "org_settings.vulnerability_settings");
    reg.register("cpe_database_url", "org_settings.vulnerability_settings");
    reg.register(
        "cpe_translations_url",
        "org_settings.vulnerability_settings",
    );
    reg.register("cve_feed_prefix_url", "org_settings.vulnerability_settings");
    reg.register("disable_data_sync", "org_settings.vulnerability_settings");
    reg.register(
        "disable_win_os_vulnerabilities",
        "org_settings.vulnerability_settings",
    );
    reg.register(
        "recent_vulnerability_max_age",
        "org_settings.vulnerability_settings",
    );

    // org_settings.activity_expiry_settings children
    reg.register("activity_expiry_settings", "org_settings");
    reg.register(
        "activity_expiry_enabled",
        "org_settings.activity_expiry_settings",
    );
    reg.register(
        "activity_expiry_window",
        "org_settings.activity_expiry_settings",
    );

    // org_settings.webhook_settings children
    reg.register("webhook_settings", "org_settings");
    reg.register("activities_webhook", "org_settings.webhook_settings");
    reg.register("failing_policies_webhook", "org_settings.webhook_settings");
    reg.register("host_status_webhook", "org_settings.webhook_settings");
    reg.register("vulnerabilities_webhook", "org_settings.webhook_settings");
    reg.register("interval", "org_settings.webhook_settings");

    // org_settings.mdm
    reg.register("mdm", "org_settings");
    reg.register("smtp_settings", "org_settings");
    reg.register("yara_rules", "org_settings");

    // org_settings.gitops children
    reg.register("gitops_mode_enabled", "org_settings.gitops");
    reg.register("repository_url", "org_settings.gitops");

    // team_settings children
    reg.register("features", "team_settings");
    reg.register("host_expiry_settings", "team_settings");
    reg.register("secrets", "team_settings");
    reg.register("integrations", "team_settings");
    reg.register("webhook_settings", "team_settings");

    // settings children (alias for team_settings)
    reg.register("features", "settings");
    reg.register("host_expiry_settings", "settings");
    reg.register("secrets", "settings");
    reg.register("integrations", "settings");
    reg.register("webhook_settings", "settings");

    // reports fields (alias for queries)
    reg.register("name", "reports[]");
    reg.register("description", "reports[]");
    reg.register("query", "reports[]");
    reg.register("platform", "reports[]");
    reg.register("interval", "reports[]");
    reg.register("logging", "reports[]");
    reg.register("min_osquery_version", "reports[]");
    reg.register("observer_can_run", "reports[]");
    reg.register("automations_enabled", "reports[]");
    reg.register("discard_data", "reports[]");
    reg.register("labels_include_any", "reports[]");
    reg.register("labels_include_all", "reports[]");
    reg.register("labels_exclude_any", "reports[]");
    reg.register("path", "reports[]");
    reg.register("paths", "reports[]");

    // integrations children (under org_settings.integrations or team_settings.integrations)
    reg.register("conditional_access_enabled", "org_settings.integrations");
    reg.register("google_calendar", "org_settings.integrations");
    reg.register("jira", "org_settings.integrations");
    reg.register("zendesk", "org_settings.integrations");

    // Policy fields
    reg.register("name", "policies[]");
    reg.register("description", "policies[]");
    reg.register("resolution", "policies[]");
    reg.register("query", "policies[]");
    reg.register("platform", "policies[]");
    reg.register("critical", "policies[]");
    reg.register("calendar_events_enabled", "policies[]");
    reg.register("conditional_access_enabled", "policies[]");
    reg.register("conditional_access_bypass_enabled", "policies[]");
    reg.register("software_title_id", "policies[]");
    reg.register("script_id", "policies[]");
    reg.register("labels_include_any", "policies[]");
    reg.register("labels_include_all", "policies[]");
    reg.register("labels_exclude_any", "policies[]");
    reg.register("run_script", "policies[]");
    reg.register("install_software", "policies[]");
    reg.register("path", "policies[]");
    reg.register("paths", "policies[]");

    // Query fields
    reg.register("name", "queries[]");
    reg.register("description", "queries[]");
    reg.register("query", "queries[]");
    reg.register("platform", "queries[]");
    reg.register("interval", "queries[]");
    reg.register("logging", "queries[]");
    reg.register("min_osquery_version", "queries[]");
    reg.register("observer_can_run", "queries[]");
    reg.register("automations_enabled", "queries[]");
    reg.register("discard_data", "queries[]");
    reg.register("labels_include_any", "queries[]");
    reg.register("labels_include_all", "queries[]");
    reg.register("labels_exclude_any", "queries[]");
    reg.register("path", "queries[]");
    reg.register("paths", "queries[]");

    // Label fields
    reg.register("name", "labels[]");
    reg.register("description", "labels[]");
    reg.register("platform", "labels[]");
    reg.register("label_membership_type", "labels[]");
    reg.register("query", "labels[]");
    reg.register("hosts", "labels[]");
    reg.register("host_ids", "labels[]");
    reg.register("criteria", "labels[]");
    reg.register("host_vitals", "labels[]");
    reg.register("path", "labels[]");
    reg.register("paths", "labels[]");

    // Script targeting fields (used in controls.scripts[], custom_settings[])
    reg.register("path", "controls.scripts[]");
    reg.register("paths", "controls.scripts[]");
    reg.register("labels_include_all", "controls.scripts[]");
    reg.register("labels_include_any", "controls.scripts[]");
    reg.register("labels_exclude_any", "controls.scripts[]");

    // software children
    reg.register("packages", "software");
    reg.register("app_store_apps", "software");
    reg.register("fleet_maintained_apps", "software");

    // software.packages[] fields
    reg.register("path", "software.packages[]");
    reg.register("url", "software.packages[]");
    reg.register("hash_sha256", "software.packages[]");
    reg.register("display_name", "software.packages[]");
    reg.register("self_service", "software.packages[]");
    reg.register("setup_experience", "software.packages[]");
    reg.register("categories", "software.packages[]");
    reg.register("labels_include_any", "software.packages[]");
    reg.register("labels_exclude_any", "software.packages[]");
    reg.register("labels_include_all", "software.packages[]");
    reg.register("pre_install_query", "software.packages[]");
    reg.register("install_script", "software.packages[]");
    reg.register("uninstall_script", "software.packages[]");
    reg.register("post_install_script", "software.packages[]");
    reg.register("icon", "software.packages[]");

    // software.app_store_apps[] fields
    reg.register("app_store_id", "software.app_store_apps[]");
    reg.register("platform", "software.app_store_apps[]");
    reg.register("display_name", "software.app_store_apps[]");
    reg.register("self_service", "software.app_store_apps[]");
    reg.register("setup_experience", "software.app_store_apps[]");
    reg.register("categories", "software.app_store_apps[]");
    reg.register("labels_include_any", "software.app_store_apps[]");
    reg.register("labels_exclude_any", "software.app_store_apps[]");
    reg.register("labels_include_all", "software.app_store_apps[]");
    reg.register("icon", "software.app_store_apps[]");
    reg.register("configuration", "software.app_store_apps[]");
    reg.register("auto_update_enabled", "software.app_store_apps[]");
    reg.register("auto_update_window_start", "software.app_store_apps[]");
    reg.register("auto_update_window_end", "software.app_store_apps[]");

    // software.fleet_maintained_apps[] fields
    reg.register("slug", "software.fleet_maintained_apps[]");
    reg.register("version", "software.fleet_maintained_apps[]");
    reg.register("display_name", "software.fleet_maintained_apps[]");
    reg.register("self_service", "software.fleet_maintained_apps[]");
    reg.register("setup_experience", "software.fleet_maintained_apps[]");
    reg.register("categories", "software.fleet_maintained_apps[]");
    reg.register("labels_include_any", "software.fleet_maintained_apps[]");
    reg.register("labels_exclude_any", "software.fleet_maintained_apps[]");
    reg.register("labels_include_all", "software.fleet_maintained_apps[]");
    reg.register("pre_install_query", "software.fleet_maintained_apps[]");
    reg.register("install_script", "software.fleet_maintained_apps[]");
    reg.register("uninstall_script", "software.fleet_maintained_apps[]");
    reg.register("post_install_script", "software.fleet_maintained_apps[]");
    reg.register("icon", "software.fleet_maintained_apps[]");

    reg
});

// ---------------------------------------------------------------------------
// File-type detection
// ---------------------------------------------------------------------------

/// Determine which schema to use based on a file path.
pub fn schema_for_path(path: &std::path::Path) -> &'static SchemaNode {
    let path_str = path.to_string_lossy();

    // fleets/*.yml or legacy teams/*.yml
    if path_str.contains("fleets/")
        || path_str.contains("fleets\\")
        || path_str.contains("teams/")
        || path_str.contains("teams\\")
    {
        return &FLEET_SCHEMA;
    }

    // lib/policies/*.yml or just policies (including under platforms/)
    if path_str.contains("policies") {
        return &POLICY_SCHEMA;
    }

    // lib/queries/*.yml, lib/reports/*.yml, or just queries/reports (including under platforms/)
    if path_str.contains("queries") || path_str.contains("reports") {
        return &QUERY_SCHEMA;
    }

    // lib/labels/*.yml or just labels (including under platforms/)
    if path_str.contains("labels") {
        return &LABEL_SCHEMA;
    }

    // platforms/<name>/default.yml or platforms/<name>/*.yml
    if path_str.contains("platforms/") || path_str.contains("platforms\\") {
        return &DEFAULT_SCHEMA;
    }

    // Default: default.yml or any other top-level config
    &DEFAULT_SCHEMA
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_schema_for_default() {
        let schema = schema_for_path(Path::new("default.yml"));
        assert!(matches!(schema, SchemaNode::Mapping(_)));
        assert!(schema.get_child("policies").is_some());
        assert!(schema.get_child("controls").is_some());
        assert!(schema.get_child("org_settings").is_some());
    }

    #[test]
    fn test_schema_for_fleet() {
        let schema = schema_for_path(Path::new("fleets/engineering.yml"));
        assert!(matches!(schema, SchemaNode::Mapping(_)));
        assert!(schema.get_child("name").is_some());
        assert!(schema.get_child("policies").is_some());
    }

    #[test]
    fn test_schema_for_legacy_teams() {
        // Backward compatibility: teams/ still routes to FLEET_SCHEMA
        let schema = schema_for_path(Path::new("teams/engineering.yml"));
        assert!(matches!(schema, SchemaNode::Mapping(_)));
        assert!(schema.get_child("name").is_some());
        assert!(schema.get_child("policies").is_some());
    }

    #[test]
    fn test_schema_for_policies() {
        let schema = schema_for_path(Path::new("lib/policies/security.yml"));
        assert!(matches!(schema, SchemaNode::ArrayOneOf(_)));
    }

    #[test]
    fn test_schema_for_queries() {
        let schema = schema_for_path(Path::new("lib/queries/compliance.yml"));
        assert!(matches!(schema, SchemaNode::ArrayOneOf(_)));
    }

    #[test]
    fn test_schema_for_labels() {
        let schema = schema_for_path(Path::new("lib/labels/hosts.yml"));
        assert!(matches!(schema, SchemaNode::ArrayOneOf(_)));
    }

    #[test]
    fn test_schema_for_reports() {
        let schema = schema_for_path(Path::new("lib/reports/compliance.yml"));
        assert!(matches!(schema, SchemaNode::ArrayOneOf(_)));
    }

    #[test]
    fn test_key_registry_lookup() {
        let reg = &*KEY_REGISTRY;
        let paths = reg.lookup("scripts").unwrap();
        assert!(paths.contains(&"controls"));

        let paths = reg.lookup("custom_settings").unwrap();
        assert!(paths.contains(&"controls.macos_settings"));
    }

    #[test]
    fn test_key_registry_paths_field() {
        let reg = &*KEY_REGISTRY;
        let paths = reg.lookup("paths").unwrap();
        assert!(paths.contains(&"reports[]"));
        assert!(paths.contains(&"policies[]"));
        assert!(paths.contains(&"queries[]"));
        assert!(paths.contains(&"labels[]"));
        assert!(paths.contains(&"controls.scripts[]"));
    }

    #[test]
    fn test_schema_key_count_does_not_regress() {
        // This test ensures we don't accidentally remove schema keys.
        // If this fails after a refactor, verify no keys were lost.
        // Update the count when intentionally adding/removing keys.
        let reg = &*KEY_REGISTRY;
        let total_keys = reg.all_keys().len();
        assert!(
            total_keys >= 160,
            "KEY_REGISTRY has {total_keys} keys, expected >= 160. Did keys get removed?"
        );
    }

    #[test]
    fn test_fleet_gitops_keys_present() {
        // Core Fleet GitOps YAML keys that must always be in the schema.
        // Source: fleet/pkg/spec/gitops.go + fleet/server/fleet/*.go
        let reg = &*KEY_REGISTRY;
        let required_keys = [
            // Top-level
            "policies",
            "reports",
            "queries",
            "labels",
            "controls",
            "software",
            "agent_options",
            "org_settings",
            "settings",
            "team_settings",
            "name",
            // Policy fields
            "query",
            "description",
            "resolution",
            "platform",
            "critical",
            "calendar_events_enabled",
            "software_title_id",
            "script_id",
            // Report/query fields
            "interval",
            "logging",
            "observer_can_run",
            "automations_enabled",
            "discard_data",
            "min_osquery_version",
            // Label fields
            "label_membership_type",
            "hosts",
            // Controls
            "enable_disk_encryption",
            "macos_updates",
            "ios_updates",
            "ipados_updates",
            "windows_updates",
            "scripts",
            "macos_settings",
            "apple_settings",
            "windows_settings",
            "android_settings",
            "custom_settings",
            "configuration_profiles",
            // Software
            "packages",
            "fleet_maintained_apps",
            "app_store_apps",
            // org_settings sections
            "features",
            "fleet_desktop",
            "host_expiry_settings",
            "server_settings",
            "sso_settings",
            "smtp_settings",
            "webhook_settings",
            "vulnerability_settings",
            "activity_expiry_settings",
            // Deprecation targets
            "path",
            "paths",
        ];

        for key in &required_keys {
            assert!(
                reg.lookup(key).is_some(),
                "Required Fleet GitOps key '{key}' missing from KEY_REGISTRY"
            );
        }
    }

    #[test]
    fn test_controls_schema_structure() {
        let schema = default_schema();
        let controls = schema.get_child("controls").unwrap();
        assert!(controls.get_child("scripts").is_some());
        assert!(controls.get_child("macos_settings").is_some());

        let macos = controls.get_child("macos_settings").unwrap();
        assert!(macos.get_child("custom_settings").is_some());
        // scripts should NOT be valid under macos_settings
        assert!(macos.get_child("scripts").is_none());
    }
}
